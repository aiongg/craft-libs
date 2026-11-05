//! The cell grid (egui_kittest).

use std::cell::Cell;
use std::collections::BTreeMap;

use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, events, symbols};
use craft_ui::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{Event, ImeEvent, Key, KeyboardShortcut, Modifiers, Painter, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::grid::{
    CellGrid, CellRange, CellRef, GridEdit, GridLayout, GridModel, GridState, LineIndex, SHEET_COLUMNS, SHEET_ROWS, SheetGeometry, column_letters,
};
use super::style::SheetStyle;

fn style() -> SheetStyle {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(SheetStyle::of(&Tokens::of(ui))));
    style.unwrap()
}

/// 20-point rows and 80-point columns, the sheet's full size.
struct Sheet {
    rows: LineIndex,
    columns: LineIndex,
    frozen: (u32, u32),
    merges: Vec<CellRange>,
    /// The zoom `paint_cell` was last told.
    painted_at: Cell<f32>,
}

impl Default for Sheet {
    fn default() -> Self {
        let none = BTreeMap::new();
        Self {
            rows: LineIndex::new(&none, 20.0, SHEET_ROWS),
            columns: LineIndex::new(&none, 80.0, SHEET_COLUMNS),
            frozen: (0, 0),
            merges: Vec::new(),
            painted_at: Cell::new(0.0),
        }
    }
}

impl GridModel for Sheet {
    fn rows(&self) -> &LineIndex {
        &self.rows
    }
    fn columns(&self) -> &LineIndex {
        &self.columns
    }
    fn frozen(&self) -> (u32, u32) {
        self.frozen
    }
    fn merges(&self) -> &[CellRange] {
        &self.merges
    }
    fn last_used(&self) -> CellRef {
        CellRef::new(9, 9)
    }
    fn paint_cell(&self, _painter: &Painter, _rect: Rect, _c: CellRef, zoom: f32) {
        self.painted_at.set(zoom);
    }
    fn cell_text(&self, c: CellRef) -> String {
        format!("{}{}", column_letters(c.column), c.row + 1)
    }
}

#[derive(Default)]
struct App {
    sheet: Sheet,
    state: GridState,
    edits: Vec<GridEdit>,
    right_to_left: bool,
    geometry: Option<SheetGeometry>,
    copy_mark: Vec<CellRange>,
    outline: (u8, u8),
    no_headings: bool,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = CellGrid::new("grid", &app.sheet, &mut app.state)
        .right_to_left(app.right_to_left)
        .copy_mark(&app.copy_mark)
        .outline_levels(app.outline.0, app.outline.1)
        .headings(!app.no_headings)
        .show(ui);
    app.edits.extend(out.edits);
    app.geometry = Some(out.geometry);
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([800.0, 500.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn center(harness: &Harness<'_, App>, cell: CellRef) -> Pos2 {
    let geometry = harness.state().geometry.as_ref().unwrap();
    geometry.mirror_rect(geometry.cell_rect(cell)).center()
}

fn click(harness: &mut Harness<'_, App>, cell: CellRef, modifiers: Modifiers) {
    let at = center(harness, cell);
    harness.click_at_with(at, PointerButton::Primary, modifiers);
}

fn press(harness: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    harness.press(KeyboardShortcut::new(modifiers, key));
}

fn active(harness: &Harness<'_, App>) -> CellRef {
    harness.state().state.selection.active()
}

// column_letters of 0, 25, 26, 16383 is A, Z, AA, XFD.
#[test]
fn column_letters_run_a_to_xfd() {
    let letters: Vec<_> = [0, 25, 26, 51, 52, 701, 702, 16383].into_iter().map(column_letters).collect();
    assert_eq!(letters, ["A", "Z", "AA", "AZ", "BA", "ZZ", "AAA", "XFD"]);
}

// LineIndex start and at with custom and hidden lines.
#[test]
fn line_index_finds_starts_and_lines() {
    let custom = BTreeMap::from([(2, 40.0), (3, 0.0)]);
    let lines = LineIndex::new(&custom, 20.0, 100);
    let starts: Vec<_> = (0..6).map(|line| lines.start(line)).collect();
    assert_eq!(starts, [0.0, 20.0, 40.0, 80.0, 80.0, 100.0]);
    assert_eq!((lines.size(2), lines.size(3), lines.size(7)), (40.0, 0.0, 20.0));
    assert_eq!([-5.0, 0.0, 19.9, 40.0, 79.9, 80.0, 99.9, 100.0, 1.0e9].map(|pos| lines.at(pos)), [0, 0, 0, 2, 2, 4, 4, 5, 99]);
    assert_eq!(lines.count(), 100);
}

fn layout(frozen: (u32, u32), right_to_left: bool) -> GridLayout {
    let none = BTreeMap::new();
    GridLayout {
        rect: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
        zoom: 1.5,
        rows: LineIndex::new(&none, 20.0, SHEET_ROWS),
        columns: LineIndex::new(&none, 80.0, SHEET_COLUMNS),
        frozen,
        scroll: vec2(100.0, 40.0),
        headings: true,
        bands: egui::Vec2::ZERO,
        right_to_left,
    }
}

// x, y and cell_at with frozen rows and columns, scrolled, at 150 %.
#[test]
fn geometry_maps_cells_with_frozen_panes_and_zoom() {
    let style = style();
    let geometry = SheetGeometry::new(layout((1, 1), false), &style);
    let cells = geometry.cells();
    assert_eq!(geometry.header_size().y, (style.column_header_height * 1.5).round());
    assert_eq!(geometry.x(0), cells.left());
    assert_eq!(geometry.x(1), cells.left() + 120.0 - 150.0, "the first scrolled column starts 100 points scrolled under the frozen one");
    assert_eq!(geometry.y(1), cells.top() + 30.0 - 60.0);
    assert_eq!(geometry.cell_rect(CellRef::new(3, 4)).size(), vec2(120.0, 30.0));
    for cell in [CellRef::new(0, 0), CellRef::new(0, 6), CellRef::new(5, 0), CellRef::new(4, 7), CellRef::new(30, 12)] {
        assert_eq!(geometry.cell_at(geometry.cell_rect(cell).center()), cell);
    }
    assert_eq!(geometry.visible().0.0, 3, "row 3 starts the scrolled rows: 20 + 40 points");
    assert_eq!(geometry.visible().1.0, 2);
    let range = |a: (u32, u32), b: (u32, u32)| CellRange::new(CellRef::new(a.0, a.1), CellRef::new(b.0, b.1));
    assert_eq!(geometry.range_panes(range((0, 0), (5, 5))).len(), 4);
    assert_eq!(geometry.range_panes(range((2, 2), (5, 5))).len(), 1);
    assert_eq!(geometry.range_panes(range((0, 2), (5, 5))).len(), 2);
    let flat = SheetGeometry::new(layout((0, 0), false), &style);
    assert_eq!(flat.range_panes(range((0, 0), (5, 5))).len(), 1);
    assert_eq!(flat.range_rect(range((0, 0), (1, 1))).size(), vec2(240.0, 60.0));
}

// mirror_x is an involution and cell_at reads mirrored input.
#[test]
fn right_to_left_mirrors_the_screen() {
    let style = style();
    let rtl = SheetGeometry::new(layout((1, 1), true), &style);
    let ltr = SheetGeometry::new(layout((1, 1), false), &style);
    for x in [0.0, 13.5, 400.0, 799.0] {
        assert_eq!(rtl.mirror_x(rtl.mirror_x(x)), x);
        assert_eq!(ltr.mirror_x(x), x);
    }
    assert_eq!(rtl.mirror_x(0.0), 800.0);
    let screen = pos2(700.0, 200.0);
    assert_eq!(rtl.cell_at(rtl.mirror_pos(screen)), ltr.cell_at(pos2(100.0, 200.0)));

    let mut harness = harness(App { right_to_left: true, ..App::default() });
    click(&mut harness, CellRef::new(2, 3), Modifiers::NONE);
    assert_eq!(active(&harness), CellRef::new(2, 3), "a click on the mirrored cell selects it");
    let geometry = harness.state().geometry.as_ref().unwrap();
    assert!(geometry.mirror_rect(geometry.cell_rect(CellRef::new(0, 0))).center().x > 400.0, "column A is at the right");
}

// extend_to, add and advance keep Excel's active cell.
#[test]
fn selection_extends_adds_and_advances() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    click(&mut harness, CellRef::new(3, 3), Modifiers::SHIFT);
    let selection = &harness.state().state.selection;
    assert_eq!(selection.ranges(), [CellRange::new(CellRef::new(1, 1), CellRef::new(3, 3))]);
    assert_eq!(selection.active(), CellRef::new(1, 1), "Shift+click keeps the active cell");

    click(&mut harness, CellRef::new(5, 5), Modifiers::COMMAND);
    let selection = &harness.state().state.selection;
    assert_eq!((selection.ranges().len(), selection.active()), (2, CellRef::new(5, 5)));

    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Tab);
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(active(&harness), CellRef::new(1, 3));
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(active(&harness), CellRef::new(2, 1), "Enter after a run of Tabs returns to the column the run started in");
}

// keys move and extend the selection; Enter and Tab advance inside it.
#[test]
fn arrows_tab_and_enter_move_the_active_cell() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(active(&harness), CellRef::new(3, 3));
    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    let selection = &harness.state().state.selection;
    assert_eq!(selection.ranges(), [CellRange::new(CellRef::new(3, 3), CellRef::new(4, 4))]);
    assert_eq!(selection.active(), CellRef::new(3, 3));
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(active(&harness), CellRef::new(4, 3), "Enter goes down inside the selection");
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(active(&harness), CellRef::new(4, 4));
    press(&mut harness, Modifiers::SHIFT, Key::Tab);
    assert_eq!(active(&harness), CellRef::new(4, 3));
    assert_eq!(harness.state().state.selection.ranges().len(), 1, "the selection stays");

    press(&mut harness, Modifiers::COMMAND, Key::ArrowDown);
    assert_eq!(active(&harness), CellRef::new(SHEET_ROWS - 1, 3), "Cmd+arrow goes to the data edge, here the sheet's");
    assert!(harness.state().state.scroll.y > 0.0, "and scrolls it into view");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    assert_eq!((active(&harness), harness.state().state.scroll.y), (CellRef::new(0, 0), 0.0));
    press(&mut harness, Modifiers::COMMAND, Key::End);
    assert_eq!(active(&harness), CellRef::new(9, 9));
}

// a typed letter reports BeginEdit with it, F2 without, Delete Clear, Backspace ClearAndEdit.
#[test]
fn typing_starts_an_edit_and_delete_clears() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    harness.type_text("x");
    press(&mut harness, Modifiers::NONE, Key::F2);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(
        harness.state().edits,
        [GridEdit::BeginEdit { typed: Some("x".into()) }, GridEdit::BeginEdit { typed: None }, GridEdit::Clear, GridEdit::ClearAndEdit]
    );

    harness.state_mut().edits.clear();
    harness.state_mut().state.editing = true;
    harness.run();
    harness.type_text("y");
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert!(harness.state().edits.is_empty(), "the grid leaves typing alone while the app's editor is open");
}

// dragging a column edge reports ResizeColumn, a double-click AutoFitColumn.
#[test]
fn header_edges_resize_and_double_click_autofits() {
    let mut harness = harness(App::default());
    let edge = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        pos2(geometry.x(1) + 80.0, geometry.cells().top() - geometry.header_size().y / 2.0)
    };
    harness.drag(edge, edge + vec2(40.0, 0.0), Modifiers::NONE);
    let resizes: Vec<_> = harness
        .state()
        .edits
        .iter()
        .filter_map(|edit| match edit {
            GridEdit::ResizeColumn { column, width } => Some((*column, width.clone())),
            _ => None,
        })
        .collect();
    assert!(resizes.iter().all(|(column, _)| *column == 1));
    assert!(matches!(resizes.first(), Some((_, EditEvent::Begin { .. }))));
    assert!(matches!(resizes.last(), Some((_, EditEvent::Commit { value, .. })) if (*value - 120.0).abs() < 0.01), "{resizes:?}");

    harness.state_mut().edits.clear();
    for frame in events::click(edge, PointerButton::Primary, 2, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.run_steps(1);
    }
    harness.run_steps(3);
    assert!(harness.state().edits.contains(&GridEdit::AutoFitColumn { column: 1 }), "{:?}", harness.state().edits);
    let (begins, ends) = begins_and_ends(&harness.state().edits);
    assert_eq!(begins, ends, "each click of the double-click ends the gesture it began: {:?}", harness.state().edits);
}

/// How many resize and fill gestures the edits begin, and how many end (`Commit`, `Cancel`).
fn begins_and_ends(edits: &[GridEdit]) -> (usize, usize) {
    let mut count = (0, 0);
    for edit in edits {
        let event = match edit {
            GridEdit::ResizeColumn { width, .. } | GridEdit::ResizeRow { height: width, .. } => width.clone(),
            _ => continue,
        };
        match event {
            EditEvent::Begin { .. } => count.0 += 1,
            EditEvent::Commit { .. } | EditEvent::Cancel { .. } => count.1 += 1,
            _ => {}
        }
    }
    count
}

fn send(harness: &mut Harness<'_, App>, events: impl IntoIterator<Item = Event>) {
    for event in events {
        harness.event(event);
    }
    harness.run_steps(1);
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

// A press and release in one pass (a trackpad tap) acts in that pass and leaves no `Begin` open.
// The harness plays a press and a release in successive passes, so this runs the context with both
// events in one raw input.
#[test]
fn a_tap_in_one_pass_acts_and_leaves_no_gesture_open() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    symbols::install(&ctx);
    let mut app = App::default();
    let pass = |events: Vec<Event>, app: &mut App| {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 500.0))), events, ..egui::RawInput::default() };
        ctx.run_ui(input, |ui| show(ui, app)).drop_without_applying_deltas();
    };
    pass(Vec::new(), &mut app);
    let edge = {
        let geometry = app.geometry.as_ref().unwrap();
        pos2(geometry.x(1) + 80.0, geometry.cells().top() - geometry.header_size().y / 2.0)
    };
    pass(vec![Event::PointerMoved(edge)], &mut app);
    pass(vec![button(edge, true), button(edge, false)], &mut app);
    pass(Vec::new(), &mut app);
    pass(Vec::new(), &mut app);
    let (begins, ends) = begins_and_ends(&app.edits);
    assert_eq!(begins, ends, "a bare tap leaves no gesture open: {:?}", app.edits);

    // egui tells the button down on a widget only until the release: a tap on a cell, press and
    // release in one pass, still selects it.
    let cell = {
        let geometry = app.geometry.as_ref().unwrap();
        geometry.cell_rect(CellRef::new(3, 3)).center()
    };
    pass(vec![Event::PointerMoved(cell)], &mut app);
    pass(vec![button(cell, true), button(cell, false)], &mut app);
    assert_eq!(app.state.selection.active(), CellRef::new(3, 3));
}

// Esc during a resize cancels the gesture the press began, reported for its column.
#[test]
fn escape_cancels_a_header_resize() {
    let mut harness = harness(App::default());
    let edge = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        pos2(geometry.x(1) + 80.0, geometry.cells().top() - geometry.header_size().y / 2.0)
    };
    send(&mut harness, [Event::PointerMoved(edge)]);
    send(&mut harness, [button(edge, true)]);
    send(&mut harness, [Event::PointerMoved(edge + vec2(30.0, 0.0))]);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    harness.run_steps(2);
    let edits = &harness.state().edits;
    assert!(edits.iter().any(|e| matches!(e, GridEdit::ResizeColumn { column: 1, width: EditEvent::Cancel { .. } })), "{edits:?}");
}

// An input method's first composition reports BeginEdit through the grid's composer.
#[test]
fn an_ime_composition_begins_an_edit() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    harness.state_mut().edits.clear();
    send(&mut harness, [Event::Ime(ImeEvent::Preedit { text: "k".into(), active_range_chars: None })]);
    assert_eq!(harness.state().edits, [GridEdit::BeginEdit { typed: Some(String::new()) }]);
}

// Double-clicking the edge where a hidden column sits asks to unhide it.
#[test]
fn a_double_click_on_a_hidden_edge_unhides() {
    let hidden = BTreeMap::from([(2, 0.0)]);
    let sheet = Sheet { columns: LineIndex::new(&hidden, 80.0, SHEET_COLUMNS), ..Sheet::default() };
    let mut harness = harness(App { sheet, ..App::default() });
    let edge = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        pos2(geometry.x(3) + 2.0, geometry.cells().top() - geometry.header_size().y / 2.0)
    };
    for frame in events::click(edge, PointerButton::Primary, 2, Modifiers::NONE) {
        send(&mut harness, frame);
    }
    harness.run_steps(3);
    assert!(harness.state().edits.contains(&GridEdit::UnhideColumn { column: 2 }), "{:?}", harness.state().edits);
}

fn drag_events(harness: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    for frame in events::drag(from, to, Modifiers::NONE) {
        send(harness, frame);
    }
    harness.run_steps(2);
}

fn handle_of(harness: &Harness<'_, App>) -> Pos2 {
    let geometry = harness.state().geometry.as_ref().unwrap();
    let range = harness.state().state.selection.current();
    geometry.mirror_pos(geometry.range_rect(range).right_bottom())
}

// Dragging the fill handle down reports the grown range as one gesture and selects it.
#[test]
fn the_fill_handle_drags_a_range() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    let (from, to) = (handle_of(&harness), center(&harness, CellRef::new(4, 1)));
    harness.state_mut().edits.clear();
    drag_events(&mut harness, from, to);
    let fills: Vec<_> =
        harness.state().edits.iter().filter_map(|e| if let GridEdit::Fill { range } = e { Some(range.clone()) } else { None }).collect();
    let target = CellRange::new(CellRef::new(1, 1), CellRef::new(4, 1));
    assert!(matches!(fills.first(), Some(EditEvent::Begin { .. })), "{fills:?}");
    assert!(matches!(fills.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == target), "{fills:?}");
    assert_eq!(harness.state().state.selection.ranges(), [target]);
}

// Dragging the selection's border reports where its top-left corner goes and selects it there.
#[test]
fn the_selection_border_drags_cells() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    let from = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        let rect = geometry.range_rect(CellRange::cell(CellRef::new(1, 1)));
        pos2(rect.center().x, rect.top())
    };
    let to = from + vec2(160.0, 40.0);
    harness.state_mut().edits.clear();
    drag_events(&mut harness, from, to);
    let moves: Vec<_> =
        harness.state().edits.iter().filter_map(|e| if let GridEdit::MoveCells { to } = e { Some(to.clone()) } else { None }).collect();
    assert!(matches!(moves.last(), Some(EditEvent::Commit { value, .. }) if *value == CellRef::new(3, 3)), "{moves:?}");
    assert_eq!(active(&harness), CellRef::new(3, 3));
}

// A press and release on the border without moving is a click: it selects the cell.
#[test]
fn a_click_on_the_selection_border_selects_without_moving() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(1, 1), Modifiers::NONE);
    click(&mut harness, CellRef::new(5, 5), Modifiers::COMMAND);
    harness.state_mut().edits.clear();
    let rect = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        geometry.range_rect(CellRange::cell(CellRef::new(5, 5)))
    };
    harness.click_at_with(pos2(rect.center().x, rect.top()), PointerButton::Primary, Modifiers::NONE);
    assert!(!harness.state().edits.iter().any(|e| matches!(e, GridEdit::MoveCells { .. })), "{:?}", harness.state().edits);
    assert_eq!(harness.state().state.selection.ranges().len(), 1);
}

// While a copy mark shows, Esc ends copy mode and Enter pastes; neither moves the active cell.
#[test]
fn copy_mode_takes_escape_and_enter() {
    let mark = vec![CellRange::new(CellRef::new(0, 0), CellRef::new(1, 1))];
    let mut harness = harness(App { copy_mark: mark, ..App::default() });
    click(&mut harness, CellRef::new(3, 3), Modifiers::NONE);
    harness.state_mut().edits.clear();
    press(&mut harness, Modifiers::NONE, Key::Escape);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().edits, [GridEdit::ClearCopyMark, GridEdit::PasteAtActive]);
    assert_eq!(active(&harness), CellRef::new(3, 3));
    harness.state_mut().copy_mark.clear();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(active(&harness), CellRef::new(4, 3), "without a mark Enter moves down");
}

// Page Down and Up move a screen of rows and Cmd+Page Down the next sheet.
#[test]
fn page_keys_move_a_screen_and_cmd_changes_sheet() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    let down = active(&harness);
    assert!(down.row > 10 && down.column == 2, "{down:?}");
    press(&mut harness, Modifiers::NONE, Key::PageUp);
    assert!(active(&harness).row < down.row - 10 && active(&harness).column == 2, "{:?}", active(&harness));
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    press(&mut harness, Modifiers::SHIFT, Key::PageDown);
    assert_eq!(harness.state().state.selection.active(), CellRef::new(2, 2));
    assert!(harness.state().state.selection.current().end.row > 10, "Shift+Page Down extends");
    harness.state_mut().edits.clear();
    press(&mut harness, Modifiers::COMMAND, Key::PageDown);
    assert_eq!(harness.state().edits, [GridEdit::NextSheet]);
}

// A merged cell is one stop for the arrow keys.
#[test]
fn arrows_step_over_a_merged_cell() {
    let merge = CellRange::new(CellRef::new(1, 1), CellRef::new(1, 3));
    let mut harness = harness(App::default());
    harness.state_mut().sheet.merges = vec![merge];
    harness.run();
    click(&mut harness, CellRef::new(1, 0), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(active(&harness), CellRef::new(1, 1));
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(active(&harness), CellRef::new(1, 4), "the arrow leaves the merge");
}

// Right to left, the Left arrow goes to the next column.
#[test]
fn arrows_mirror_in_a_right_to_left_sheet() {
    let mut harness = harness(App { right_to_left: true, ..App::default() });
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(active(&harness), CellRef::new(2, 3));
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(active(&harness), CellRef::new(2, 2));
}

// The Menu key and Shift+F10 ask for the menu at the active cell, a secondary click at the pointer.
#[test]
fn context_menu_keys_and_clicks_report_anchor_and_pointer() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    harness.state_mut().edits.clear();
    press(&mut harness, Modifiers::NONE, Key::Menu);
    press(&mut harness, Modifiers::SHIFT, Key::F10);
    let cell = {
        let geometry = harness.state().geometry.as_ref().unwrap();
        geometry.mirror_rect(geometry.cell_rect(CellRef::new(2, 2)))
    };
    let menus = |edits: &[GridEdit]| -> Vec<(Rect, Option<Pos2>)> {
        edits.iter().filter_map(|e| if let GridEdit::ContextMenu { anchor, pointer, .. } = e { Some((*anchor, *pointer)) } else { None }).collect()
    };
    assert_eq!(menus(&harness.state().edits), [(cell, None), (cell, None)]);

    harness.state_mut().edits.clear();
    let at = center(&harness, CellRef::new(4, 4));
    harness.click_at_with(at, PointerButton::Secondary, Modifiers::NONE);
    let target = harness.state().geometry.as_ref().unwrap().cell_rect(CellRef::new(4, 4));
    assert_eq!(menus(&harness.state().edits), [(target, Some(at))]);
}

// The scroll stops at the sheet's extent: the last 40 rows and 20 columns still fill the window.
#[test]
fn scrolling_stops_at_the_sheets_extent() {
    let mut harness = harness(App::default());
    harness.state_mut().state.scroll = vec2(1.0e12, 1.0e12);
    harness.run();
    let scroll = harness.state().state.scroll;
    assert_eq!(scroll, vec2(f64::from(SHEET_COLUMNS - 20) as f32 * 80.0, f64::from(SHEET_ROWS - 40) as f32 * 20.0));
}

// `paint_cell` is told the zoom the grid draws at.
#[test]
fn paint_cell_is_told_the_zoom() {
    let harness = harness(App { state: GridState { zoom: 2.0, ..GridState::default() }, ..App::default() });
    assert_eq!(harness.state().sheet.painted_at.get(), 2.0);
}

// The outline bands sit inside the grid's rect: the cells start after them and the response's
// geometry carries them; without headings the level buttons get a corner.
#[test]
fn outline_levels_reserve_the_bands_inside_the_grid() {
    let style = style();
    let plain = harness(App::default());
    let plain = plain.state().geometry.as_ref().unwrap().clone();
    let rows_only = harness(App { outline: (2, 0), ..App::default() });
    let geometry = rows_only.state().geometry.as_ref().unwrap();
    let band = super::outline::OutlineBand::thickness(2, 1.0, &style);
    assert_eq!(geometry.bands(), vec2(band, 0.0));
    assert_eq!(geometry.cells().left() - plain.cells().left(), band);
    assert_eq!(geometry.cells().right(), plain.cells().right(), "the grid's rect is the same");

    let corner = harness(App { outline: (2, 0), no_headings: true, ..App::default() });
    let bands = corner.state().geometry.as_ref().unwrap().bands();
    assert_eq!(bands, vec2(band, super::outline::lane(style.outline_lane, 1.0) + super::outline::BAND_SLACK));
}

// After a key scrolls the active cell into view, the grid's node names it as the active
// descendant in the same pass.
#[test]
fn the_active_descendant_follows_a_key_in_the_same_pass() {
    let mut harness = harness(App::default());
    click(&mut harness, CellRef::new(2, 2), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    let cell = active(&harness);
    assert!(cell.row > 10);
    let label = format!("{}{}", column_letters(cell.column), cell.row + 1);
    let named = harness.get_by_label(&label).accesskit_node().id();
    let grid = harness.get_by_role(Role::Grid);
    assert_eq!(grid.accesskit_node().active_descendant().map(|node| node.id()), Some(named));
}

/// The grid drawn to pixels, with its geometry.
fn render(app: App) -> Result<(egui::ColorImage, SheetGeometry), craft_ui::testing::render::RenderError> {
    use craft_ui::testing::render::{Render, image_of};
    let mut harness = Render::new([800.0, 500.0])
        .setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .harness(show, app)?;
    harness.run();
    let image = image_of(&mut harness)?;
    let geometry = harness.state().geometry.clone().expect("a pass ran");
    Ok((image, geometry))
}

fn pixel(image: &egui::ColorImage, at: Pos2) -> egui::Color32 {
    image.pixels[at.y as usize * image.size[0] + at.x as usize]
}

/// How many pixels of `area` are not `colour`.
fn not_colored(image: &egui::ColorImage, area: Rect, colour: egui::Color32) -> usize {
    let mut count = 0;
    for y in area.top().floor() as usize..area.bottom().ceil() as usize {
        for x in area.left().floor() as usize..area.right().ceil() as usize {
            count += usize::from(pixel(image, pos2(x as f32, y as f32)) != colour);
        }
    }
    count
}

// A merged cell has no gridline inside it.
#[test]
fn no_gridline_shows_inside_a_merged_cell() -> Result<(), craft_ui::testing::render::RenderError> {
    let merge = CellRange::new(CellRef::new(2, 1), CellRef::new(2, 3));
    let (plain, geometry) = render(App::default())?;
    let (merged, _) = render(App { sheet: Sheet { merges: vec![merge], ..Sheet::default() }, ..App::default() })?;
    let background = pixel(&plain, geometry.cell_rect(CellRef::new(5, 5)).center());
    let (inner, row) = (geometry.x(2), geometry.cell_rect(CellRef::new(2, 2)));
    let across = Rect::from_min_max(pos2(inner - 0.5, row.top() + 3.0), pos2(inner + 0.5, row.bottom() - 3.0));
    assert!(not_colored(&plain, across, background) > 0, "the gridline is there without the merge");
    assert_eq!(not_colored(&merged, across, background), 0, "and gone inside the merge");
    Ok(())
}

// A hidden column leaves a double divider on its neighbours' header edge; a visible edge has none.
#[test]
fn a_hidden_column_leaves_a_double_divider_in_the_header() -> Result<(), craft_ui::testing::render::RenderError> {
    let style = style();
    let columns = BTreeMap::from([(2, 0.0)]);
    let sheet = Sheet { columns: LineIndex::new(&columns, 80.0, SHEET_COLUMNS), ..Sheet::default() };
    let (image, geometry) = render(App { sheet, ..App::default() })?;
    let header = Rect::from_min_max(pos2(0.0, geometry.cells().top() - geometry.header_size().y), pos2(0.0, geometry.cells().top()));
    let chrome = pixel(&image, pos2(geometry.cells().left() - 2.0, header.top() + 1.0));
    let beside = |x: f32| {
        let offset = style.hidden_mark_offset;
        Rect::from_min_max(pos2(x - offset - 1.0, header.top() + style.header_separator_inset + 3.0), pos2(x - offset + 1.0, header.bottom() - 4.0))
    };
    let hidden_edge = geometry.x(3);
    let plain_edge = geometry.x(5);
    assert!(not_colored(&image, beside(hidden_edge), chrome) > 0, "marks beside the hidden column");
    assert_eq!(not_colored(&image, beside(plain_edge), chrome), 0, "none at a plain edge");
    Ok(())
}

// A selected header gets the accent bar on its inner edge.
#[test]
fn a_selected_header_has_an_accent_bar() -> Result<(), craft_ui::testing::render::RenderError> {
    let (image, geometry) = render(App::default())?;
    let tokens = {
        let mut tokens = None;
        egui::__run_test_ui(|ui| tokens = Some(Tokens::of(ui)));
        tokens.unwrap()
    };
    let cell = geometry.cell_rect(CellRef::new(0, 0));
    let bar = pos2(cell.center().x, geometry.cells().top() - 1.5);
    let above = pos2(cell.center().x, geometry.cells().top() - 6.0);
    let accent = {
        let c = tokens.color.accent.solid.to_egui();
        c
    };
    let near = |a: egui::Color32, b: egui::Color32| a.to_array().iter().zip(b.to_array()).all(|(x, y)| x.abs_diff(y) <= 2);
    assert!(near(pixel(&image, bar), accent), "bar {:?} vs {accent:?}", pixel(&image, bar));
    assert!(!near(pixel(&image, above), accent), "the bar is thin");
    Ok(())
}
