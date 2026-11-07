//! The header row, toggle-column sweeps, row drags and the free-layout move.

use super::kit::{Count, Dismissed, Row};
use super::table::{Free, Table};
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, EditEvent, GesturePhase, ItemSource};
use craft_ui::contract::{
    CollectionEvent, CollectionProps, CollectionResponse, ColumnCommands, ColumnEvent, ColumnKind, ColumnSpec, DropMark, ListDrag, Props as _,
    ReorderRule, SelectMode, ShowCollection as _, ToggleColumn,
};
use craft_ui::runtime::commands::{self, FireSource};
use craft_ui::runtime::drag;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 240.0);
const COL: f32 = 100.0;
/// How far a measured position may differ from the expected one.
const CLOSE: f32 = 1.0;
const NAME: ColumnKey = ColumnKey::new("name");
const SIZE: ColumnKey = ColumnKey::new("size");
const DATE: ColumnKey = ColumnKey::new("date");
const MUTE: ColumnKey = ColumnKey::new("mute");

/// What a collection reported and where its rows were.
#[derive(Default)]
struct Scene {
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    rows: RefCell<Vec<(usize, Rect)>>,
}

impl Scene {
    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.events.borrow_mut().extend(out.events);
    }

    fn note(&self, key: usize, rect: Rect) {
        let mut rows = self.rows.borrow_mut();
        rows.retain(|(held, _)| *held != key);
        rows.push((key, rect));
    }

    fn row(&self, key: usize) -> Rect {
        self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was shown")
    }

    /// The centre of header cell `at`, `width` wide columns from the left edge.
    fn header_cell(&self, at: usize, widths: &[f32]) -> Pos2 {
        let top = self.row(0).min.y;
        let before: f32 = widths[..at].iter().sum();
        Pos2::new(ORIGIN.x + before + widths[at] / 2.0, (ORIGIN.y + top) / 2.0)
    }

    fn columns(&self) -> Vec<ColumnEvent> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Column(column) = event { Some(column.clone()) } else { None })
            .collect()
    }

    fn sorts(&self) -> Vec<(ColumnKey, bool)> {
        self.columns()
            .into_iter()
            .filter_map(|event| if let ColumnEvent::Sort { key, descending, .. } = event { Some((key, descending)) } else { None })
            .collect()
    }

    fn moves(&self) -> Vec<(ColumnKey, usize)> {
        self.columns().into_iter().filter_map(|event| if let ColumnEvent::Moved { key, to, .. } = event { Some((key, to)) } else { None }).collect()
    }

    fn widths(&self) -> Vec<f32> {
        self.columns().into_iter().filter_map(|event| if let ColumnEvent::Resized { width, .. } = event { Some(width) } else { None }).collect()
    }

    fn menus(&self) -> Vec<ColumnKey> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::HeaderMenu(menu) = event { Some(menu.column.clone()) } else { None })
            .collect()
    }

    fn toggles(&self) -> Vec<(usize, bool)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Toggle(edit) = event { edit.value().map(|toggle| (toggle.key, toggle.on)) } else { None })
            .collect()
    }

    fn reorders(&self) -> Vec<(usize, usize)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Reorder(edit) = event { edit.value().map(|moved| (moved.key, moved.to)) } else { None })
            .collect()
    }
}

fn show_table(ui: &mut Ui, props: CollectionProps, source: &dyn ItemSource<usize>, scene: &Scene) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let out = Table::new(props).id_salt("table").show_items(ui, source, &mut |key, cx| {
            scene.note(*key, cx.rect);
            Row::new()
        });
        scene.take(out);
    });
}

fn name_column(width: f32) -> ColumnSpec {
    ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(width)).sortable().resizable().movable()
}

fn file_columns(width: f32) -> CollectionProps {
    CollectionProps::new().columns(vec![
        name_column(width),
        ColumnSpec::new(SIZE, "Size", ColumnWidth::Fixed(COL)).sortable(),
        ColumnSpec::new(DATE, "Date", ColumnWidth::Fixed(COL)).movable(),
    ])
}

/// A header click sorts the column, and the next click on it reverses the order.
#[trial]
fn a_header_click_sorts_and_the_next_click_reverses() {
    let (scene, source) = (Scene::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, file_columns(COL), &source, &scene);
    frames.idle(3, &mut show);
    let widths = [COL; 3];
    for _ in 0..2 {
        pointer::click(&mut frames, scene.header_cell(0, &widths), &mut show);
        frames.idle(2, &mut show);
    }
    assert_eq!(scene.sorts(), vec![(NAME, false), (NAME, true)]);
    pointer::click(&mut frames, scene.header_cell(2, &widths), &mut show);
    assert_eq!(scene.sorts().len(), 2, "a column that is not sortable reports no sort");
}

/// Dragging a header's edge resizes a column the app owns the width of; dragging the header moves it.
#[trial]
fn a_header_drag_resizes_and_moves_a_column() {
    let (scene, source) = (Scene::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let width = scene.widths().last().copied().unwrap_or(COL);
        show_table(ui, file_columns(width), &source, &scene);
    };
    frames.idle(3, &mut show);
    let edge = scene.header_cell(0, &[COL]) + Vec2::new(COL / 2.0 - 1.0, 0.0);
    pointer::drag(&mut frames, edge, Vec2::new(30.0, 0.0), &mut show);
    let width = scene.widths().last().copied().expect("the edge drag reports the new width");
    assert!(width > COL, "the column grew: {width}");
    assert!(scene.moves().is_empty() && scene.sorts().is_empty(), "a resize moves nothing and sorts nothing");

    let widths = [width, COL, COL];
    let from = scene.header_cell(0, &widths);
    let to_second = Vec2::new(width / 2.0 + COL / 2.0, 0.0);
    pointer::drag(&mut frames, from, to_second, &mut show);
    assert_eq!(scene.moves(), vec![(NAME, 1)]);
    assert!(scene.sorts().is_empty(), "a drag is not a click");
}

/// The header's context menu request names the column, and the header commands resolve against it and move it.
#[trial]
fn the_header_context_menu_lists_its_commands() {
    let (scene, source) = (Scene::default(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, file_columns(COL), &source, &scene);
    frames.idle(3, &mut show);
    let at = scene.header_cell(0, &[COL]);
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::secondary(at, true)], &mut show);
    frames.frame(vec![pointer::secondary(at, false)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(scene.menus(), vec![NAME]);
    let state = |command: ColumnCommands| commands::resolve(&frames.ctx, command);
    assert_eq!(ColumnCommands::ALL.map(|command| state(command).label.to_string()), ["Resize Column…", "Move Column Left", "Move Column Right"]);
    assert_eq!(ColumnCommands::ALL.map(|command| state(command).enabled), [true, false, true], "the first column cannot move left");
    assert!(commands::fire(&frames.ctx, ColumnCommands::MoveRight, FireSource::Menu));
    frames.idle(2, &mut show);
    assert_eq!(scene.moves(), vec![(NAME, 1)]);
}

/// Rows with a toggle cell.
struct Switches {
    on: RefCell<Vec<bool>>,
}

impl DisplayOrder<usize> for Switches {
    fn len(&self) -> usize {
        self.on.borrow().len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.len()).then_some(*key)
    }
}

impl ItemSource<usize> for Switches {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == MUTE && self.on.borrow()[at]
    }
}

/// A press on a sweepable toggle cell flips it, and the drag over the rows below sets each of them once.
#[trial]
fn a_toggle_sweep_sets_every_row_it_crosses_once() {
    const ROWS: usize = 6;
    const LAST: usize = 3;
    let (scene, source) = (Scene::default(), Switches { on: RefCell::new(vec![false; ROWS]) });
    let props = || {
        CollectionProps::new().columns(vec![
            ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
            ColumnSpec::new(MUTE, "Mute", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
        ])
    };
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (key, on) in scene.toggles() {
            source.on.borrow_mut()[key] = on;
        }
        show_table(ui, props(), &source, &scene);
    };
    frames.idle(3, &mut show);
    let cell = |key: usize| Pos2::new(ORIGIN.x + COL * 1.5, scene.row(key).center().y);
    let by = cell(LAST) - cell(0);
    pointer::drag(&mut frames, cell(0), by, &mut show);
    assert_eq!(scene.toggles(), (0..=LAST).map(|key| (key, true)).collect::<Vec<_>>());
}

/// Turning on a cell of an exclusive toggle column turns the column's other cells off.
#[trial]
fn turning_on_an_exclusive_toggle_turns_the_others_off() {
    let (scene, source) = (Scene::default(), Switches { on: RefCell::new(vec![true, false, false]) });
    let props = || {
        CollectionProps::new().columns(vec![
            ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(COL)),
            ColumnSpec::new(MUTE, "Solo", ColumnWidth::Fixed(COL)).kind(ColumnKind::Toggle(ToggleColumn::exclusive())),
        ])
    };
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, props(), &source, &scene);
    frames.idle(3, &mut show);
    pointer::click(&mut frames, Pos2::new(ORIGIN.x + COL * 1.5, scene.row(2).center().y), &mut show);
    let mut toggles = scene.toggles();
    toggles.sort_unstable();
    assert_eq!(toggles, vec![(0, false), (2, true)]);
}

/// A press on the empty space below the rows empties the selection only when the collection allows it.
#[trial]
fn a_press_below_the_rows_empties_the_selection_only_when_allowed() {
    let source = Count(3);
    for allow in [false, true] {
        let scene = Scene::default();
        let props = || CollectionProps::new().select(SelectMode::Single).allow_empty(allow);
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_table(ui, props(), &source, &scene);
        frames.idle(3, &mut show);
        pointer::click(&mut frames, scene.row(1).center(), &mut show);
        let below = Pos2::new(scene.row(1).center().x, scene.row(2).max.y + 40.0);
        pointer::click(&mut frames, below, &mut show);
        let cleared = scene.events.borrow().iter().any(|event| matches!(event, CollectionEvent::Selection(craft_behavior::SelectionChange::Clear)));
        assert_eq!(cleared, allow, "allow_empty = {allow}");
    }
}

/// A row dragged to another position shows the insertion mark while it is held and reports one `Reorder` at the drop.
#[trial]
fn a_dragged_row_shows_the_mark_and_reports_one_reorder() {
    let (scene, source) = (Scene::default(), Count(6));
    let props = || CollectionProps::new().select(SelectMode::Single).reorder(ReorderRule::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_table(ui, props(), &source, &scene);
    frames.idle(3, &mut show);
    let (from, below) = (scene.row(1).center(), scene.row(3));
    let target = Pos2::new(from.x, below.max.y - 1.0);
    frames.frame(vec![pointer::moved(from)], &mut show);
    frames.frame(vec![pointer::press(from)], &mut show);
    for step in 1..=4 {
        frames.frame(vec![pointer::moved(from + (target - from) * (step as f32 / 4.0))], &mut show);
    }
    frames.idle(1, &mut show);
    assert!(drag::active::<ListDrag<usize>>(&frames.ctx).is_some_and(|payload| payload.grabbed == 1));
    match drag::mark(&frames.ctx) {
        Some(DropMark::Line { from, to, .. }) => {
            assert!((from.y - below.max.y).abs() < CLOSE && (to.y - below.max.y).abs() < CLOSE, "the mark lies under row 3")
        }
        other => panic!("the drag shows an insertion line, found {other:?}"),
    }
    assert!(scene.reorders().is_empty(), "nothing is reported while the row is held");
    frames.frame(vec![pointer::release(target)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(scene.reorders(), vec![(1, 3)]);
    assert!(drag::mark(&frames.ctx).is_none(), "the mark ends with the drag");
}

/// Dragging an item of a free layout reports a move gesture whose commit is the pointer's offset.
#[trial]
fn a_free_layout_item_moves_by_drag() {
    let scene = Scene::default();
    let source = Count(2);
    let rects =
        vec![Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(80.0, 40.0)), Rect::from_min_size(Pos2::new(200.0, 120.0), Vec2::new(80.0, 40.0))];
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = Free::new(rects.clone()).id_salt("free").show_items(ui, &source, &mut |_, _| Row::new());
            scene.take(out);
        });
    };
    frames.idle(3, &mut show);
    let from = ORIGIN + rects[0].center().to_vec2();
    let by = Vec2::new(40.0, 30.0);
    pointer::drag(&mut frames, from, by, &mut show);
    let moves: Vec<EditEvent<Vec2>> =
        scene.events.borrow().iter().filter_map(|event| if let CollectionEvent::Move(edit) = event { Some(edit.clone()) } else { None }).collect();
    assert_eq!(moves.first().and_then(EditEvent::phase), Some(GesturePhase::Begin));
    assert_eq!(moves.last().and_then(EditEvent::phase), Some(GesturePhase::Commit));
    let offset = moves.last().and_then(|edit| edit.value().copied()).expect("the commit carries the offset");
    assert!((offset - by).length() < CLOSE, "the commit is the pointer's offset: {offset:?}");
    assert_eq!(moves.iter().filter(|edit| edit.phase() == Some(GesturePhase::Commit)).count(), 1);
}
