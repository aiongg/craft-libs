//! The Character and Paragraph controls in a harness.

use craft_ui::localized::Labels as _;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::character::*;

struct Character {
    values: CharacterValues,
    unit: TypeUnit,
    changes: Vec<CharacterChange>,
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

fn character(values: CharacterValues, unit: TypeUnit) -> Harness<'static, Character> {
    let state = Character { values, unit, changes: Vec::new() };
    Harness::<Character>::builder()
        .with_size([420.0, 480.0])
        .allow_missing_glyphs()
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, s: &mut Character| {
                let families = ["Hack".to_owned(), "Ubuntu".to_owned()];
                let r = CharacterControls::new(&s.values, &families, &families).unit(s.unit).show(ui);
                for change in r.changes {
                    apply(&mut s.values, &change);
                    s.changes.push(change);
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
    let mut h = character(base(), TypeUnit::Points);
    type_into(&mut h, Role::SpinButton, &labels.size, "24");
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Size(24.0)));

    let mut h = character(CharacterValues { size: Some(24.0), ..base() }, TypeUnit::Pixels { ppi: 144.0 });
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, &labels.size).accesskit_node().numeric_value(), Some(48.0));
    type_into(&mut h, Role::SpinButton, &labels.size, "36");
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Size(18.0)));
}

// Pressing All Caps returns `Caps(AllCaps)`; pressing it again returns `Caps(Normal)`; Small
// Caps while All Caps is on returns `Caps(SmallCaps)`.
#[test]
fn caps_toggles_are_one_choice() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), TypeUnit::Points);
    h.get_by_role_and_label(Role::Button, &labels.all_caps).click();
    h.run();
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Caps(Caps::AllCaps)));
    h.get_by_role_and_label(Role::Button, &labels.small_caps).click();
    h.run();
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Caps(Caps::SmallCaps)));
    h.get_by_role_and_label(Role::Button, &labels.small_caps).click();
    h.run();
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Caps(Caps::Normal)));
    assert_eq!(h.state().changes.len(), 3);
}

// Kerning accepts "optical" (any case) and "-40"; a mixed value shows an empty field.
#[test]
fn kerning_field_and_mixed_values() {
    let labels = CharacterLabels::english();
    let mut h = character(base(), TypeUnit::Points);
    type_into(&mut h, Role::TextInput, &labels.kerning, "OpTiCal");
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Kerning(Kerning::Optical)));
    type_into(&mut h, Role::TextInput, &labels.kerning, "-40");
    assert_eq!(h.state().changes.last(), Some(&CharacterChange::Kerning(Kerning::Manual(-40.0))));

    let mixed = CharacterValues { kerning: None, tracking: None, ..base() };
    let h = character(mixed, TypeUnit::Points);
    assert_eq!(h.get_by_role_and_label(Role::TextInput, &labels.kerning).accesskit_node().value().unwrap_or_default(), "");
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, &labels.tracking).accesskit_node().numeric_value(), None);
}

struct Paragraph {
    values: ParagraphValues,
    point_text: bool,
    changes: Vec<ParagraphChange>,
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
                s.changes.extend(ParagraphControls::new(&s.values).point_text(s.point_text).show(ui).changes);
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
    let mut h = character(CharacterValues { leading: Some(Leading::Points(20.0)), ..base() }, TypeUnit::Points);
    h.get_by_role_and_label(Role::Button, &labels.leading).click();
    h.run();
    h.get_by_label(&labels.auto).click();
    h.run();
    assert_eq!(h.state().changes, [CharacterChange::Leading(Leading::Auto)]);
}
