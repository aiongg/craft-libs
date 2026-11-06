//! Who has the keyboard.

use egui::{Context, Id, LayerId, RawInput, Sense, pos2, vec2};

use super::key_owner::{KeyOwner, OwnerGate, canvas_keys_free_for, install, keys_free, owner, register_canvas};

pub(crate) fn pass(ctx: &Context, f: impl FnMut(&mut egui::Ui)) {
    ctx.run_ui(RawInput::default(), f).drop_without_applying_deltas();
}

/// Two passes with a focusable widget `id` that has the keyboard; registered as the canvas when
/// `canvas`.
pub(crate) fn focus_widget(ctx: &Context, id: Id, canvas: bool) {
    focus_widget_claiming(ctx, id, canvas, &[]);
}

/// [`focus_widget`] whose widget claims `keys` on every pass.
pub(crate) fn focus_widget_claiming(ctx: &Context, id: Id, canvas: bool, keys: &[crate::behavior::ClaimedKey]) {
    for _ in 0..2 {
        pass(ctx, |ui| {
            let r = ui.interact(
                egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(50.0, 50.0)),
                id,
                Sense::click_and_drag().union(Sense::focusable_noninteractive()),
            );
            r.request_focus();
            crate::behavior::KeyClaim::claim(ui.ctx(), id, keys);
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

#[test]
fn layer_keys_are_free_over_a_focused_control_and_not_under_a_popover_or_a_text_field() {
    let layer = LayerId::background();
    let ctx = Context::default();
    assert!(keys_free(&ctx, layer));
    focus_widget(&ctx, Id::unique("button"), false);
    assert!(keys_free(&ctx, layer), "a focused control that is not text leaves the keys free");
    crate::popup_list::record_open_as(&ctx, Id::unique("popover"), true, true);
    assert!(!keys_free(&ctx, layer), "an open popover has the keys");

    let ctx = Context::default();
    let mut text = String::new();
    for _ in 0..2 {
        pass(&ctx, |ui| ui.text_edit_singleline(&mut text).request_focus());
    }
    assert!(!keys_free(&ctx, layer), "a text field has the keys");
}

// egui takes the focus off a text field at the start of the pass in which Esc is pressed; the
// field has read the key, so that pass it still owns the keyboard, and the next it does not.
#[test]
fn a_text_field_owns_the_keys_in_the_pass_egui_drops_its_focus_on_escape() {
    let ctx = Context::default();
    install(&ctx);
    let mut text = String::new();
    for _ in 0..2 {
        pass(&ctx, |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
            owner(ui.ctx(), &OwnerGate::default());
        });
    }
    let mut owners = Vec::new();
    let escape = || RawInput {
        events: vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }],
        ..RawInput::default()
    };
    ctx.run_ui(escape(), |ui| {
        ui.text_edit_singleline(&mut text);
        owners.push(owner(ui.ctx(), &OwnerGate::default()));
        assert!(!keys_free(ui.ctx(), LayerId::background()), "keys_free agrees with owner in the Escape pass");
    })
    .drop_without_applying_deltas();
    pass(&ctx, |ui| {
        ui.text_edit_singleline(&mut text);
        owners.push(owner(ui.ctx(), &OwnerGate::default()));
    });
    assert_eq!(owners, [KeyOwner::Text, KeyOwner::App]);
}

#[test]
fn a_chord_the_focused_control_claimed_is_not_free_for_the_canvas() {
    use crate::behavior::ClaimedKey;
    use egui::{Key, KeyboardShortcut, Modifiers};
    let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
    let cut = KeyboardShortcut::new(Modifiers::SHIFT, Key::Delete);
    let ctx = Context::default();
    focus_widget_claiming(&ctx, Id::unique("curve"), false, &[ClaimedKey::exact(Modifiers::NONE, Key::Delete)]);
    assert!(!canvas_keys_free_for(&ctx, &delete));
    assert!(canvas_keys_free_for(&ctx, &cut), "an unclaimed chord stays free");
    let ctx = Context::default();
    assert!(canvas_keys_free_for(&ctx, &delete), "nothing focused");
}

fn key(key: egui::Key) -> RawInput {
    RawInput {
        events: vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }],
        ..RawInput::default()
    }
}

/// A context with a focused text field after two passes, nobody asking [`owner`].
fn focused_field(text: &mut String) -> Context {
    let ctx = Context::default();
    install(&ctx);
    for _ in 0..2 {
        pass(&ctx, |ui| {
            ui.text_edit_singleline(text).request_focus();
        });
    }
    ctx
}

#[test]
fn a_tab_between_passes_after_escape_in_a_field_toggles_the_chrome() {
    let mut text = String::new();
    let ctx = focused_field(&mut text);
    ctx.run_ui(key(egui::Key::Escape), |ui| {
        ui.text_edit_singleline(&mut text);
    })
    .drop_without_applying_deltas();
    let mut raw = key(egui::Key::Tab);
    assert!(crate::shell::take_tab_toggle(&ctx, &mut raw), "the field gave up the keyboard with the Escape");
    assert!(raw.events.is_empty());
}

#[test]
fn a_reader_after_the_field_in_the_tab_pass_still_sees_the_field_as_owner() {
    let mut text = String::new();
    let ctx = focused_field(&mut text);
    let mut owners = Vec::new();
    ctx.run_ui(key(egui::Key::Tab), |ui| {
        owners.push(owner(ui.ctx(), &OwnerGate::default()));
        ui.text_edit_singleline(&mut text);
        owners.push(owner(ui.ctx(), &OwnerGate::default()));
    })
    .drop_without_applying_deltas();
    assert_eq!(owners, [KeyOwner::Text, KeyOwner::Text]);
}

#[test]
fn escape_after_idle_passes_nobody_read_still_gives_the_field_the_keys() {
    let mut text = String::new();
    let ctx = focused_field(&mut text);
    let mut owners = Vec::new();
    ctx.run_ui(key(egui::Key::Escape), |ui| {
        ui.text_edit_singleline(&mut text);
        owners.push(owner(ui.ctx(), &OwnerGate::default()));
    })
    .drop_without_applying_deltas();
    assert_eq!(owners, [KeyOwner::Text]);
}
