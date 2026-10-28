//! [`FieldTab`]'s keys in a modal dialog.

use egui::{Event, Id, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::FieldTab;
use crate::controls::{Button, Checkbox, NumberField, TextField};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, install};

/// A dialog's fields and controls: Name, the Reset button, Width, the Snap check box, Height and a
/// disabled Depth, in a modal; in a [`FieldTab`] scope or not.
struct Dialog {
    scoped: bool,
    name: String,
    width: f64,
    height: f64,
    snap: bool,
}

fn dialog(scoped: bool) -> Harness<'static, Dialog> {
    let numbers = LocaleNumbers::from_tag("en");
    let contents = move |ui: &mut egui::Ui, d: &mut Dialog| {
        let edit = TextField::new("name", "Name", &d.name).show(ui);
        if let Some(name) = edit.event.as_ref().and_then(|e| e.value()) {
            d.name.clone_from(name);
        }
        ui.add(Button::new("Reset").id_salt("reset"));
        for (label, value) in [("Width", &mut d.width), ("Height", &mut d.height)] {
            if label == "Height" {
                let _ = Checkbox::bool("Snap", &mut d.snap).id_salt("snap").show(ui);
            }
            let edit = NumberField::new(label, label, *value, &numbers).show(ui);
            if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                *value = *v;
            }
        }
        let _ = NumberField::new("depth", "Depth", 1.0, &numbers).enabled(false).show(ui);
    };
    Harness::builder().with_size([480.0, 480.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, d: &mut Dialog| {
            egui::Modal::new(Id::unique("dialog")).show(ui.ctx(), |ui| {
                if d.scoped {
                    FieldTab::new("dialog").show(ui, |ui| contents(ui, d));
                } else {
                    contents(ui, d);
                }
            });
        },
        Dialog { scoped, name: "Untitled".to_owned(), width: 100.0, height: 50.0, snap: false },
    )
}

fn focused_label(harness: &Harness<'_, Dialog>) -> Option<String> {
    harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label())
}

fn chord(harness: &mut Harness<'_, Dialog>, modifiers: Modifiers, key: Key) -> Option<String> {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
    focused_label(harness)
}

fn type_text(harness: &mut Harness<'_, Dialog>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

#[test]
fn tab_in_a_field_tab_scope_visits_only_the_fields_and_ctrl_tab_and_the_arrows_reach_the_rest() {
    let mut harness = dialog(true);
    harness.run();
    let (none, shift, ctrl) = (Modifiers::NONE, Modifiers::SHIFT, Modifiers::CTRL);
    assert_eq!(chord(&mut harness, none, Key::Tab).as_deref(), Some("Name"), "from nothing focused, the first field");
    type_text(&mut harness, "Poster");
    assert_eq!(harness.state().name, "Poster", "the field's text was selected: typing replaced it");
    assert_eq!(chord(&mut harness, none, Key::Tab).as_deref(), Some("Width"), "past the button");
    type_text(&mut harness, "512");
    assert_eq!(chord(&mut harness, none, Key::Tab).as_deref(), Some("Height"), "past the check box");
    type_text(&mut harness, "300");
    assert_eq!((harness.state().width, harness.state().height), (512.0, 300.0));
    assert_eq!(chord(&mut harness, none, Key::Tab).as_deref(), Some("Name"), "past the disabled field, around to the first");
    assert_eq!(chord(&mut harness, shift, Key::Tab).as_deref(), Some("Height"), "Shift+Tab, around to the last");
    assert_eq!(chord(&mut harness, shift, Key::Tab).as_deref(), Some("Width"));

    // Ctrl+Tab goes through every control, as Tab does outside a scope.
    assert_eq!(chord(&mut harness, ctrl, Key::Tab).as_deref(), Some("Snap"));
    assert_eq!(chord(&mut harness, ctrl | shift, Key::Tab).as_deref(), Some("Width"));
    assert_eq!(chord(&mut harness, ctrl | shift, Key::Tab).as_deref(), Some("Reset"));
    // The arrows move from a control other than a field, and Tab goes on to a field from it.
    assert_eq!(chord(&mut harness, none, Key::ArrowDown).as_deref(), Some("Width"));
    assert_eq!(chord(&mut harness, ctrl, Key::Tab).as_deref(), Some("Snap"));
    assert_eq!(chord(&mut harness, none, Key::Space).as_deref(), Some("Snap"));
    assert!(harness.state().snap, "Space checks it");
    assert_eq!(chord(&mut harness, none, Key::ArrowUp).as_deref(), Some("Width"));
    assert_eq!(chord(&mut harness, none, Key::Tab).as_deref(), Some("Height"));

    let mut plain = dialog(false);
    plain.run();
    let order: Vec<String> = a11y::tab_order(&mut plain).into_iter().map(|s| s.label).collect();
    assert_eq!(order, ["Name", "Reset", "Width", "Snap", "Height"], "without a scope Tab visits every control");
}
