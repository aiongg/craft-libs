//! The arrow split of 18.4 in `SheetGrid`: arrows are navigation with no cell open and in Enter mode, the caret's in
//! Edit mode, and the reference cursor's in formula point mode.

use craft_behavior::CellSelection;
use craft_ui::contract::{Bind, Props as _, Role, ShowCollection as _};
use craft_ui::testing::examples::sheet::{CellEdit, Rows, SheetGrid, SheetRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// Enough rows that two steps down from the clicked middle cell stay inside the sheet.
const ROWS: usize = 12;
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

    fn type_text(&mut self, text: &str) {
        self.fixture.type_text(text);
        self.fixture.idle(2);
    }
}

/// With no cell open an arrow moves the lead cell; typing opens it, and an arrow then still navigates and commits
/// the draft into the cell it was typed in.
#[trial]
fn arrows_navigate_with_no_cell_open_and_in_enter_mode() {
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

/// F2 opens the cell with its text and claims Left and Right for the caret: the lead stays, the text changes where
/// the caret is, and Enter commits.
#[trial]
fn edit_mode_claims_the_horizontal_arrows_for_the_caret() {
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
