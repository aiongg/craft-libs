//! The arrow split of 18.4 in `SheetGrid`: arrows are navigation with no cell open and in Enter mode, the caret's in
//! Edit mode, and the reference cursor's in formula point mode.

use craft_behavior::{CellSelection, SelectionModel as _};
use craft_ui::contract::{Bind, Props as _, Role, ShowCollection as _};
use craft_ui::testing::examples::sheet::{CellEdit, Rows, SheetGrid, SheetRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const ROWS: usize = 8;
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
                .show_items(ui, &Rows(held.len()), &mut |key, icx| SheetRow::new(&held[*key], icx.cells().to_vec()));
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
        let row = *selection.rows.lead().expect("a cell holds the lead");
        let column = selection.lead_column.as_ref().and_then(|key| COLUMNS.iter().position(|name| craft_behavior::ColumnKey::new(name) == *key));
        (row, column.unwrap_or(0))
    }

    fn press(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
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
    assert_eq!(rig.lead(), (row, column));
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
