//! [`PageList`] (N3.21): Illustrator's Artboards panel and InDesign's page list as one list: a
//! numbered row per page or artboard with its name, picked, reordered by dragging or with Move
//! Up / Move Down, renamed in place, with New and Delete in the footer and a lock column.
//!
//! ```ignore
//! let r = PageList::new("artboards", &rows, &mut picked, active).show(ui);
//! match r.action {
//!     Some(PageListAction::Activate(i)) => doc.set_active_artboard(i),
//!     Some(PageListAction::Fit(i)) => view.fit_artboard(i),
//!     Some(PageListAction::Rename { rows, name }) => doc.rename_artboards(&rows, &name),
//!     Some(PageListAction::Move(mv)) => doc.move_artboards(mv),
//!     Some(PageListAction::New { duplicate, options }) => doc.new_artboard(duplicate, options),
//!     _ => {}
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::Selection;
use egui::{Response, Ui};

use super::thumbnails::PageMove;

craft_ui::words! {
    /// The list's words (VectorCraft's strings).
    #[non_exhaustive]
    pub struct PageListLabels {
        new_artboard: "pageList.new" = "New Artboard";
        delete: "pageList.delete" = "Delete Artboard";
        move_up: "pageList.moveUp" = "Move Up";
        move_down: "pageList.moveDown" = "Move Down";
        options: "pageList.options" = "Artboard Options…";
        rename: "pageList.rename" = "Rename";
        locked: "pageList.locked" = "Locked";
        unlocked: "pageList.unlocked" = "Unlocked";
    }
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageListRow {
    pub name: String,
    pub locked: bool,
}

/// What the user asked for this pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageListAction {
    /// A click on a row: it becomes the active page or artboard.
    Activate(usize),
    /// A double-click on a row's number: fit it in the window.
    Fit(usize),
    /// A name typed in place (double-click the name, or Rename): with several rows picked, the
    /// name goes to all of them and the app numbers them.
    Rename { rows: Vec<usize>, name: String },
    /// Rows dragged, or Move Up / Move Down.
    Move(PageMove),
    /// New: `duplicate` when a row was dropped on New (its index), `options` with Alt held (open
    /// the options for the new one first).
    New { duplicate: Option<usize>, options: bool },
    /// Delete: the picked rows that aren't locked.
    Delete(Vec<usize>),
    /// The options icon: a click on the active row's, a double-click on another's.
    Options(usize),
    /// A click in the lock column.
    ToggleLock(usize),
}

/// The list's result.
#[derive(Debug)]
pub struct PageListResponse {
    pub action: Option<PageListAction>,
    pub response: Response,
}

/// The list, built per pass. Rows are craft-ui list rows (row height, selection fill, focus
/// ring); the active row has the selected fill, picked rows the highlight.
///
/// - Click activates and picks; ⇧-click picks a range from the last plain click, ⌘/Ctrl-click
///   toggles one; ⌘/Ctrl-A picks all when the list has the keyboard.
/// - Drag reorders (craft-ui's reorder behaviour); a row dropped on New duplicates it.
/// - Keys: one Tab stop; ↑/↓ move the active row, F2 or Enter renames, Delete deletes, ⌘/Ctrl
///   ↑/↓ move the picked rows.
pub struct PageList<'a> {
    id_salt: egui::IdSalt,
    rows: &'a [PageListRow],
    picked: &'a mut Selection<usize>,
    active: usize,
    labels: Option<PageListLabels>,
}

impl<'a> PageList<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, rows: &'a [PageListRow], picked: &'a mut Selection<usize>, active: usize) -> Self {
        PageList { id_salt: egui::IdSalt::new(id_salt), rows, picked, active, labels: None }
    }

    pub fn labels(mut self, labels: PageListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // vectorcraft: crates/ui-egui/src/panels/artboards.rs:396 `show`, :297 `select_range`, :311
    // `toggle`, :243 `rename`, :371 `duplicate`.
    pub fn show(self, ui: &mut Ui) -> PageListResponse {
        todo!()
    }
}
