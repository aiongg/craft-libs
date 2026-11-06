//! The story driver and the story-scoped file service.

use std::sync::Arc;

use craft_host::{AppInfo, Files, Host, NoPickFiles};
use egui::accesskit::Role;
use egui::{Context, Key, RawInput, vec2};
use egui_kittest::HarnessBuilder;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, dialog_id};
use crate::theme::{Base, install};

/// A dialog with one fixed id, as most library dialogs have.
struct Ask {
    id: DialogId,
    state: DialogState,
}

impl Ask {
    fn new() -> Self {
        Self { id: dialog_id("story-ask"), state: DialogState::new() }
    }
}

impl StackDialog for Ask {
    type Result = ();

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        let shown = Dialog::with_id(self.id, "Ask").stack(stack).footer(Footer::standard()).show(ctx, &mut self.state, &mut (), |ui, ()| {
            ui.label("Body");
        });
        match shown.outcome? {
            Outcome::Ok => Some(DialogResult::Ok(())),
            Outcome::Cancel => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

#[test]
fn drive_shows_the_dialog_for_one_instance_and_reports_cancel() {
    let mut harness =
        HarnessBuilder::default().with_size(vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, open: &mut [bool; 2]| {
                for (n, open) in open.iter_mut().enumerate() {
                    ui.push_id(n, |ui| drive(ui, "ask", open, (), Ask::new, |_| {}, |()| "OK".to_owned()));
                }
            },
            [true, true],
        );
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::Dialog).count(), 1, "the second instance yields the dialog's one id");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state()[0], "the answered instance clears its open arg");
    assert!(harness.query_by_label("Result: Cancel").is_some(), "Cancel is worded by the dialog's own Cancel label");
}

fn files_of(ctx: &Context) -> *const () {
    Host::try_of_ctx(ctx).map_or(std::ptr::null(), |host| std::ptr::from_ref(host.files()).cast())
}

#[test]
fn a_story_after_an_ensure_files_story_sees_the_galleries_files() {
    let ctx = Context::default();
    let gallery: Arc<dyn Files> = Arc::new(NoPickFiles::default());
    Host::builder(AppInfo { id: "gallery", name: "Gallery", version: "1" }).files(Arc::clone(&gallery)).install(&ctx);
    let fake: Arc<dyn Files> = Arc::new(NoPickFiles::default());
    ctx.run_ui(RawInput::default(), |ui| {
        {
            let _files = ensure_files(ui.ctx(), Arc::clone(&fake));
            assert_eq!(files_of(ui.ctx()), Arc::as_ptr(&fake).cast(), "the story draws with its own files");
            assert_eq!(Host::try_of(ui).map(|host| host.app().id), Some("gallery"), "and the gallery's other services");
        }
        assert_eq!(files_of(ui.ctx()), Arc::as_ptr(&gallery).cast(), "the next story sees the gallery's files");
    })
    .drop_without_applying_deltas();
}

// The state under a name survives passes with the same args and is made again when they change.
#[test]
fn reset_on_drops_the_shared_state_when_the_args_change() {
    let mut harness = HarnessBuilder::default().with_size(vec2(100.0, 50.0)).build_ui_state(
        |ui, (args, seen): &mut (u32, Vec<(bool, u32)>)| {
            let reset = reset_on(ui, "counter", args);
            let state = shared(ui, "counter", || *args * 100);
            let mut count = lock(&state);
            *count += 1;
            seen.push((reset, *count));
        },
        (1, Vec::new()),
    );
    harness.run_steps(2);
    harness.state_mut().0 = 2;
    harness.run_steps(2);
    let seen = &harness.state().1;
    let resets: Vec<usize> = seen.iter().enumerate().filter(|(_, (reset, _))| *reset).map(|(n, _)| n).collect();
    assert_eq!(resets.len(), 1, "one reset, at the change: {seen:?}");
    let at = resets[0];
    assert!(at > 0 && seen[..at].iter().all(|(_, n)| (101..200).contains(n)), "kept while the args stay: {seen:?}");
    assert_eq!(seen[at].1, 201, "made again from the new args: {seen:?}");
    assert!(seen[at + 1..].iter().all(|(_, n)| (202..300).contains(n)), "kept after the change: {seen:?}");
}
