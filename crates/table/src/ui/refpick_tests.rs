//! Reference picking.

use super::grid::CellRef;
use super::refpick::ReferencePicker;

fn cell(row: u32, column: u32) -> CellRef {
    CellRef { row, column }
}

/// A reference may follow `=` or an operator.
fn can_point(text: &str, caret: usize) -> bool {
    let before: String = text.chars().take(caret).collect();
    before.trim_end().ends_with(['=', '(', ',', '+', '-'])
}

fn quote(name: &str) -> String {
    format!("'{name}'")
}

// an empty box becomes =$B$3.
#[test]
fn a_click_inserts_an_absolute_reference() {
    let mut picker = ReferencePicker::new("Sheet1");
    let mut text = String::new();
    assert!(picker.pick(&mut text, cell(2, 1), "Sheet1", false, can_point, quote));
    assert_eq!(text, "=$B$3");
    assert_eq!(picker.take_refocus(), Some(5));
    assert_eq!(picker.take_refocus(), None, "the keyboard goes back once");

    let mut operand = "=A1".to_owned();
    picker.set_caret(3, true);
    assert!(!picker.pick(&mut operand, cell(2, 1), "Sheet1", false, can_point, quote), "no place for a reference after an operand");
    assert_eq!(operand, "=A1");
}

// extend makes $B$3:$C$5; the next click replaces it.
#[test]
fn a_drag_grows_a_range_and_another_click_replaces_it() {
    let mut picker = ReferencePicker::new("Sheet1");
    let mut text = "=SUM(".to_owned();
    picker.set_caret(5, false);
    assert!(picker.pick(&mut text, cell(2, 1), "Sheet1", false, can_point, quote));
    assert!(picker.pick(&mut text, cell(4, 2), "Sheet1", true, can_point, quote));
    assert_eq!(text, "=SUM($B$3:$C$5");
    assert!(picker.pick(&mut text, cell(0, 3), "Sheet1", false, can_point, quote));
    assert_eq!(text, "=SUM($D$1");
    picker.set_caret(text.chars().count(), true);
    assert!(!picker.pick(&mut text, cell(0, 0), "Sheet1", false, can_point, quote), "typing forgets the last reference");
}

// a pick on another sheet inserts 'My Sheet'!$A$1.
#[test]
fn other_sheets_are_quoted() {
    let mut picker = ReferencePicker::new("Sheet1");
    let mut text = "=".to_owned();
    picker.set_caret(1, false);
    assert!(picker.pick(&mut text, cell(0, 0), "My Sheet", false, can_point, quote));
    assert_eq!(text, "='My Sheet'!$A$1");
    assert!(picker.pick(&mut text, cell(0, 0), "SHEET1", false, can_point, quote), "the dialog's own sheet needs no prefix, in any case");
    assert_eq!(text, "=$A$1");
}
