//! Tests of the marquee style cells (task K13).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::fields::ConstraintFields;
use crate::canvas::Constraint;
use crate::edit::EditEvent;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::theme::{Base, install};

struct Host {
    constraint: Constraint,
    enabled: bool,
    unit: Option<LengthUnit>,
    events: Vec<EditEvent<Constraint>>,
    picked: Option<LengthUnit>,
}

impl Host {
    fn new(constraint: Constraint) -> Self {
        Self { constraint, enabled: true, unit: None, events: Vec::new(), picked: None }
    }
}

fn bar(ui: &mut egui::Ui, host: &mut Host) {
    let context = UnitContext { pixels_per_inch: 300.0, ..UnitContext::default() };
    let mut fields = ConstraintFields::new("style", &host.constraint).enabled(host.enabled);
    if let Some(unit) = host.unit {
        fields = fields.size_units(unit, &LengthUnit::ALL, context, context);
    }
    let out = ui.horizontal(|ui| fields.show(ui)).inner;
    host.picked = host.picked.or(out.unit_picked);
    if let Some(event) = out.event {
        if let Some(&value) = event.value() {
            host.constraint = value;
        }
        host.events.push(event);
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h = Harness::builder().with_size([640.0, 240.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(bar, host);
    h.run_steps(2);
    h
}

fn type_into(h: &mut Harness<'_, Host>, label: &str, text: &str, key: egui::Key) {
    h.get_by_role_and_label(Role::SpinButton, label).click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    h.run();
    h.key_press(key);
    h.run_steps(3);
}

// Picking Fixed Ratio gives Ratio(1, 1), Fixed Size Size(64, 64), Normal Free, each one `Set`.
#[test]
fn constraint_fields_style_pick_resets_the_numbers() {
    let mut h = harness(Host::new(Constraint::Free));
    for (name, want) in [("Fixed Ratio", Constraint::Ratio(1.0, 1.0)), ("Fixed Size", Constraint::Size(64.0, 64.0)), ("Normal", Constraint::Free)] {
        h.get_by_role(Role::ComboBox).click();
        h.run_steps(2);
        h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(name)).click();
        h.run_steps(3);
        assert_eq!(h.state().constraint, want);
        assert!(matches!(h.state().events.last(), Some(EditEvent::Set { .. })), "{:?}", h.state().events);
    }
    assert_eq!(h.state().events.len(), 3);
}

// Typing 2.5 in Width of Ratio(1, 1) gives Ratio(2.5, 1), committed on Enter; out of range
// clamps to 0.001..=999.
#[test]
fn constraint_fields_typed_width_commits_on_enter() {
    let mut h = harness(Host::new(Constraint::Ratio(1.0, 1.0)));
    h.get_by_role_and_label(Role::SpinButton, "Width").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("2.5".into()));
    h.run_steps(2);
    assert!(!h.state().events.iter().any(|e| matches!(e, EditEvent::Commit { .. })), "{:?}", h.state().events);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Commit { value: Constraint::Ratio(2.5, 1.0), .. })), "{:?}", h.state().events);
    assert_eq!(h.state().constraint, Constraint::Ratio(2.5, 1.0));

    type_into(&mut h, "Height", "5000", egui::Key::Enter);
    assert_eq!(h.state().constraint, Constraint::Ratio(2.5, 999.0));
}

// The swap button turns Size(100, 50) into Size(50, 100).
#[test]
fn constraint_fields_swap_exchanges_width_and_height() {
    let mut h = harness(Host::new(Constraint::Size(100.0, 50.0)));
    h.get_by_role_and_label(Role::Button, "Swaps height and width").click();
    h.run_steps(3);
    assert_eq!(h.state().constraint, Constraint::Size(50.0, 100.0), "{:?}", h.state().events);
}

// Fixed Size in inches at 300 ppi shows 1 in for 300 px; typing "2in" gives 600 px; a picked unit
// comes back in `unit_picked`.
#[test]
fn constraint_fields_fixed_size_converts_units() {
    let mut host = Host::new(Constraint::Size(300.0, 150.0));
    host.unit = Some(LengthUnit::Inches);
    let mut h = harness(host);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Width").accesskit_node().data().value(), Some("1 in"));
    type_into(&mut h, "Width", "2in", egui::Key::Enter);
    assert_eq!(h.state().constraint, Constraint::Size(600.0, 150.0));

    let at = h.get_by_role_and_label(Role::SpinButton, "Height").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed, modifiers: egui::Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
    h.get_by_label("Millimeters").click();
    h.run_steps(3);
    assert_eq!(h.state().picked, Some(LengthUnit::Millimetres));
}

// Free greys Width, swap and Height; `enabled(false)` greys every cell and reports nothing.
#[test]
fn constraint_fields_grey_for_normal_and_when_disabled() {
    let h = harness(Host::new(Constraint::Free));
    assert!(h.get_by_role_and_label(Role::SpinButton, "Width").accesskit_node().data().is_disabled());
    assert!(h.get_by_role_and_label(Role::Button, "Swaps height and width").accesskit_node().data().is_disabled());
    assert!(!h.get_by_role(Role::ComboBox).accesskit_node().data().is_disabled());

    let mut host = Host::new(Constraint::Ratio(1.0, 1.0));
    host.enabled = false;
    let mut h = harness(host);
    assert!(h.get_by_role(Role::ComboBox).accesskit_node().data().is_disabled());
    h.get_by_role_and_label(Role::Button, "Swaps height and width").click();
    h.run_steps(3);
    assert!(h.state().events.is_empty());
}
