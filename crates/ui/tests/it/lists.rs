//! The pure logic of the list family.

use craft_behavior::{ColumnKey, DisplayOrder, RowAt, SelectionChange, SelectionModel as _};
use craft_i18n::Format;
use craft_ui::contract::{CollectionEvent, Show as _};
use craft_ui::list::{
    EventLevel, EventLog, EventLogState, EventWords, Finding, FindingsWords, List, ListRow, LogEvent, OpenSet, Severity, UndoMark, UndoWords,
    footer_text, summary_text, undo_count,
};
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::internals::place_row as place;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};

/// `count` rows keyed by position.
struct Rows(usize);

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

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
    let order = Rows(4);
    let covers = |mark: &UndoMark, key: usize| mark.contains(RowAt::new(&key, key, &order));
    assert!(!covers(&mark, 0));
    mark.apply(SelectionChange::Set(vec![2]));
    assert_eq!((covers(&mark, 0), covers(&mark, 2), covers(&mark, 3)), (true, true, false));
    assert_eq!(mark.actions(), 3);
    mark.apply(SelectionChange::Clear);
    assert_eq!(mark.actions(), 0);
}

#[trial]
fn the_footer_counts_marked_actions_and_a_pick_undoes_through_its_row() {
    let words = UndoWords::english();
    assert_eq!(footer_text(&words, &Format::from_tag("en-US"), &UndoMark::default()), None);
    assert_eq!(footer_text(&words, &Format::from_tag("en-US"), &UndoMark::at(0)).as_deref(), Some("Undo 1 Action"));
    assert_eq!(footer_text(&words, &Format::from_tag("en-US"), &UndoMark::at(3)).as_deref(), Some("Undo 4 Actions"));
    assert_eq!(undo_count(&[CollectionEvent::Selection(SelectionChange::Clear)]), None);
    assert_eq!(undo_count(&[CollectionEvent::Activate(2)]), Some(3));
}

#[trial]
fn an_undo_count_shows_locale_digits() {
    let words = UndoWords::english();
    assert_eq!(footer_text(&words, &Format::from_tag("ar-EG"), &UndoMark::at(2)).as_deref(), Some("Undo \u{663} Actions"));
}

#[trial]
fn a_list_predicate_closure_needs_no_parameter_types() {
    let _ = List::<usize>::new("Layers")
        .can_rename(|key| key % 2 == 0)
        .can_drag(|key| *key > 0)
        .accept_from(|from| from == egui::Id::NULL)
        .accepts(|_, over, insert| over.is_some_and(|over| *over < insert));
}

const NAME: ColumnKey = ColumnKey::new("name");
const NOTE: ColumnKey = ColumnKey::new("note");

/// The height a row with one text cell takes when the cell is on one line or wrapped, and the fixture that shows it.
fn text_cell_row(text: &'static str, wrapped: bool) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let width = ui.available_width();
        let cells = [
            (NAME, Rect::from_min_size(Pos2::ZERO, Vec2::new(width - 80.0, 20.0))),
            (NOTE, Rect::from_min_size(Pos2::new(width - 80.0, 0.0), Vec2::new(80.0, 20.0))),
        ];
        let row = ListRow::new("Name").cells(&cells);
        let _ = if wrapped { row.cell_lines(NOTE, text) } else { row.cell(NOTE, text) }.show(ui);
    });
    fixture.settle();
    fixture
}

#[trial]
fn a_text_cell_is_a_cell_node_in_its_column_and_a_wrapped_one_makes_the_row_taller() {
    use craft_ui::contract::Role;
    let note = "A note long enough that it wraps onto several lines in a narrow column";
    let one_line = text_cell_row(note, false);
    assert!(one_line.by_label(note).is_some(), "the text cell has a node named by its text");
    assert_eq!(one_line.by_role(Role::CELL).len(), 2, "the label column and the text column each show a cell");
    let wrapped = text_cell_row(note, true);
    assert!(wrapped.shown().height() > one_line.shown().height(), "the row measures the wrapped cell");
}

/// With the label column narrowed, the label stays in the label column's rect and the text cells after it do not
/// lead the label band.
#[trial]
fn a_narrow_label_column_keeps_the_label_in_its_rect() {
    let pages = ColumnKey::new("pages");
    let size = ColumnKey::new("size");
    let cells = [
        (NAME, Rect::from_min_size(Pos2::ZERO, Vec2::new(50.0, 20.0))),
        (pages.clone(), Rect::from_min_size(Pos2::new(50.0, 0.0), Vec2::new(72.0, 20.0))),
        (size.clone(), Rect::from_min_size(Pos2::new(122.0, 0.0), Vec2::new(178.0, 20.0))),
    ];
    let row = ListRow::new("Name").cells(&cells).cell(pages, "12").cell(size, "2.4 MB");
    let label = place(&row, 300.0, false).label;
    assert!(label.width() > 0.0 && label.min.x >= 0.0 && label.min.x < 50.0, "the label starts inside the label column");
}
