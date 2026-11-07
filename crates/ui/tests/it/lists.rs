//! The pure logic of the list family.

use craft_behavior::{DisplayOrder as _, SelectionChange, SelectionModel as _};
use craft_i18n::Format;
use craft_ui::contract::CollectionEvent;
use craft_ui::list::{
    EventLevel, EventLog, EventLogState, EventWords, Finding, FindingsWords, LogEvent, OpenSet, Severity, UndoMark, UndoWords, footer_text,
    summary_text, undo_count,
};
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::trial;

fn log() -> Vec<LogEvent> {
    vec![
        LogEvent::new(1, EventLevel::Info, "Open", "File opened", 0),
        LogEvent::new(2, EventLevel::Error, "Render", "Out of memory", 10_000),
        LogEvent::new(3, EventLevel::Warning, "Auto-save", "Disk almost full", 20_000).count(3),
    ]
}

#[trial]
fn the_log_shows_newest_first_at_or_above_the_level() {
    let events = log();
    let all = EventLog::new(&events, &EventLogState::default());
    assert_eq!((0..all.len()).map(|at| all.key(at)).collect::<Vec<_>>(), [3, 2, 1]);
    let mut warnings = EventLogState::default();
    warnings.min_level = EventLevel::Warning;
    let serious = EventLog::new(&events, &warnings);
    assert_eq!((0..serious.len()).map(|at| serious.key(at)).collect::<Vec<_>>(), [3, 2]);
    assert_eq!(serious.index_of(&1), None);
    let counts = serious.counts();
    assert_eq!((counts.info, counts.warning, counts.error), (1, 1, 1));
}

#[trial]
fn a_repeated_event_shows_its_count_and_a_future_event_is_new() {
    let words = EventWords::english();
    let format = Format::from_tag("en-US");
    let events = log();
    assert_eq!(events[2].message_with_count(&words, &format), "Disk almost full  (\u{d7}3)");
    assert_eq!(events[1].message_with_count(&words, &format), "Out of memory");
    assert_eq!(events[2].message_with_count(&words, &Format::from_tag("ar-EG")), "Disk almost full  (\u{d7}\u{663})");
    assert_eq!(events[1].age_seconds(5_000), 0);
    assert_eq!(events[1].age_seconds(70_000), 60);
    assert_eq!(events[2].details_line(&words, "2 min ago"), "Warning \u{b7} Auto-save \u{b7} 2 min ago");
}

fn sample() -> Vec<Finding> {
    vec![
        Finding::new(1, Severity::Error, "Missing font"),
        Finding::new(2, Severity::Warning, "Low resolution").detail("page 2"),
        Finding::new(3, Severity::Warning, "Overset text"),
    ]
}

#[trial]
fn the_summary_counts_errors_and_warnings() {
    let words = FindingsWords::english();
    assert_eq!(summary_text(&words, &Format::from_tag("en-US"), &[]), "No errors");
    assert_eq!(summary_text(&words, &Format::from_tag("en-US"), &sample()), "1 error, 2 warnings");
    assert_eq!(summary_text(&words, &Format::from_tag("en-US"), &sample()[1..]), "0 errors, 2 warnings");
    assert_eq!(summary_text(&words, &Format::from_tag("en-US"), &sample()[..1]), "1 error, 0 warnings");
    assert_eq!(summary_text(&words, &Format::from_tag("ar-EG"), &sample()), "\u{661} error, \u{662} warnings");
}

#[trial]
fn apply_opens_one_group_or_every_group() {
    let groups = ["a", "b"];
    let mut open = OpenSet::new();
    open.apply(&"a", true, false, groups.iter());
    assert_eq!((open.is_open(&"a"), open.is_open(&"b")), (true, false));
    open.apply(&"a", true, true, groups.iter());
    assert!(open.is_open(&"b"));
    open.apply(&"b", false, true, groups.iter());
    assert!(!open.is_open(&"a") && !open.is_open(&"b"));
}

#[trial]
fn a_mark_covers_the_row_and_every_newer_one() {
    let mut mark = UndoMark::default();
    assert!(!mark.contains(&0));
    mark.apply(SelectionChange::Set(vec![2]));
    assert_eq!((mark.contains(&0), mark.contains(&2), mark.contains(&3)), (true, true, false));
    assert_eq!(mark.actions(), 3);
    mark.apply(SelectionChange::Clear);
    assert_eq!(mark.actions(), 0);
}

#[trial]
fn the_footer_counts_marked_actions_and_a_pick_undoes_through_its_row() {
    let words = UndoWords::english();
    assert_eq!(footer_text(&words, &UndoMark::default()), None);
    assert_eq!(footer_text(&words, &UndoMark::at(0)).as_deref(), Some("Undo 1 Action"));
    assert_eq!(footer_text(&words, &UndoMark::at(3)).as_deref(), Some("Undo 4 Actions"));
    assert_eq!(undo_count(&[CollectionEvent::LeadMoved(1)]), None);
    assert_eq!(undo_count(&[CollectionEvent::Activate(2)]), Some(3));
}
