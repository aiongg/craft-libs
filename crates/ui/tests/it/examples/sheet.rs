//! The arrow split of 18.4 in `SheetGrid`: arrows are navigation with no cell open and in Enter mode, the caret's in
//! Edit mode, and the reference cursor's in formula point mode.

use craft_behavior::{CellSelection, RowAt, SelectionModel as _};
use craft_ui::contract::{Bind, Props as _, Role, ShowCollection as _};
use craft_ui::testing::examples::sheet::{CellEdit, Rows, SheetGrid, SheetRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// Enough rows that two steps down from the clicked middle cell stay inside the sheet.
const ROWS: usize = 12;
/// Far enough below the grid's middle to land at least two rows down.
const ROW_STEP: f32 = 80.0;
const COLUMNS: [&str; 4] = ["A", "B", "C", "D"];

struct Rig {
    fixture: Fixture,
    selection: Rc<RefCell<CellSelection<usize>>>,
    committed: Rc<RefCell<Vec<CellEdit>>>,
    rows: Rc<RefCell<Vec<Vec<String>>>>,
}

/// A sheet of `ROWS` rows of numbers with a lead cell set by a click on its middle.
fn rig() -> Rig {
    let cells: Vec<Vec<String>> = (0..ROWS).map(|row| (0..4).map(|column| format!("{}", row * 4 + column)).collect()).collect();
    let rows = Rc::new(RefCell::new(cells));
    let selection = Rc::new(RefCell::new(CellSelection::<usize>::default()));
    let committed = Rc::new(RefCell::new(Vec::new()));
    let (shown_rows, shown_selection, shown_committed) = (rows.clone(), selection.clone(), committed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let held = shown_rows.borrow().clone();
        let mut cells = shown_selection.borrow().clone();
        let out =
            SheetGrid::new(Bind::bound(&mut cells), &held)
                .label("Sheet")
                .id_salt("sheet")
                .show_items(ui, &Rows::new(held.len()), &mut |key, icx| SheetRow::new(&held[*key], icx.cells().to_vec()));
        *shown_selection.borrow_mut() = cells;
        if let Some(edit) = out.committed {
            shown_rows.borrow_mut()[edit.row][edit.column] = edit.text.clone();
            shown_committed.borrow_mut().push(edit);
        }
    });
    fixture.settle();
    let grid = fixture.by_role(Role::TABLE).first().copied().expect("the grid showed");
    fixture.click(&grid);
    fixture.idle(2);
    Rig { fixture, selection, committed, rows }
}

impl Rig {
    /// The row and column of the lead cell.
    fn lead(&self) -> (usize, usize) {
        let selection = self.selection.borrow();
        let active = selection.active().expect("a cell holds the lead");
        (active.row, active.column as usize)
    }

    fn press(&mut self, key: Key) {
        self.press_with(key, Modifiers::NONE);
    }

    fn press_with(&mut self, key: Key, mods: Modifiers) {
        self.fixture.key(key, mods);
        self.fixture.idle(2);
    }

    /// A click at `at` with Ctrl (Cmd on macOS) held.
    fn ctrl_click(&mut self, at: egui::Pos2) {
        self.fixture.frame(vec![Event::ModifiersChanged(Modifiers::COMMAND)]);
        self.fixture.frame(vec![craft_ui::testing::pointer::moved(at)]);
        self.fixture.frame(vec![craft_ui::testing::pointer::press(at)]);
        self.fixture.frame(vec![craft_ui::testing::pointer::release(at)]);
        self.fixture.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        self.fixture.idle(2);
    }

    fn type_text(&mut self, text: &str) {
        self.fixture.type_text(text);
        self.fixture.idle(2);
    }
}

#[trial]
fn arrows_navigate_with_no_cell_open_and_edit_mode_claims_the_horizontal_arrows() {
    {
        // With no cell open an arrow moves the lead cell; typing opens it, and an arrow then still navigates and commits
        // the draft into the cell it was typed in.
        let mut rig = rig();
        let (row, column) = rig.lead();
        rig.press(Key::ArrowDown);
        assert_eq!(rig.lead(), (row + 1, column), "an arrow moves the lead cell when no cell is open");
        assert!(rig.committed.borrow().is_empty());
        rig.type_text("x");
        assert!(rig.committed.borrow().is_empty(), "typing opens the cell and commits nothing");
        rig.press(Key::ArrowDown);
        assert_eq!(rig.lead(), (row + 2, column), "in Enter mode the arrows are still navigation");
        assert_eq!(*rig.committed.borrow(), [CellEdit::new(row + 1, column, "x")], "moving away commits the draft into the cell it was for");
        assert_eq!(rig.rows.borrow()[row + 1][column], "x");
    }
    {
        // F2 opens the cell with its text and claims Left and Right for the caret: the lead stays, the text changes where
        // the caret is, and Enter commits.
        let mut rig = rig();
        let (row, column) = rig.lead();
        let original = rig.rows.borrow()[row][column].clone();
        rig.press(Key::F2);
        rig.press(Key::ArrowLeft);
        assert_eq!(rig.lead(), (row, column), "Left moved the caret, not the lead cell");
        rig.type_text("z");
        rig.press(Key::Enter);
        let expected = format!("{}z{}", &original[..original.len() - 1], &original[original.len() - 1..]);
        assert_eq!(*rig.committed.borrow(), [CellEdit::new(row, column, expected)]);
        assert_eq!(rig.lead(), (row + 1, column), "Enter commits and moves down");
    }
}

/// Enter after an entry moves down; Tab runs along the row and the next Enter returns to the column the run started
/// in; Shift runs them back. Inside a selection of several cells both cycle inside it.
#[trial]
fn enter_and_tab_move_the_active_cell_as_in_excel() {
    let mut rig = rig();
    // Start near the first column, so that the Tabs have room.
    rig.press(Key::ArrowLeft);
    rig.press(Key::ArrowLeft);
    let (row, column) = rig.lead();
    rig.type_text("x");
    rig.press(Key::Enter);
    assert_eq!(rig.lead(), (row + 1, column), "Enter after an entry moves down");
    assert_eq!(*rig.committed.borrow(), [CellEdit::new(row, column, "x")]);
    rig.press(Key::Tab);
    assert_eq!(rig.lead(), (row + 1, column + 1), "Tab moves along the row");
    rig.press(Key::Enter);
    assert_eq!(rig.lead(), (row + 2, column), "Enter after a run of Tabs returns to the column it started in");
    rig.press_with(Key::Enter, Modifiers::SHIFT);
    assert_eq!(rig.lead(), (row + 1, column), "Shift+Enter moves up");

    rig.press_with(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(rig.selection.borrow().ranges().len(), 1);
    rig.press(Key::Tab);
    assert_eq!(rig.lead(), (row + 1, column + 1), "inside a selection Tab moves the active cell within it");
    rig.press(Key::Tab);
    assert_eq!(rig.lead(), (row + 1, column), "past the last cell it cycles back to the first");
    rig.press_with(Key::Tab, Modifiers::SHIFT);
    assert_eq!(rig.lead(), (row + 1, column + 1), "Shift+Tab runs the cycle back");
    assert_eq!(rig.selection.borrow().ranges().len(), 1, "the selection is kept");
}

/// A formula that ends in an operator claims the arrows for its reference cursor: an arrow writes the neighbouring
/// cell's name into the draft and the lead stays.
#[trial]
fn formula_point_mode_moves_a_reference_cursor_with_the_arrows() {
    let mut rig = rig();
    let (row, column) = rig.lead();
    rig.type_text("=");
    rig.press(Key::ArrowDown);
    assert_eq!(rig.lead(), (row, column), "the arrow moved the reference cursor, not the lead cell");
    rig.press(Key::Enter);
    let name = format!("={}{}", COLUMNS[column], row + 2);
    assert_eq!(*rig.committed.borrow(), [CellEdit::new(row, column, name)]);
}

/// Ctrl-click flips one cell: a second cell joins the selection and the first stays; a Ctrl-click on a selected
/// cell takes only that cell out.
#[trial]
fn ctrl_click_toggles_one_cell_and_keeps_the_others() {
    let mut rig = rig();
    let grid = rig.fixture.by_role(Role::TABLE).first().copied().expect("the grid showed");
    let first = rig.lead();
    let other_column = (first.1 + 1) % COLUMNS.len();
    rig.ctrl_click(grid.center() + Vec2::new(0.0, ROW_STEP));
    let second = rig.lead();
    assert_ne!(second.0, first.0, "the Ctrl-click landed on another row");
    let order = Rows::new(ROWS);
    let cell = |rig: &Rig, (row, column): (usize, usize)| rig.selection.borrow().contains_cell(RowAt::new(&row, row, &order), column as u32);
    assert!(cell(&rig, first) && cell(&rig, second), "both cells are selected");
    assert!(!cell(&rig, (first.0, other_column)), "the rest of the first cell's row is not");
    rig.ctrl_click(grid.center());
    assert!(!cell(&rig, first) && cell(&rig, second), "a Ctrl-click on a selected cell takes only that cell out");
}

/// The table carries its row and column counts, each row its index and the counts, and each cell its row and column.
#[trial]
fn the_table_rows_and_cells_carry_their_indexes_and_counts() {
    let rig = rig();
    let (fixture, tree) = (&rig.fixture, rig.fixture.tree());
    let by_place = |role: Role| {
        let mut found = fixture.by_role(role);
        found.sort_by(|a, b| (a.rect.min.y, a.rect.min.x).partial_cmp(&(b.rect.min.y, b.rect.min.x)).unwrap_or(std::cmp::Ordering::Equal));
        found
    };
    let table = fixture.by_role(Role::TABLE).first().copied().expect("the table showed");
    let counts = tree.grid_of(table.node);
    assert_eq!((counts.row_count, counts.column_count), (Some(ROWS), Some(COLUMNS.len())));
    for (at, row) in by_place(Role::ROW).iter().enumerate() {
        let place = tree.grid_of(row.node);
        assert_eq!((place.row_index, place.row_count, place.column_count), (Some(at), Some(ROWS), Some(COLUMNS.len())), "row {at}");
    }
    let cells = by_place(Role::CELL);
    assert!(cells.len() >= COLUMNS.len(), "the first row's cells showed");
    for (at, cell) in cells.iter().enumerate() {
        let place = tree.grid_of(cell.node);
        assert_eq!((place.row_index, place.column_index), (Some(at / COLUMNS.len()), Some(at % COLUMNS.len())), "cell {at}");
    }
}
