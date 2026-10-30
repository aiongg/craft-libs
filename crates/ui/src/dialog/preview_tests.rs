//! The Preview check box, hold-to-compare and the dialog-local history in a harness.

use egui::{Event, Key, Modifiers, PointerButton, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::controls::{Button, TextField};
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

struct App {
    state: DialogState<i32>,
    amount: i32,
    modeless: bool,
    open: bool,
    steps: Vec<Step>,
    /// Cmd/Ctrl+Z was still unconsumed after the dialog: it would have reached the document.
    leaked: bool,
}

fn harness(modeless: bool) -> Harness<'static, App> {
    let app = App { state: DialogState::new(), amount: 0, modeless, open: true, steps: Vec::new(), leaked: false };
    HarnessBuilder::default().with_size(WINDOW).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = &ui.ctx().clone();
            if !app.open {
                return;
            }
            let mut dialog = Dialog::new("preview", "Brightness").footer(Footer::standard().preview("Preview")).history(true);
            if app.modeless {
                dialog = dialog.modeless();
            }
            let compare = std::cell::Cell::new(None);
            let shown = dialog.show(ctx, &mut app.state, &mut app.amount, |ui, amount| {
                if ui.add(Button::new("Bump")).clicked() {
                    *amount += 1;
                }
                compare.set(Some(ui.add(Button::new("Compare"))));
                let _ = TextField::new("name", "Name", "Untitled").show(ui);
            });
            if let Some(response) = compare.take() {
                app.state.preview_mut().hold_to_compare(&response);
            }
            app.leaked |= ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z));
            app.steps.extend(shown.history);
            app.open &= !shown.outcome.is_some_and(Outcome::closes);
        },
        app,
    )
}

fn bump(harness: &mut Harness<'_, App>) {
    harness.get_by_label("Bump").click();
    harness.run();
}

fn cmd_z(harness: &mut Harness<'_, App>, shift: bool) {
    let modifiers = if shift { Modifiers::COMMAND | Modifiers::SHIFT } else { Modifiers::COMMAND };
    harness.key_press_modifiers(modifiers, Key::Z);
    harness.run();
}

#[test]
fn the_footer_check_box_toggles_preview() {
    let mut harness = harness(false);
    harness.run();
    assert!(harness.state().state.preview().on, "on by default");
    harness.get_by_label("Preview").click();
    harness.run();
    assert!(!harness.state().state.preview().on);
    assert!(!harness.state().state.preview().shows_result());
    harness.get_by_label("Preview").click();
    harness.run();
    assert!(harness.state().state.preview().shows_result());
}

#[test]
fn compare_is_true_only_while_the_pointer_or_space_is_held() {
    let mut harness = harness(false);
    harness.run();
    let at = harness.get_by_label("Compare").rect().center();
    harness.hover_at(at);
    harness.run();
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(button(true));
    harness.run();
    assert!(harness.state().state.preview().compare, "pointer held");
    assert!(!harness.state().state.preview().shows_result());
    harness.event(button(false));
    harness.run();
    assert!(!harness.state().state.preview().compare);

    harness.get_by_label("Compare").focus();
    harness.run();
    harness.key_down(Key::Space);
    harness.run();
    assert!(harness.state().state.preview().compare, "space held on the focused target");
    harness.key_up(Key::Space);
    harness.run();
    assert!(!harness.state().state.preview().compare);
}

#[test]
fn command_z_steps_the_fields_and_never_reaches_the_document() {
    let mut harness = harness(false);
    harness.run();
    bump(&mut harness);
    bump(&mut harness);
    assert_eq!(harness.state().amount, 2);
    cmd_z(&mut harness, false);
    assert_eq!((harness.state().amount, &harness.state().steps[..]), (1, &[Step::Undo][..]));
    cmd_z(&mut harness, false);
    assert_eq!(harness.state().amount, 0);
    cmd_z(&mut harness, true);
    assert_eq!((harness.state().amount, harness.state().steps.last()), (1, Some(&Step::Redo)));
    assert!(!harness.state().leaked, "the dialog consumed the keys");
}

#[test]
fn a_focused_text_field_keeps_command_z() {
    let mut harness = harness(false);
    harness.run();
    bump(&mut harness);
    harness.get_by_label("Name").focus();
    harness.run();
    cmd_z(&mut harness, false);
    assert_eq!(harness.state().amount, 1, "the field's typing undo, not the dialog's step");
    assert!(harness.state().steps.is_empty());
}

#[test]
fn a_modeless_dialog_steps_with_the_focus_inside() {
    let mut harness = harness(true);
    harness.run();
    bump(&mut harness);
    harness.get_by_label("Bump").focus();
    harness.run();
    cmd_z(&mut harness, false);
    assert_eq!(harness.state().amount, 0);
}

#[test]
fn closing_drops_the_history_and_the_preview_flags() {
    let mut harness = harness(false);
    harness.run();
    bump(&mut harness);
    harness.state_mut().state.preview_mut().on = false;
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().state.history().is_none());
    assert!(harness.state().state.preview().on);
}
