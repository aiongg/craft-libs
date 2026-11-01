//! Reference picking: a formula field in point mode takes a cell reference from a click on the
//! grid (N6.07).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use super::grid::CellRef;

/// Excel's point mode for a dialog's formula box (GridCraft's conditional-format and validation
/// dialogs): while the box is in point mode, a click on the grid writes the cell's absolute
/// reference at the caret, a drag grows it into a range, and the next click replaces the
/// reference the last one inserted. An empty box starts with `=`. After an operand there is no
/// place for a reference and nothing changes. A reference to another sheet carries the sheet's
/// name, quoted by the app's rule.
///
/// The app keeps one per open dialog: it calls [`set_caret`](Self::set_caret) while the box has
/// the keyboard, [`pick`](Self::pick) on grid presses and drags, and gives the box the keyboard
/// back when [`take_refocus`](Self::take_refocus) says so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferencePicker {
    /// The dialog's own sheet, by name (its index changes when sheets move).
    sheet: String,
    /// Caret (chars), mirrored from the box.
    caret: usize,
    /// The reference last inserted, replaced by the next pick.
    point: Option<Pointed>,
    refocus: bool,
}

/// The reference a pick inserted: its byte span in the text and the cell the press was on (a
/// drag grows the range from it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pointed {
    span: (usize, usize),
    anchor: CellRef,
}

impl ReferencePicker {
    pub fn new(sheet: impl Into<String>) -> Self {
        Self { sheet: sheet.into(), caret: 0, point: None, refocus: false }
    }

    /// The caret (chars) while the box has the keyboard; typing forgets the last reference.
    pub fn set_caret(&mut self, caret: usize, typed: bool) {
        self.caret = caret;
        if typed {
            self.point = None;
        }
    }

    /// Insert the reference to `cell` on `sheet` into `text`; `extend` (a drag) grows a range
    /// from the press's cell. Returns whether `text` changed. `can_point` is the app's formula rule
    /// (a reference may go at the caret: after `=`, an operator, `(` or `,`); `quote` quotes a
    /// sheet name (`'My Sheet'`).
    // gridcraft: crates/ui-egui/src/dialogs.rs:140 `pick_reference`, `:79` `RefPick`.
    #[expect(clippy::ptr_arg, reason = "skeleton: the body edits the String")]
    pub fn pick(
        &mut self,
        text: &mut String,
        cell: CellRef,
        sheet: &str,
        extend: bool,
        can_point: impl Fn(&str, usize) -> bool,
        quote: impl Fn(&str) -> String,
    ) -> bool {
        todo!()
    }

    /// Whether to give the box the keyboard back this pass, caret after the reference (once).
    pub fn take_refocus(&mut self) -> Option<usize> {
        std::mem::take(&mut self.refocus).then_some(self.caret)
    }
}
