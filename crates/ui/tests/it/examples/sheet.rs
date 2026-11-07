//! The arrow split of 18.4 in `SheetGrid`: arrows are navigation with no cell open and in Enter mode, the caret's in
//! Edit mode, and the reference cursor's in formula point mode.

use craft_behavior::{CellSelection, SelectionModel as _};
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
        let row = *selection.rows.lead().expect("a cell holds the lead");
        let column = selection.lead_column.as_ref().and_then(|key| COLUMNS.iter().position(|name| craft_behavior::ColumnKey::new(name) == *key));
        (row, column.unwrap_or(0))
    }

    fn press(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
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
    let cell = |rig: &Rig, (row, column): (usize, usize)| rig.selection.borrow().contains_cell(&row, column as u32);
    assert!(cell(&rig, first) && cell(&rig, second), "both cells are selected");
    assert!(!cell(&rig, (first.0, other_column)), "the rest of the first cell's row is not");
    rig.ctrl_click(grid.center());
    assert!(!cell(&rig, first) && cell(&rig, second), "a Ctrl-click on a selected cell takes only that cell out");
}
