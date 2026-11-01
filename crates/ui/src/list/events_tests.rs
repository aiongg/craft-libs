//! `EventLog` in a harness.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

const NOW: u64 = 10_000_000;

fn event(id: u64, level: EventLevel, message: &str, count: u32, minutes_ago: u64) -> LogEvent {
    LogEvent { id, level, source: "Auto-save".into(), message: message.into(), count, time_ms: NOW - minutes_ago * 60_000 }
}

fn sample() -> Vec<LogEvent> {
    vec![
        event(1, EventLevel::Info, "Saved", 1, 10),
        event(2, EventLevel::Warning, "Disk almost full", 1, 3),
        event(3, EventLevel::Error, "Render failed", 3, 3),
        event(4, EventLevel::Warning, "Slow disk", 1, 0),
    ]
}

type State = (Vec<LogEvent>, EventLogState, bool);

fn harness(min_level: EventLevel) -> Harness<'static, State> {
    Harness::<State>::builder().with_size([420.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, (events, state, cleared)| {
            *cleared |= EventLog::new("events", events, state, NOW).show(ui).clear_all;
        },
        (sample(), EventLogState { min_level, selected: None }, false),
    )
}

// Warnings and Errors hides info events; the counts still count every level.
#[test]
fn the_filter_hides_events_below_its_level() {
    let mut harness = harness(EventLevel::Warning);
    harness.run();
    assert!(harness.query_by_role_and_label(Role::ListBoxOption, "Info: Saved, 10 minutes ago").is_none());
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: Disk almost full, 3 minutes ago");
    harness.get_by_label("1 message");
}

// A click selects a row and shows the details box; a second click deselects it.
#[test]
fn clicking_an_event_shows_its_details_and_again_hides_them() {
    let mut harness = harness(EventLevel::Info);
    harness.run();
    let details = "Warning · Auto-save · 3 minutes ago";
    assert!(harness.query_by_label(details).is_none());
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: Disk almost full, 3 minutes ago").click();
    harness.run();
    assert_eq!(harness.state().1.selected, Some(2));
    harness.get_by_label(details);
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: Disk almost full, 3 minutes ago").click();
    harness.run();
    assert_eq!(harness.state().1.selected, None);
    assert!(harness.query_by_label(details).is_none());
}

// An event with count 3 reads "message  (×3)".
#[test]
fn a_repeated_event_shows_its_count() {
    let mut harness = harness(EventLevel::Info);
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "Error: Render failed  (×3), 3 minutes ago");
}

// A row 3 minutes older than `now_ms` reads "3 minutes ago" (`relative_age`), one newer than
// `now_ms` "less than a minute ago"; the bar reads "1 error" and "2 warnings".
#[test]
fn rows_show_their_age_and_the_bar_counts_in_plural_forms() {
    let mut harness = harness(EventLevel::Info);
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: Disk almost full, 3 minutes ago");
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: Slow disk, less than a minute ago");
    harness.get_by_label("1 error");
    harness.get_by_label("2 warnings");
    harness.get_by_label("1 message");
}

// Clear All sets `clear_all` and the state's `selected` is `None`.
#[test]
fn clear_all_reports_and_drops_the_selection() {
    let mut harness = harness(EventLevel::Info);
    harness.state_mut().1.selected = Some(2);
    harness.run();
    harness.get_by_label("Clear All").click();
    harness.run();
    assert!(harness.state().2);
    assert_eq!(harness.state().1.selected, None);
}
