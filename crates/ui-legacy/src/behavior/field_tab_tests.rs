//! [`FieldTab`]'s keys in a modal dialog.

use egui::{Event, Id, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::FieldTab;
use crate::controls::{Button, Checkbox, ComboBox, NumberField, Select, TextField};
use crate::edit::EditEvent;
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

/// A dialog with lists in a [`FieldTab`] scope: the Name field, a Blend select, a searchable Font
/// select, a Size combo box and an OK button, with every event the lists sent.
struct Lists {
    name: String,
    blend: usize,
    font: usize,
    size: String,
    blends: Vec<EditEvent<usize>>,
    fonts: Vec<EditEvent<usize>>,
    sizes: Vec<EditEvent<String>>,
}

const FONTS: [&str; 4] = ["Garamond", "Bodoni", "Baskerville", "Futura"];

fn lists() -> Harness<'static, Lists> {
    Harness::builder().with_size([480.0, 480.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, d: &mut Lists| {
            egui::Modal::new(Id::unique("dialog")).show(ui.ctx(), |ui| {
                FieldTab::new("dialog").show(ui, |ui| {
                    let edit = TextField::new("name", "Name", &d.name).show(ui);
                    if let Some(name) = edit.event.as_ref().and_then(|e| e.value()) {
                        d.name.clone_from(name);
                    }
                    let blend = ["Normal", "Multiply", "Screen"]
                        .into_iter()
                        .enumerate()
                        .fold(Select::new("blend", "Blend", &mut d.blend), |s, (i, m)| s.option(i, m));
                    d.blends.extend(blend.show(ui).event);
                    let font = Select::new("font", "Font", &mut d.font).rows(FONTS.len(), |i| i, |i| FONTS[i]).searchable("Search fonts", "Clear");
                    d.fonts.extend(font.show(ui).event);
                    let edit = ComboBox::new("size", "Size", &d.size).options(["10 pt", "12 pt", "14 pt"]).show(ui);
                    if let Some(size) = edit.event.as_ref().and_then(|e| e.value()) {
                        d.size.clone_from(size);
                    }
                    d.sizes.extend(edit.event);
                    ui.add(Button::new("OK").id_salt("ok"));
                });
            });
        },
        Lists { name: "Untitled".to_owned(), blend: 0, font: 0, size: "12 pt".to_owned(), blends: Vec::new(), fonts: Vec::new(), sizes: Vec::new() },
    )
}

fn press(harness: &mut Harness<'_, Lists>, modifiers: Modifiers, key: Key) -> Option<String> {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
    harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label())
}

fn expanded(harness: &Harness<'_, Lists>, label: &str) -> Option<bool> {
    harness.get_by_label(label).accesskit_node().data().is_expanded()
}

/// The kinds of `events` in order, with the value of each but a `Begin`.
fn kinds<T: Clone + std::fmt::Debug>(events: &[EditEvent<T>]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "Begin".to_owned(),
            EditEvent::Update { value, .. } => format!("Update({value:?})"),
            EditEvent::Commit { value, changed, .. } => format!("Commit({value:?}, {changed})"),
            EditEvent::Cancel { reason, .. } => format!("Cancel({reason:?})"),
            EditEvent::Set { value, .. } => format!("Set({value:?})"),
        })
        .collect()
}

#[test]
fn tab_in_an_open_list_inside_a_field_tab_scope_is_the_list_s() {
    let (none, ctrl, alt) = (Modifiers::NONE, Modifiers::CTRL, Modifiers::ALT);
    let mut harness = lists();
    harness.run();
    assert_eq!(press(&mut harness, none, Key::Tab).as_deref(), Some("Name"));
    assert_eq!(press(&mut harness, ctrl, Key::Tab).as_deref(), Some("Blend"));
    press(&mut harness, alt, Key::ArrowDown);
    assert_eq!(expanded(&harness, "Blend"), Some(true));
    press(&mut harness, none, Key::ArrowDown);
    assert_eq!(harness.state().blend, 1, "the open list's value follows the arrows");
    // Tab chooses the active option and moves the focus on in egui's order, as outside a scope.
    assert_eq!(press(&mut harness, none, Key::Tab).as_deref(), Some("Font"));
    assert_eq!((harness.state().blend, expanded(&harness, "Blend")), (1, Some(false)));
    assert_eq!(kinds(&harness.state().blends), ["Begin", "Update(1)", "Commit(1, true)"]);

    // Ctrl+Tab in the open list chooses too.
    press(&mut harness, alt, Key::ArrowDown);
    press(&mut harness, none, Key::ArrowDown);
    press(&mut harness, ctrl, Key::Tab);
    assert_eq!((harness.state().font, expanded(&harness, "Font")), (1, Some(false)));
    assert_eq!(kinds(&harness.state().fonts), ["Begin", "Update(1)", "Commit(1, true)"]);

    // From a searchable list's search field.
    harness.get_by_label("Font").focus();
    harness.run();
    press(&mut harness, alt, Key::ArrowDown);
    harness.event(Event::Text("ba".to_owned()));
    harness.run();
    assert_eq!(harness.state().font, 1, "the first match waits");
    press(&mut harness, none, Key::Tab);
    assert_eq!((harness.state().font, expanded(&harness, "Font")), (2, Some(false)), "Tab chooses the match");
    assert_eq!(kinds(&harness.state().fonts[3..]), ["Begin", "Commit(2, true)"]);

    // The combo box: Tab commits what the field shows and moves on in egui's order.
    harness.get_by_label("Size").focus();
    harness.run();
    press(&mut harness, alt, Key::ArrowDown);
    assert_eq!(expanded(&harness, "Size"), Some(true));
    press(&mut harness, none, Key::ArrowDown);
    assert_eq!(press(&mut harness, none, Key::Tab).as_deref(), Some("OK"));
    assert_eq!((harness.state().size.as_str(), expanded(&harness, "Size")), ("14 pt", Some(false)));
    assert!(matches!(harness.state().sizes.last(), Some(EditEvent::Commit { value, .. }) if value == "14 pt"), "{:?}", kinds(&harness.state().sizes));

    // With every list closed, Tab is the scope's again.
    assert_eq!(press(&mut harness, none, Key::Tab).as_deref(), Some("Name"));
    // A list a click closed after it was shown is closed for the next pass's scope.
    harness.get_by_label("Blend").click();
    harness.run();
    let at = harness.get_by(|n| n.role() == egui::accesskit::Role::ListBoxOption && n.label().as_deref() == Some("Screen")).rect().center();
    harness.hover_at(at);
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.input_mut().events.extend([button(true), button(false)]);
    harness.step();
    assert_eq!((harness.state().blend, expanded(&harness, "Blend")), (2, Some(false)));
    assert!(!crate::popup_list::any_open(&harness.ctx));
}
