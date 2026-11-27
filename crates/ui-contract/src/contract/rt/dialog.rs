//! Dialogs on craft-ui-rt: each open dialog is a scope on the modal layer, a layer child of the
//! node whose build shows it (the `DialogHost`'s caller). Dialogs shown later lie above earlier
//! ones; the modal layer blocks the layers below it in its window, and a modal-rank dialog blocks
//! every other window of the app ([`craft_ui_rt::LayerSpec::app_modal`]). Escape, when no focused
//! node takes it, dismisses the top dialog.
//!
//! `DialogHost::run` shows each dialog on its stack through [`show_dialog`], in stack order, keyed
//! by its `DialogId`; a dialog's body runs when it opens or when a node in it is marked. A kept dialog returns its
//! kept value with `Shown::fresh` false, so a result the body returns (a button's pick) is read once, in the build
//! its body ran.

use super::overlay::{LayerShow, close_reason, dismissed, show_layer};
use super::ui::Ui;
use crate::contract::Shown;
use crate::contract::overlay::Overlay;
use crate::runtime::overlay::CloseReason;

/// Shows the dialog keyed `key` with `body` as its content, placed and dismissed as `overlay`
/// says, as a layer child of the node `ui` builds. `Err` with the reason when ui-rt dismissed it
/// since the last pass: it shows nothing then, and its node goes. `Shown::fresh` is true when
/// `body` ran in this build.
pub fn show_dialog<R: Clone + 'static>(
    ui: &mut Ui<'_, '_>,
    key: u64,
    overlay: &Overlay,
    mut body: impl FnMut(&mut Ui<'_, '_>) -> R,
) -> Result<Shown<R>, CloseReason> {
    let owner = ui.cx.node();
    let show = LayerShow { sweep: ui.flow.sweep, ..LayerShow::of(overlay, owner, ui.flow.region, false, &ui.flow.tokens) };
    if let Some(reason) = dismissed(ui.cx, show.layer, key) {
        return Err(close_reason(reason));
    }
    let id = ui.flow.id.with(("craft-dialog", key));
    let mut fresh = false;
    let value = show_layer(ui.cx, id, key, show, false, &mut |ui| {
        fresh = true;
        body(ui)
    });
    Ok(Shown { value, fresh })
}
