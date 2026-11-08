//! `CellGrid`: geometry, keys, header edges, the fill handle, the range border, copy mode and the chrome on the sheet.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use craft_behavior::{CellSelection, EditEvent, PartKey};
use craft_table::ui::{
    CellAt, CellEdit, CellGrid, CellProblem, CellRange, ColumnFilter, GridEdit, GridLayout, GridModel, GridState, LineIndex, SHEET_COLUMNS,
    SHEET_ROWS, SheetGeometry, SheetMetrics, absolute_reference, column_letters,
};
use craft_ui::contract::PaintUi;
use craft_ui::primitives::TextRun;
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};

/// The default line sizes of the test sheet, points at 100 %.
const ROW: f32 = 20.0;
const COLUMN: f32 = 64.0;

struct Sheet {
    rows: LineIndex,
    columns: LineIndex,
    zoom: Rc<Cell<f32>>,
}

impl Sheet {
    fn new(hidden_column: Option<u32>, zoom: Rc<Cell<f32>>) -> Self {
        let custom: BTreeMap<u32, f32> = hidden_column.into_iter().map(|column| (column, 0.0)).collect();
        Sheet { rows: LineIndex::new(&BTreeMap::new(), ROW, SHEET_ROWS), columns: LineIndex::new(&custom, COLUMN, SHEET_COLUMNS), zoom }
    }
}

impl GridModel for Sheet {
    fn rows(&self) -> &LineIndex {
        &self.rows
    }

    fn columns(&self) -> &LineIndex {
        &self.columns
    }

    fn last_used(&self) -> CellAt<u32> {
        CellAt::new(9, 4)
    }

    fn paint_cell(&self, ui: &mut PaintUi<'_>, rect: Rect, cell: CellAt<u32>, zoom: f32) {
        self.zoom.set(zoom);
        ui.draw().text(rect, &TextRun::new(self.cell_text(cell)).elide());
    }

    fn cell_text(&self, cell: CellAt<u32>) -> String {
        format!("{}", cell.row * 10 + cell.column)
    }
}

struct Rig {
    fixture: Fixture,
    state: Rc<RefCell<GridState>>,
    edits: Rc<RefCell<Vec<GridEdit>>>,
    marks: Rc<RefCell<Vec<CellRange<u32>>>>,
    committed: Rc<RefCell<Vec<CellEdit>>>,
    zoom: Rc<Cell<f32>>,
}

fn rig(hidden_column: Option<u32>, selection: CellSelection<u32>, zoom: f32) -> Rig {
    let state = Rc::new(RefCell::new(GridState { selection, zoom, ..GridState::default() }));
    let edits = Rc::new(RefCell::new(Vec::new()));
    let marks = Rc::new(RefCell::new(Vec::new()));
    let committed = Rc::new(RefCell::new(Vec::new()));
    let seen_zoom = Rc::new(Cell::new(0.0));
    let (shown_state, shown_edits, shown_marks, shown_zoom) = (state.clone(), edits.clone(), marks.clone(), seen_zoom.clone());
    let shown_committed = committed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let model = Sheet::new(hidden_column, shown_zoom.clone());
        let mut state = shown_state.borrow().clone();
        let marks = shown_marks.borrow().clone();
        let out = CellGrid::new("Sheet", &model, &mut state).copy_mark(&marks).show_sheet(ui);
        *shown_state.borrow_mut() = state;
        shown_edits.borrow_mut().extend(out.edits);
        shown_committed.borrow_mut().extend(out.committed);
    });
    fixture.settle();
    Rig { fixture, state, edits, marks, committed, zoom: seen_zoom }
}

impl Rig {
    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(2);
    }

    fn active(&self) -> Option<CellAt<u32>> {
        self.state.borrow().selection.active().copied()
    }

    fn take_edits(&self) -> Vec<GridEdit> {
        self.edits.borrow_mut().drain(..).collect()
    }

    fn part(&self, key: PartKey, index: u32) -> craft_ui::testing::Target {
        let id = self.fixture.root_id().expect("the grid showed");
        self.fixture.part(id, key.at(u64::from(index))).expect("the part has a node")
    }
}

fn a1() -> CellSelection<u32> {
    CellSelection::cell(CellAt::new(0, 0))
}

fn geometry(zoom: f32, rtl: bool) -> SheetGeometry {
    let rows = LineIndex::new(&BTreeMap::new(), ROW, SHEET_ROWS);
    let columns = LineIndex::new(&BTreeMap::new(), COLUMN, SHEET_COLUMNS);
    let room = Vec2::new(columns.start(6) as f32, rows.start(15) as f32);
    let layout = GridLayout::new(Rect::from_min_size(Pos2::ZERO, room), rows, columns).zoom(zoom).frozen(1, 1).right_to_left(rtl);
    SheetGeometry::new(layout, SheetMetrics::new(ROW, 8.0, 4.0))
}

#[trial]
fn column_letters_run_a_to_xfd() {
    assert_eq!(column_letters(0), "A");
    assert_eq!(column_letters(25), "Z");
    assert_eq!(column_letters(26), "AA");
    assert_eq!(column_letters(16_383), "XFD");
    assert_eq!(absolute_reference(CellAt::new(2, 1)), "$B$3");
}

#[trial]
fn geometry_maps_cells_with_frozen_panes_and_zoom() {
    let plain = geometry(1.0, false);
    let zoomed = geometry(2.0, false);
    let cell = CellAt::new(3, 2);
    let (a, b) = (plain.cell_rect(cell), zoomed.cell_rect(cell));
    assert_eq!(b.width(), a.width() * 2.0, "a cell is twice as wide at 200 %");
    assert_eq!(plain.cell_at(a.center()), cell, "the point in a cell maps back to it");
    assert_eq!(plain.frozen(), (1, 1));
    assert!(plain.cell_rect(CellAt::new(0, 0)).max.x <= plain.cell_rect(CellAt::new(0, 1)).min.x, "the frozen column ends where the next begins");
}

#[trial]
fn right_to_left_mirrors_the_screen() {
    let sheet = geometry(1.0, true);
    let cell = sheet.cell_rect(CellAt::new(0, 0));
    let shown = sheet.mirror_rect(cell);
    assert!(shown.left() > cell.left(), "the first column is at the end of the screen");
    assert_eq!(sheet.mirror_rect(shown), cell, "mirroring twice returns the rect");
}

#[trial]
fn arrows_tab_and_enter_move_the_active_cell() {
    let mut rig = rig(None, a1(), 1.0);
    rig.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.active(), Some(CellAt::new(1, 0)));
    rig.press(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.active(), Some(CellAt::new(1, 1)), "Tab advances along the row");
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.active(), Some(CellAt::new(2, 1)), "Enter advances down");
    rig.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(rig.active(), Some(CellAt::new(2, 0)), "Shift+Tab goes back");
}

#[trial]
fn typing_starts_an_edit_and_delete_clears() {
    let mut rig = rig(None, a1(), 1.0);
    rig.fixture.type_text("x");
    rig.fixture.idle(2);
    assert!(rig.take_edits().iter().any(|edit| matches!(edit, GridEdit::BeginEdit { typed: Some(text) } if text == "x")));
    rig.press(Key::Escape, Modifiers::NONE);
    rig.press(Key::Delete, Modifiers::NONE);
    assert!(rig.take_edits().contains(&GridEdit::Clear));
}

fn resizes(edits: &[GridEdit]) -> Vec<&EditEvent<f32>> {
    edits.iter().filter_map(|edit| if let GridEdit::ResizeColumn { width, .. } = edit { Some(width) } else { None }).collect()
}

#[trial]
fn header_edges_resize_and_double_click_autofits() {
    let mut rig = rig(None, a1(), 1.0);
    let edge = rig.part(PartKey::new("column_edge"), 1);
    rig.fixture.drag(&edge, Vec2::X * (ROW + ROW / 2.0));
    let edits = rig.take_edits();
    let events = resizes(&edits);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "the drag begins one gesture");
    assert!(
        matches!(events.last(), Some(EditEvent::Commit { after, .. }) if (*after - (COLUMN + ROW + ROW / 2.0)).abs() < 1.0),
        "and commits the dragged width"
    );
    rig.fixture.click(&edge).click(&edge);
    assert!(rig.take_edits().iter().any(|edit| matches!(edit, GridEdit::AutoFitColumn { column: 1 })), "a double click autofits");
}

#[trial]
fn escape_cancels_a_header_resize() {
    let mut rig = rig(None, a1(), 1.0);
    let edge = rig.part(PartKey::new("column_edge"), 1);
    rig.fixture.press(&edge);
    rig.fixture.frame(vec![craft_ui::testing::pointer::moved(edge.center() + Vec2::X * ROW)]);
    rig.press(Key::Escape, Modifiers::NONE);
    let edits = rig.take_edits();
    assert!(resizes(&edits).iter().any(|event| matches!(event, EditEvent::Cancel { .. })), "Esc cancels the resize");
    assert!(!resizes(&edits).iter().any(|event| matches!(event, EditEvent::Commit { .. })));
}

#[trial]
fn a_double_click_on_a_hidden_edge_unhides() {
    let mut rig = rig(Some(2), a1(), 1.0);
    let edge = rig.part(PartKey::new("column_edge"), 1);
    rig.fixture.click(&edge).click(&edge);
    assert!(rig.take_edits().iter().any(|edit| matches!(edit, GridEdit::UnhideColumn { column: 2 })));
}

fn selected_block() -> CellSelection<u32> {
    CellSelection::range(CellRange::cells(CellAt::new(1, 1), CellAt::new(2, 2)))
}

#[trial]
fn the_fill_handle_drags_a_range() {
    let mut rig = rig(None, selected_block(), 1.0);
    let handle = rig.part(PartKey::new("fill_handle"), 0);
    rig.fixture.drag(&handle, Vec2::Y * (ROW * 2.0));
    let fills: Vec<_> =
        rig.take_edits().into_iter().filter_map(|edit| if let GridEdit::Fill { range } = edit { Some(range) } else { None }).collect();
    assert!(matches!(fills.first(), Some(EditEvent::Begin { .. })));
    assert!(
        matches!(fills.last(), Some(EditEvent::Commit { after, .. }) if after.rows().end() - after.rows().start() == 3),
        "the range grows two rows below the block"
    );
}

// A secondary drag of the fill handle reports the menu at release with the range it grew to and
// fills nothing.
#[trial]
fn a_secondary_drag_of_the_fill_handle_reports_the_menu() {
    let mut rig = rig(None, selected_block(), 1.0);
    let from = rig.part(PartKey::new("fill_handle"), 0).center();
    let to = from + Vec2::Y * (ROW * 2.0);
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    rig.fixture.frame(vec![Event::PointerMoved(from)]);
    rig.fixture.frame(vec![button(from, true)]);
    for step in 1..=4 {
        rig.fixture.frame(vec![Event::PointerMoved(from + (to - from) * (step as f32 / 4.0))]);
    }
    rig.fixture.frame(vec![button(to, false)]);
    rig.fixture.idle(2);
    let edits = rig.take_edits();
    let menus: Vec<_> = edits.iter().filter_map(|edit| if let GridEdit::FillMenu { range, .. } = edit { Some(range) } else { None }).collect();
    assert_eq!(menus.len(), 1, "{edits:?}");
    assert_eq!(menus[0].rows().end() - menus[0].rows().start(), 3, "the range grew two rows below the block");
    assert!(!edits.iter().any(|edit| matches!(edit, GridEdit::Fill { range: EditEvent::Commit { .. } })), "{edits:?}");
}

#[trial]
fn the_selection_border_drags_cells() {
    let mut rig = rig(None, selected_block(), 1.0);
    let border = rig.part(PartKey::new("range_border"), 0);
    rig.fixture.drag(&border, Vec2::X * COLUMN);
    let moves: Vec<_> = rig.take_edits().into_iter().filter_map(|edit| if let GridEdit::MoveCells { to } = edit { Some(to) } else { None }).collect();
    assert!(matches!(moves.last(), Some(EditEvent::Commit { after, .. }) if after.column == 2), "the block moves one column right");
}

#[trial]
fn a_click_on_the_selection_border_selects_without_moving() {
    let mut rig = rig(None, selected_block(), 1.0);
    let border = rig.part(PartKey::new("range_border"), 0);
    rig.fixture.click(&border);
    assert!(!rig.take_edits().iter().any(|edit| matches!(edit, GridEdit::MoveCells { .. })));
    assert!(rig.active().is_some());
}

#[trial]
fn copy_mode_takes_escape_and_enter() {
    let mut rig = rig(None, a1(), 1.0);
    rig.marks.borrow_mut().push(CellRange::cell(CellAt::new(0, 0)));
    rig.fixture.idle(2);
    rig.press(Key::Escape, Modifiers::NONE);
    assert!(rig.take_edits().contains(&GridEdit::ClearCopyMark));
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(rig.take_edits().contains(&GridEdit::PasteAtActive));
}

#[trial]
fn page_keys_move_a_screen_and_cmd_changes_sheet() {
    let mut rig = rig(None, a1(), 1.0);
    rig.press(Key::PageDown, Modifiers::NONE);
    assert!(rig.active().is_some_and(|cell| cell.row > 1), "Page Down moves a screen of rows");
    rig.press(Key::PageDown, Modifiers::COMMAND);
    assert!(rig.take_edits().contains(&GridEdit::NextSheet));
    rig.press(Key::PageUp, Modifiers::COMMAND);
    assert!(rig.take_edits().contains(&GridEdit::PreviousSheet));
}

#[trial]
fn paint_cell_is_told_the_zoom() {
    let rig = rig(None, a1(), 2.0);
    assert_eq!(rig.zoom.get(), 2.0);
}

#[trial]
fn outline_levels_reserve_the_bands_inside_the_grid() {
    use craft_table::ui::{OutlineAxis, OutlineGroup};
    let seen = Rc::new(RefCell::new(None));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let model = Sheet::new(None, Rc::new(Cell::new(0.0)));
        let mut state = GridState::default();
        let rows = OutlineAxis { levels: 2, groups: vec![OutlineGroup { start: 1, end: 2, level: 1, collapsed: false, summary_before: false }] };
        let columns = OutlineAxis::default();
        let plain = CellGrid::new("Plain", &model, &mut state).show_sheet(ui).geometry;
        let outlined = CellGrid::new("Outlined", &model, &mut state).outline(&rows, &columns).show_sheet(ui).geometry;
        *shown.borrow_mut() = Some((plain.bands(), outlined.bands()));
    });
    fixture.settle();
    let (plain, outlined) = seen.borrow().expect("both grids showed");
    assert_eq!(plain, Vec2::ZERO);
    assert!(outlined.x > 0.0, "two outline levels reserve a band before the row headers");
}

#[trial]
fn a_filter_button_over_the_header_takes_the_press() {
    let hits = Rc::new(RefCell::new(Vec::new()));
    let shown = hits.clone();
    let mut fixture = Fixture::new(move |ui| {
        let model = Sheet::new(None, Rc::new(Cell::new(0.0)));
        let mut state = GridState::default();
        let filters = [ColumnFilter::new(1)];
        let out = CellGrid::new("Sheet", &model, &mut state).filters(&filters).show_sheet(ui);
        if let Some((column, _)) = out.filter {
            shown.borrow_mut().push(column);
        }
    });
    fixture.settle();
    let button = fixture.by_label("B").expect("the filter button is named after its column");
    fixture.click(&button);
    assert_eq!(*hits.borrow(), vec![1], "the press opened the filter and did not select the header");
}

#[trial]
fn an_error_tag_beside_a_cell_takes_the_press() {
    let fixes = Rc::new(vec![Label::from("Convert to number")]);
    let mut fixture = Fixture::new({
        let fixes = fixes.clone();
        move |ui| {
            let model = Sheet::new(None, Rc::new(Cell::new(0.0)));
            let mut state = GridState::default();
            CellGrid::new("Sheet", &model, &mut state).error_tag(CellProblem::new(CellAt::new(2, 2), "Number stored as text", &fixes)).show_sheet(ui);
        }
    });
    fixture.settle();
    let tag = fixture.by_label("Number stored as text").expect("the tag is named by its problem");
    let cell = tag.rect;
    fixture.click(&tag);
    assert!(cell.height() > 0.0, "the tag takes room beside the cell");
}

#[trial]
fn the_error_tag_sits_left_or_right_of_the_cell() {
    let rects = Rc::new(RefCell::new(Vec::new()));
    let fixes = Rc::new(vec![Label::from("Ignore")]);
    let (shown, shown_fixes) = (rects.clone(), fixes.clone());
    let mut fixture = Fixture::new(move |ui| {
        let model = Sheet::new(None, Rc::new(Cell::new(0.0)));
        let mut state = GridState::default();
        let out = CellGrid::new("Sheet", &model, &mut state).error_tag(CellProblem::new(CellAt::new(2, 2), "Problem", &shown_fixes)).show_sheet(ui);
        shown.borrow_mut().push(out.geometry.mirror_rect(out.geometry.cell_rect(CellAt::new(2, 2))));
    });
    fixture.settle();
    let tag = fixture.by_label("Problem").expect("the tag has a node").rect;
    let cell = *rects.borrow().last().expect("the grid showed");
    assert!(tag.right() <= cell.left() || tag.left() >= cell.right(), "the tag sits beside the cell, not over it");
}

#[trial]
fn point_mode_inserts_an_absolute_reference() {
    let mut rig = rig(None, CellSelection::cell(CellAt::new(5, 5)), 1.0);
    rig.fixture.type_text("=");
    rig.fixture.idle(2);
    rig.press(Key::ArrowUp, Modifiers::NONE);
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(rig.take_edits().iter().any(|edit| matches!(edit, GridEdit::BeginEdit { .. })));
    let written = rig.committed.borrow().last().cloned();
    assert_eq!(written, Some(CellEdit::new(CellAt::new(5, 5), "=$F$5")), "the arrow pointed at the cell above and Enter committed the formula");
}
