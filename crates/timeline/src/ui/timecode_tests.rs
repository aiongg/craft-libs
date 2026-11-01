//! The timecode field and panel (egui_kittest).

use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, PointerButton, Rect, Ui, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::timecode::{TimecodeField, TimecodePanel, TimecodePanelSettings, TimecodeRow};
use crate::rates::FrameRate;
use crate::time::Tick;
use crate::timecode::TimeFormat;

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
}

fn show_field(ui: &mut Ui, app: &mut Field) {
    let format = format();
    let out = TimecodeField::new("field", app.time, &format).show(ui);
    app.rect = out.response.rect;
    app.submitted |= out.submitted;
    app.events.extend(out.event);
}

fn field() -> Harness<'static, Field> {
    harness(show_field, Field { time: secs(10.0), events: Vec::new(), submitted: false, rect: Rect::NOTHING })
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

// typing +15 and Enter commits 15 frames later; Esc cancels.
#[test]
fn typing_commits_relative_and_absolute_times() {
    let enter = KeyboardShortcut::new(Modifiers::NONE, Key::Enter);
    let mut harness = field();
    type_into(&mut harness, "+15");
    harness.press(enter);
    let events = &harness.state().events;
    assert!(matches!(events.as_slice(), [EditEvent::Set { value, .. }] if *value == rate().tick_of(255)), "{events:?}");
    assert!(harness.state().submitted);

    let mut harness = field();
    type_into(&mut harness, "00:00:20:00");
    harness.press(enter);
    assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { value, .. }] if *value == secs(20.0)));

    let mut harness = field();
    type_into(&mut harness, "+15");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    assert!(harness.state().events.is_empty());
    assert!(harness.query_by_role(Role::TextInput).is_none(), "the field closed");
    assert_eq!(harness.get_by_role(Role::SpinButton).value().as_deref(), Some("00:00:10:00"));
}

// invalid text keeps the field open and reports no commit.
#[test]
fn a_time_that_does_not_parse_commits_nothing() {
    let mut harness = field();
    type_into(&mut harness, "xyz");
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Enter));
    harness.run();
    assert!(harness.state().events.is_empty());
    let input = harness.get_by_role(Role::TextInput);
    assert_eq!(input.accesskit_node().data().invalid(), Some(egui::accesskit::Invalid::True));
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
