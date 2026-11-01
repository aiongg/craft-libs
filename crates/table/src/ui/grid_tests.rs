//! The cell grid (egui_kittest).

use std::collections::BTreeMap;

use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, events, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Key, KeyboardShortcut, Modifiers, Painter, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;

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
}

impl Default for Sheet {
    fn default() -> Self {
        let none = BTreeMap::new();
        Self { rows: LineIndex::new(&none, 20.0, SHEET_ROWS), columns: LineIndex::new(&none, 80.0, SHEET_COLUMNS), frozen: (0, 0) }
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
    fn last_used(&self) -> CellRef {
        CellRef::new(9, 9)
    }
    fn paint_cell(&self, _painter: &Painter, _rect: Rect, _c: CellRef) {}
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
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = CellGrid::new("grid", &app.sheet, &mut app.state).right_to_left(app.right_to_left).show(ui);
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
}
