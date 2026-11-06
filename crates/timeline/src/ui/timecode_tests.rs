//! The timecode field and panel (egui_kittest).

use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, events, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, PointerButton, Rect, Ui, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::timecode::{TimecodeField, TimecodePanel, TimecodePanelSettings, TimecodeRow};
use crate::rates::FrameRate;
use crate::time::Tick;
use crate::timecode::{TimeDisplay, TimeFormat, TimecodeError};

fn rate() -> FrameRate {
    FrameRate::new(24, 1)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(rate())
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
    let mut harness = Harness::<S>::builder()
        .with_size([320.0, 200.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, state);
    harness.run();
    harness
}

struct Field {
    time: Tick,
    events: Vec<EditEvent<Tick>>,
    submitted: bool,
    rect: Rect,
    errors: Vec<TimecodeError>,
    preset: Option<&'static str>,
}

fn show_field(ui: &mut Ui, app: &mut Field) {
    let format = format();
    let mut field = TimecodeField::new("field", app.time, &format);
    if let Some(text) = app.preset {
        field = field.typing(text);
    }
    let out = field.show_full(ui);
    app.rect = out.edit.response.rect;
    app.submitted |= out.edit.submitted;
    app.events.extend(out.edit.event);
    app.errors.extend(out.error);
}

fn field() -> Harness<'static, Field> {
    harness(show_field, Field { time: secs(10.0), events: Vec::new(), submitted: false, rect: Rect::NOTHING, errors: Vec::new(), preset: None })
}

fn play(harness: &mut Harness<'_, Field>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
    harness.run();
}

// a drag of 10 pt moves 10 frames, 100 with Shift.
#[test]
fn dragging_the_field_scrubs_a_frame_per_point() {
    for (modifiers, frames) in [(Modifiers::NONE, 10), (Modifiers::SHIFT, 100)] {
        let mut harness = field();
        let centre = harness.state().rect.center();
        harness.drag(centre, centre + vec2(10.0, 0.0), modifiers);
        let events = &harness.state().events;
        let expected = rate().tick_of(240 + frames);
        assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
        assert!(matches!(events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == expected), "{events:?}");
    }
    let mut harness = field();
    harness.state_mut().time = Tick::ZERO;
    harness.run();
    let centre = harness.state().rect.center();
    harness.drag(centre, centre - vec2(30.0, 0.0), Modifiers::NONE);
    assert!(harness.state().events.iter().all(|event| event.value().is_none_or(|value| *value >= Tick::ZERO)), "never before the sequence start");
}

fn type_into(harness: &mut Harness<'_, Field>, text: &str) {
    let centre = harness.state().rect.center();
    harness.click_at(centre);
    harness.type_text(text);
}

// typing +15 and Enter commits 15 frames later, as one typing run that reports no draft on the way;
// Esc cancels.
#[test]
fn typing_commits_relative_and_absolute_times() {
    let enter = KeyboardShortcut::new(Modifiers::NONE, Key::Enter);
    let mut harness = field();
    type_into(&mut harness, "+15");
    harness.press(enter);
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Commit { value, .. }) if *value == rate().tick_of(255)), "{events:?}");
    assert!(events.iter().all(|event| !matches!(event, EditEvent::Update { .. })), "{events:?}");
    assert!(harness.state().submitted);

    let mut harness = field();
    type_into(&mut harness, "00:00:20:00");
    harness.press(enter);
    assert!(matches!(harness.state().events.last(), Some(EditEvent::Commit { value, .. }) if *value == secs(20.0)));

    let mut harness = field();
    type_into(&mut harness, "+15");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    assert!(harness.state().events.is_empty());
    assert!(harness.query_by_role(Role::TextInput).is_none(), "the field closed");
    assert_eq!(harness.get_by_role(Role::SpinButton).value().as_deref(), Some("00:00:10:00"));
}

// invalid text commits nothing and stays in the field, marked invalid.
#[test]
fn a_time_that_does_not_parse_commits_nothing() {
    let mut harness = field();
    type_into(&mut harness, "xyz");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
    harness.run();
    assert!(harness.state().events.is_empty());
    let input = harness.get_by_role(Role::TextInput);
    assert_eq!(input.accesskit_node().data().invalid(), Some(egui::accesskit::Invalid::True));
    assert_eq!(harness.state().errors, [TimecodeError::NotATime("xyz".into())], "Enter reports the error and keeps the text");
}

// a click away from text that doesn't parse commits nothing, reports the error and leaves the text
// marked invalid, as every craft-ui text field does; the focus is not pulled back.
#[test]
fn clicking_away_from_bad_text_keeps_it_marked_invalid() {
    let mut harness = field();
    type_into(&mut harness, "xyz");
    let away = harness.state().rect.center() + vec2(0.0, 80.0);
    play(&mut harness, events::click(away, PointerButton::Primary, 1, Modifiers::NONE));
    let input = harness.get_by_role(Role::TextInput);
    assert_eq!(input.accesskit_node().data().invalid(), Some(egui::accesskit::Invalid::True));
    assert!(harness.state().events.is_empty());
    assert_eq!(harness.state().errors, [TimecodeError::NotATime("xyz".into())]);
}

// Left on the whole selected text puts the caret on its last digit, so Backspace removes the
// digit before it; egui alone would jump to the start.
#[test]
fn left_on_the_selected_value_goes_to_its_last_digit() {
    let mut harness = field();
    let centre = harness.state().rect.center();
    play(&mut harness, events::click(centre, PointerButton::Primary, 1, Modifiers::NONE));
    play(&mut harness, events::key_press(Key::ArrowLeft, Modifiers::NONE));
    play(&mut harness, events::key_press(Key::Backspace, Modifiers::NONE));
    assert_eq!(harness.get_by_role(Role::TextInput).value().as_deref(), Some("00:00:10:0"));
}

// the preset opens the field in its typing state once; Esc closes it for good.
#[test]
fn a_preset_opens_the_field_in_typing_state() {
    let mut harness = harness(
        show_field,
        Field { time: secs(10.0), events: Vec::new(), submitted: false, rect: Rect::NOTHING, errors: Vec::new(), preset: Some("12") },
    );
    assert_eq!(harness.get_by_role(Role::TextInput).value().as_deref(), Some("12"));
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    harness.run();
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

struct Panel {
    settings: TimecodePanelSettings,
    changed: usize,
}

fn show_panel(ui: &mut Ui, app: &mut Panel) {
    let text = |row: &TimecodeRow| format!("{:?}", row.kind);
    let out = TimecodePanel::new("panel", &mut app.settings, &text).show(ui);
    app.changed += usize::from(out.settings_changed);
}

// a row that shows its own display can go back to the sequence's display.
#[test]
fn a_row_can_return_to_the_sequence_display() {
    let row = TimecodeRow { display: Some(TimeDisplay::Frames), ..TimecodeRow::default() };
    let mut harness = harness(show_panel, Panel { settings: TimecodePanelSettings { rows: vec![row] }, changed: 0 });
    let value = harness.get_by_value("Current Time \u{b7} Active Monitor: CurrentTime").rect();
    harness.click_at_with(value.center(), PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label("Sequence Display");
    for _ in 0..7 {
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp));
    }
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
    assert_eq!(harness.state().settings.rows[0].display, None);
    assert_eq!(harness.state().changed, 1);
}

// + adds a row; the row menu's Remove Row removes it.
#[test]
fn panel_rows_can_be_added_and_removed() {
    let mut harness = harness(show_panel, Panel { settings: TimecodePanelSettings { rows: vec![TimecodeRow::default()] }, changed: 0 });
    harness.get_by_role_and_label(Role::Button, "Add a timecode row").click();
    harness.run();
    assert_eq!(harness.state().settings.rows.len(), 2);
    assert_eq!(harness.state().changed, 1);
    let second = harness.get_by_value("Duration \u{b7} Active Monitor: Duration").rect();
    harness.click_at_with(second.center(), PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label("Remove Row");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp));
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
    assert_eq!(harness.state().settings.rows.len(), 1);
    assert_eq!(harness.state().changed, 2);
    assert_eq!(harness.state().settings.rows[0], TimecodeRow::default());
}
