//! Who has the keyboard.

use egui::{Context, Id, RawInput, Sense, pos2, vec2};

use super::key_owner::{KeyOwner, OwnerGate, owner, register_canvas};

pub(crate) fn pass(ctx: &Context, f: impl FnMut(&mut egui::Ui)) {
    ctx.run_ui(RawInput::default(), f).drop_without_applying_deltas();
}

/// Two passes with a focusable widget `id` that has the keyboard; registered as the canvas when
/// `canvas`.
pub(crate) fn focus_widget(ctx: &Context, id: Id, canvas: bool) {
    for _ in 0..2 {
        pass(ctx, |ui| {
            let r = ui.interact(
                egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(50.0, 50.0)),
                id,
                Sense::click_and_drag().union(Sense::focusable_noninteractive()),
            );
            r.request_focus();
            if canvas {
                register_canvas(ui.ctx(), id);
            }
        });
    }
}

#[test]
fn the_registered_canvas_with_focus_is_the_app_and_another_widget_is_not() {
    let ctx = Context::default();
    assert_eq!(owner(&ctx, &OwnerGate::default()), KeyOwner::App);
    focus_widget(&ctx, Id::unique("other"), false);
    assert_eq!(owner(&ctx, &OwnerGate::default()), KeyOwner::Widget);

    let ctx = Context::default();
    focus_widget(&ctx, Id::unique("canvas"), true);
    assert_eq!(owner(&ctx, &OwnerGate::default()), KeyOwner::App, "the focused canvas counts as no focus");
}

#[test]
fn the_gate_and_a_text_field_take_the_keys_first() {
    let ctx = Context::default();
    let mut text = String::new();
    for _ in 0..2 {
        pass(&ctx, |ui| ui.text_edit_singleline(&mut text).request_focus());
    }
    assert_eq!(owner(&ctx, &OwnerGate::default()), KeyOwner::Text);
    assert_eq!(owner(&ctx, &OwnerGate { dialog: true, ..OwnerGate::default() }), KeyOwner::Dialog);
    assert_eq!(owner(&ctx, &OwnerGate { recording: true, dialog: true, ..OwnerGate::default() }), KeyOwner::Recorder);
    assert_eq!(owner(&ctx, &OwnerGate { blocked: true, recording: true, ..OwnerGate::default() }), KeyOwner::Blocked);
}
