//! The Paragraph dialog.

use std::cell::Cell;

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::units::LengthUnit;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::paragraph_dialog::{sample_plan, *};

struct App {
    host: DialogHost,
    answers: Vec<ParagraphAnswer>,
}

fn start() -> ParagraphFormat {
    ParagraphFormat {
        align: Some(ParagraphAlign::Left),
        before_text: Some(0.0),
        after_text: None,
        special: Some(SpecialIndent::None),
        by: Some(0.0),
        space_before: Some(0.0),
        space_after: Some(0.0),
        line_spacing: Some(LineSpacing::Multiple(1.5)),
        ..ParagraphFormat::default()
    }
}

fn open(format: ParagraphFormat) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(ParagraphDialog::new(format).unit(LengthUnit::Inches));
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 1100.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<ParagraphDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        );
    harness.run();
    harness
}

fn type_into(harness: &mut Harness<'_, App>, field: &str, text: &str) {
    harness.get_by_role_and_label(Role::SpinButton, field).focus();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

fn choose(harness: &mut Harness<'_, App>, list: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, list).click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}

fn format(harness: &Harness<'_, App>) -> ParagraphFormat {
    harness.state().host.get::<ParagraphDialog>().expect("open").format().clone()
}

// Before text 0.5 in, Hanging by 0.25 in, Exactly 18 pt returns 36, Hanging, 18 and LineSpacing::Exactly(18.0).
#[test]
fn ok_returns_the_format_in_points() {
    let mut harness = open(start());
    type_into(&mut harness, "Before text", "0.5");
    choose(&mut harness, "Special", "Hanging");
    type_into(&mut harness, "By", "0.25");
    choose(&mut harness, "Line spacing", "Exactly");
    harness.get_by_label("OK").click();
    harness.run();
    let answer = harness.state().answers.last().cloned().expect("OK answers");
    assert!(!answer.open_tabs);
    let f = answer.format;
    assert_eq!(
        (f.before_text, f.special, f.by, f.line_spacing),
        (Some(36.0), Some(SpecialIndent::Hanging), Some(18.0), Some(LineSpacing::Exactly(18.0)))
    );
}

// Choosing Exactly with At 1.5 sets At to 18; choosing Double sets Multiple(2.0).
#[test]
fn line_spacing_presets_fix_their_at_value() {
    let mut harness = open(start());
    choose(&mut harness, "Line spacing", "Exactly");
    assert_eq!(format(&harness).line_spacing, Some(LineSpacing::Exactly(18.0)));
    choose(&mut harness, "Line spacing", "Double");
    assert_eq!(format(&harness).line_spacing, Some(LineSpacing::Multiple(2.0)));
    choose(&mut harness, "Line spacing", "Multiple");
    assert_eq!(format(&harness).line_spacing, Some(LineSpacing::Multiple(2.0)), "a multiple stays when Multiple is chosen");
}

// A value out of range shows the field's message and OK waits for a fix; "abc" in Spacing before is not
// taken. (The number field marks rejected text itself and keeps the value from before.)
#[test]
fn a_value_out_of_range_refuses_ok_with_its_message() {
    let mut harness = open(ParagraphFormat { space_before: Some(2000.0), ..start() });
    harness.get_by_label_contains("Spacing before must be from");
    harness.get_by_label("OK").click();
    harness.run();
    assert!(harness.state().host.get::<ParagraphDialog>().is_some() && harness.state().answers.is_empty(), "OK refused");

    type_into(&mut harness, "Spacing before", "12");
    assert_eq!(format(&harness).space_before, Some(12.0));
    assert!(harness.query_by_label_contains("Spacing before must be from").is_none());
    type_into(&mut harness, "Spacing before", "abc");
    assert_eq!(format(&harness).space_before, Some(12.0), "text that is not a number is not taken");
    harness.get_by_label("OK").click();
    harness.run();
    assert!(harness.state().answers.is_empty(), "OK waits for the rejected text");
    type_into(&mut harness, "Spacing before", "13");
    harness.get_by_label("OK").click();
    harness.run();
    assert_eq!(harness.state().answers.last().and_then(|a| a.format.space_before), Some(13.0));
}

// Tabs… returns Ok with open_tabs true and the format as edited.
#[test]
fn tabs_button_answers_with_open_tabs() {
    let mut harness = open(start());
    type_into(&mut harness, "Spacing after", "6");
    harness.get_by_label("Tabs…").click();
    harness.run();
    let answer = harness.state().answers.last().cloned().expect("Tabs… answers");
    assert!(answer.open_tabs);
    assert_eq!(answer.format, ParagraphFormat { space_after: Some(6.0), ..start() });
    assert!(harness.state().host.get::<ParagraphDialog>().is_none());
}

// The preview's lines follow the indents: a hanging indent pulls the first line out, and a
// paragraph's spacing moves the lines under it.
#[test]
fn the_preview_lays_out_the_values() {
    let format = ParagraphFormat {
        before_text: Some(36.0),
        special: Some(SpecialIndent::Hanging),
        by: Some(18.0),
        space_before: Some(10.0),
        line_spacing: Some(LineSpacing::Exactly(20.0)),
        ..ParagraphFormat::default()
    };
    let plan = sample_plan(&format, 400.0, 12.0);
    let subject: Vec<_> = plan.rows.iter().filter(|r| r.subject).collect();
    assert_eq!((subject[0].rect.left(), subject[1].rect.left()), (36.0, 54.0));
    assert_eq!(subject[1].rect.height(), 20.0);
    assert_eq!(subject[0].rect.top(), 3.0 * 12.0 + 10.0);
    assert!(!subject[2].full && subject[0].full);
}

// Text that is not a number (a rejected field) disables OK until it is fixed, also after the focus leaves: "abc" in Before text leaves OK disabled and a click answers nothing; a number enables it.
#[test]
fn rejected_text_refuses_ok() {
    let mut harness = open(start());
    type_into(&mut harness, "Before text", "abc");
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled(), "OK waits for the rejected text");
    harness.get_by_label("OK").click();
    harness.run();
    assert!(harness.state().answers.is_empty());

    type_into(&mut harness, "Before text", "0.5");
    assert!(!harness.get_by_label("OK").accesskit_node().is_disabled());
}

// A field's Cancel restores the value its gesture began from: Begin and Update to 1 in then Cancel leave Before text at 0.
#[test]
fn cancelling_a_field_gesture_restores_the_start() {
    let mut dialog = ParagraphDialog::new(start());
    let valid = Cell::new(true);
    let id = GestureId::next();
    let to = |v: f32| LengthEdit { event: Some(EditEvent::update(id, Some(v), egui::Modifiers::NONE)), rejected: false };
    dialog.edit(Row::BeforeText, LengthEdit { event: Some(EditEvent::begin(id)), rejected: false }, &valid);
    dialog.edit(Row::BeforeText, to(72.0), &valid);
    assert_eq!(dialog.format().before_text, Some(72.0));
    dialog.edit(Row::BeforeText, LengthEdit { event: Some(EditEvent::cancel(id, CancelReason::Escape)), rejected: false }, &valid);
    assert_eq!(dialog.format().before_text, Some(0.0));
}

// Spacing kept as lines (DeckCraft `pct`) shows its number without points, answers the kind it opened with while the value is untouched, and answers points once the value changes.
#[test]
fn a_spacing_in_lines_keeps_its_kind_until_its_value_changes() {
    let lines = ParagraphFormat { space_before: Some(1.5), space_before_kind: SpacingKind::Lines, ..start() };
    let mut harness = open(lines.clone());
    let shown = harness.get_by_role_and_label(Role::SpinButton, "Spacing before").value().unwrap_or_default();
    assert!(shown.starts_with("1.5") && !shown.contains("pt"), "{shown:?}");
    harness.get_by_label("OK").click();
    harness.run();
    let answer = harness.state().answers.last().cloned().expect("OK answers");
    assert_eq!((answer.format.space_before, answer.format.space_before_kind), (Some(1.5), SpacingKind::Lines));

    let mut harness = open(lines);
    type_into(&mut harness, "Spacing before", "12");
    assert_eq!((format(&harness).space_before, format(&harness).space_before_kind), (Some(12.0), SpacingKind::Points));
    assert_eq!(format(&harness).space_after_kind, SpacingKind::Points);
}

// A spacing of 11 lines is out of range (DeckCraft keeps at most 10).
#[test]
fn lines_of_spacing_have_their_own_range() {
    let harness = open(ParagraphFormat { space_after: Some(11.0), space_after_kind: SpacingKind::Lines, ..start() });
    harness.get_by_label_contains("Spacing after must be from 0 to 10");
}
