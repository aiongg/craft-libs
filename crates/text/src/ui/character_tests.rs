//! The Character and Paragraph controls in a harness.

use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::character::*;

struct Character {
    values: CharacterValues,
    unit: (LengthUnit, UnitContext),
    edits: Vec<EditEvent<CharacterChange>>,
}

impl Character {
    /// The last change any edit carried.
    fn last(&self) -> Option<&CharacterChange> {
        self.edits.iter().rev().find_map(|edit| edit.value())
    }
}

fn points() -> (LengthUnit, UnitContext) {
    (LengthUnit::Points, UnitContext::default())
}

fn pixels(ppi: f64) -> (LengthUnit, UnitContext) {
    (LengthUnit::Pixels, UnitContext { pixels_per_inch: ppi, ..UnitContext::default() })
}

/// The app's side: each change goes into the values the next pass shows.
fn apply(values: &mut CharacterValues, change: &CharacterChange) {
    match change {
        CharacterChange::Size(v) => values.size = Some(*v),
        CharacterChange::Tracking(v) => values.tracking = Some(*v),
        CharacterChange::Kerning(k) => values.kerning = Some(*k),
        CharacterChange::Caps(c) => values.caps = Some(*c),
        _ => {}
    }
}

fn character(values: CharacterValues, unit: (LengthUnit, UnitContext)) -> Harness<'static, Character> {
    let state = Character { values, unit, edits: Vec::new() };
    Harness::<Character>::builder()
        .with_size([420.0, 480.0])
        .allow_missing_glyphs()
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, s: &mut Character| {
                let families = ["Hack".to_owned(), "Ubuntu".to_owned()];
                let r = CharacterControls::new("character", &s.values, &families, &families).unit(s.unit.0, s.unit.1).show(ui);
                for edit in r.edits {
                    if let Some(change) = edit.value() {
                        apply(&mut s.values, change);
                    }
                    s.edits.push(edit);
                }
            },
            state,
        )
}

fn type_into(h: &mut Harness<'_, Character>, role: Role, label: &str, text: &str) {
    h.get_by_role_and_label(role, label).click();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.event(Event::Text(text.into()));
    h.key_press(Key::Enter);
    h.run();
}

fn base() -> CharacterValues {
    CharacterValues { size: Some(12.0), tracking: Some(0.0), kerning: Some(Kerning::Metrics), caps: Some(Caps::Normal), ..CharacterValues::default() }
}

// Typing 24 in the size field returns `Size(24.0)`; in pixels at 144 ppi the field shows 48 and
// typing 48 returns `Size(24.0)`.
#[test]
fn size_field_converts_the_type_unit() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), points());
    type_into(&mut h, Role::SpinButton, &labels.size, "24");
    assert_eq!(h.state().last(), Some(&CharacterChange::Size(24.0)));

    let mut h = character(CharacterValues { size: Some(24.0), ..base() }, pixels(144.0));
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, &labels.size).accesskit_node().numeric_value(), Some(48.0));
    type_into(&mut h, Role::SpinButton, &labels.size, "36");
    assert_eq!(h.state().last(), Some(&CharacterChange::Size(18.0)));
}

// Pressing All Caps returns `Caps(AllCaps)`; pressing it again returns `Caps(Normal)`; Small
// Caps while All Caps is on returns `Caps(SmallCaps)`.
#[test]
fn caps_toggles_are_one_choice() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), points());
    h.get_by_role_and_label(Role::Button, &labels.all_caps).click();
    h.run();
    assert_eq!(h.state().last(), Some(&CharacterChange::Caps(Caps::AllCaps)));
    h.get_by_role_and_label(Role::Button, &labels.small_caps).click();
    h.run();
    assert_eq!(h.state().last(), Some(&CharacterChange::Caps(Caps::SmallCaps)));
    h.get_by_role_and_label(Role::Button, &labels.small_caps).click();
    h.run();
    assert_eq!(h.state().last(), Some(&CharacterChange::Caps(Caps::Normal)));
    assert_eq!(h.state().edits.len(), 3);
}

// Kerning accepts "optical" (any case) and "-40"; a mixed value shows an empty field.
#[test]
fn kerning_field_and_mixed_values() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), points());
    type_into(&mut h, Role::TextInput, &labels.kerning, "OpTiCal");
    assert_eq!(h.state().last(), Some(&CharacterChange::Kerning(Kerning::Optical)));
    type_into(&mut h, Role::TextInput, &labels.kerning, "-40");
    assert_eq!(h.state().last(), Some(&CharacterChange::Kerning(Kerning::Manual(-40.0))));

    let mixed = CharacterValues { kerning: None, tracking: None, ..base() };
    let h = character(mixed, points());
    assert_eq!(h.get_by_role_and_label(Role::TextInput, &labels.kerning).accesskit_node().value().unwrap_or_default(), "");
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, &labels.tracking).accesskit_node().numeric_value(), None);
}

struct Paragraph {
    values: ParagraphValues,
    point_text: bool,
    changes: Vec<ParagraphChange>,
}

// Each control reports an edit event: a typing run in a number field is a gesture that ends in a
// `Commit`, Esc in it a `Cancel` (the host reverts), a preset or toggle a `Set`.
#[test]
fn field_edits_are_gestures_and_toggles_are_sets() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), points());
    type_into(&mut h, Role::SpinButton, &labels.size, "24");
    let edits = &h.state().edits;
    assert!(matches!(edits.last(), Some(EditEvent::Commit { value: CharacterChange::Size(v), changed: true, .. }) if *v == 24.0), "{edits:?}");
    h.state_mut().edits.clear();

    h.get_by_role_and_label(Role::SpinButton, &labels.size).click();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.event(Event::Text("30".into()));
    h.key_press(Key::Escape);
    h.run();
    let edits = &h.state().edits;
    assert!(!edits.iter().any(|e| matches!(e, EditEvent::Commit { changed: true, .. })), "{edits:?}");

    h.state_mut().edits.clear();
    h.get_by_role_and_label(Role::Button, &labels.all_caps).click();
    h.run();
    assert!(matches!(h.state().edits[..], [EditEvent::Set { value: CharacterChange::Caps(Caps::AllCaps), .. }]));
}

// The colour swatch asks for a colour with the current one; the request is set for the pass of the
// click only.
#[test]
fn swatch_requests_a_colour_for_one_pass() {
    let labels = CharacterLabels::english();
    let requests = std::cell::RefCell::new(Vec::new());
    let values = CharacterValues { color: Some(egui::Color32::from_rgb(10, 20, 30)), ..base() };
    let mut h = Harness::<()>::builder()
        .with_size([420.0, 480.0])
        .allow_missing_glyphs()
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(|ui| {
            let families = ["Hack".to_owned()];
            let r = CharacterControls::new("character", &values, &families, &families).show(ui);
            requests.borrow_mut().push(r.pick_color);
        });
    h.run();
    assert!(requests.borrow().iter().all(Option::is_none));
    h.get_by_role_and_label(Role::ColorWell, &labels.text_color).click();
    h.run();
    let seen: Vec<_> = requests.borrow().iter().flatten().cloned().collect();
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].key, COLOR_KEY);
    assert_eq!(egui::Color32::from(seen[0].color), egui::Color32::from_rgb(10, 20, 30));
}

// With `point_text(true)` the four justify buttons are disabled; Left returns `Align(Left)`.
#[test]
fn justify_buttons_need_a_box() {
    let labels = ParagraphLabels::english();
    let state =
        Paragraph { values: ParagraphValues { align: Some(TextAlign::Center), ..ParagraphValues::default() }, point_text: true, changes: Vec::new() };
    let mut h = Harness::<Paragraph>::builder()
        .with_size([420.0, 320.0])
        .allow_missing_glyphs()
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, s: &mut Paragraph| {
                s.changes.extend(
                    ParagraphControls::new("paragraph", &s.values)
                        .point_text(s.point_text)
                        .show(ui)
                        .edits
                        .into_iter()
                        .filter_map(|e| e.value().copied()),
                );
            },
            state,
        );
    for name in [&labels.justify_left, &labels.justify_center, &labels.justify_right, &labels.justify_all] {
        assert!(h.get_by_role_and_label(Role::Button, name).accesskit_node().is_disabled(), "{name} is disabled on point text");
    }
    for name in [&labels.left, &labels.center, &labels.right] {
        assert!(!h.get_by_role_and_label(Role::Button, name).accesskit_node().is_disabled());
    }
    h.get_by_role_and_label(Role::Button, &labels.left).click();
    h.run();
    assert_eq!(h.state().changes, [ParagraphChange::Align(TextAlign::Left)]);
}

// The leading menu's Auto row returns `Leading(Auto)` and leaves 0 out of the field's range.
#[test]
fn leading_menu_has_an_auto_row() {
    let labels = CharacterLabels::english();
    let mut h = character(CharacterValues { leading: Some(Leading::Points(20.0)), ..base() }, points());
    h.get_by_role_and_label(Role::Button, &labels.leading).click();
    h.run();
    h.get_by_label(&labels.auto).click();
    h.run();
    assert_eq!(h.state().last(), Some(&CharacterChange::Leading(Leading::Auto)));
    assert_eq!(h.state().edits.len(), 1);
}
