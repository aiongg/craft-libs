//! What stories of panels and dialogs share: state kept in egui memory, the dialog driver and a
//! fake file service scoped to the story.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_host::{AppInfo, Files, Host, HostScope};
use egui::{Context, Id, Ui};

use super::{StoryLabels, dialog_closed, dialog_shown};
use crate::dialog::{DialogHost, DialogLabels, DialogResult, StackDialog};
use crate::localized::Labels as _;

/// State of one story instance, kept in egui memory under `name` in `ui`'s scope and made by
/// `init` the first time. Two instances of a story (a component tab and a variants view) each
/// get their own. `init` runs outside egui's data lock, so it may read the theme or load textures.
pub fn shared<T: Send + 'static>(ui: &Ui, name: &str, init: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let key = ui.make_persistent_id(("craft_ui::story::shared", name));
    if let Some(kept) = ui.data(|d| d.get_temp::<Arc<Mutex<T>>>(key)) {
        return kept;
    }
    let made = Arc::new(Mutex::new(init()));
    ui.data_mut(|d| d.insert_temp(key, made.clone()));
    made
}

/// Locks [`shared`] state; a story that panicked while holding it leaves the state as it was.
pub fn lock<T>(state: &Mutex<T>) -> MutexGuard<'_, T> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether this story instance may draw the dialog `name` in this pass. A dialog has one fixed
/// id, so two instances of a story drawn in the same pass (a component tab and a variants view)
/// must not both show it: only the first to ask in a pass does. [`drive`] claims by itself.
pub fn claim(ctx: &Context, name: &str, instance: Id) -> bool {
    let key = Id::unique(("craft_ui::story::claim", name));
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let (claimed_pass, owner) = d.get_temp::<(u64, Id)>(key).unwrap_or((u64::MAX, instance));
        if claimed_pass == pass && owner != instance {
            return false;
        }
        d.insert_temp(key, (pass, instance));
        true
    })
}

/// Shows the dialog `D` on a [`DialogHost`] kept for this story instance under `name`.
///
/// - The dialog is open while [`dialog_shown`] says so and this instance holds the
///   [`claim`] on `D`; otherwise it is cancelled.
/// - `make` builds it when it opens, and again when `signature` (the args that build it) changes.
/// - `tune` runs on the open dialog every pass before it shows: the story answers the dialog's
///   requests there (a colour the dialog asked the app to pick).
/// - Once it closes, the answer stays under the tile: `describe` words an OK result, Cancel shows
///   the dialog's own Cancel label, and `open` is cleared ([`dialog_closed`]).
pub fn drive<D: StackDialog, S: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    name: &str,
    open: &mut bool,
    signature: S,
    make: impl FnOnce() -> D,
    tune: impl FnOnce(&mut D),
    describe: impl FnOnce(D::Result) -> String,
) {
    let ctx = ui.ctx().clone();
    let key = ui.make_persistent_id(("craft_ui::story::drive", name));
    let (result_key, signature_key) = (key.with("result"), key.with("signature"));
    let host: Arc<Mutex<DialogHost>> = ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, Arc::<Mutex<DialogHost>>::default).clone());
    let mut host = lock(&host);
    if dialog_shown(ui, *open) && claim(&ctx, std::any::type_name::<D>(), ui.scope_id()) {
        let previous: Option<S> = ui.data(|d| d.get_temp::<Option<S>>(signature_key)).flatten();
        if previous.is_some_and(|previous| previous != signature) {
            host.cancel::<D>(&ctx);
            host.take::<D>();
        }
        ui.data_mut(|d| d.insert_temp(signature_key, Some(signature)));
        if !host.is_open::<D>() {
            host.open(make());
            ui.data_mut(|d| d.remove_temp::<String>(result_key));
        }
        if let Some(dialog) = host.get_mut::<D>() {
            tune(dialog);
        }
        host.show(&ctx);
        if let Some(result) = host.take::<D>() {
            let text = match result {
                DialogResult::Ok(value) => describe(value),
                DialogResult::Cancel => DialogLabels::of(ui).cancel,
            };
            ui.data_mut(|d| d.insert_temp(result_key, text));
            dialog_closed(ui, open);
        }
    } else {
        host.cancel::<D>(&ctx);
        host.take::<D>();
        ui.data_mut(|d| d.insert_temp(signature_key, None::<S>));
    }
    if let Some(text) = ui.data(|d| d.get_temp::<String>(result_key)) {
        ui.label(craft_i18n::fmt(&StoryLabels::of(ui).result, &[("result", &text)]));
    }
}

/// Gives the story `files` as the Host's file service until the returned scope drops; hold it
/// until the story has drawn. The installed Host keeps its other services, and the next story
/// sees it unchanged. Without a Host, one with the defaults is installed first and stays, so
/// the story's settings outlive the pass.
///
/// Keep `files` in egui memory and pass the same one every pass: a picker's answer comes in a
/// later pass.
pub fn ensure_files(ctx: &Context, files: Arc<dyn Files>) -> HostScope {
    let host = Host::try_of_ctx(ctx).unwrap_or_else(|| {
        let host = Host::builder(AppInfo { id: "craft-stories", name: "Craft Stories", version: "0" }).build();
        host.clone().install(ctx);
        Arc::new(host)
    });
    host.with_files(files).install_scoped(ctx)
}
