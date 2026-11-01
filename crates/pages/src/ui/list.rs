//! [`PageList`] (N3.21): Illustrator's Artboards panel and InDesign's page list as one list: a
//! numbered row per page or artboard with its name, picked, reordered by dragging or with the
//! stacking keys, renamed in place, with New and Delete in the footer and a lock column.
//!
//! A composite of craft-ui's list pieces, never a copy: [`List`](craft_ui::list::List) with
//! [`ListRow`](craft_ui::list::ListRow)s (a [`ToggleColumn::lock`](craft_ui::list::ToggleColumn::lock)
//! column, the number painted in the row's glyph slot, the options icon as a trailing item),
//! `List::reorder` for drags and stacking keys, `List`'s rename for names, and a
//! [`PanelFooter`](craft_ui::composites::PanelFooter) with New and Delete as drop-aware buttons.
//!
//! ```ignore
//! let out = PageList::new("artboards", &rows, &mut picked, active).show(ui);
//! for event in out.events {
//!     match event {
//!         PageListEvent::Activate(i) => doc.set_active_artboard(i),
//!         PageListEvent::Fit(i) => view.fit_artboard(i),
//!         PageListEvent::Rename { row, text } => doc.rename_artboards(&picked, row, &text),
//!         PageListEvent::Reorder(EditEvent::Commit { value, .. }) => doc.move_artboards(value),
//!         PageListEvent::Lock(edit) => doc.apply_locks(edit),
//!         PageListEvent::New { duplicate, options } => doc.new_artboard(duplicate, options),
//!         _ => {}
//!     }
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::{Move, Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use craft_ui::list::{ContextMenuRequest, ToggleEdit};
use egui::{Response, Ui};

craft_ui::words! {
    /// The list's words (VectorCraft's strings).
    #[non_exhaustive]
    pub struct PageListLabels {
        artboards: "pageList.artboards" = "Artboards";
        new_artboard: "pageList.new" = "New Artboard";
        delete: "pageList.delete" = "Delete Artboard";
        options: "pageList.options" = "Artboard Options";
        name: "pageList.name" = "Name";
        lock: "pageList.lock" = "Lock";
    }
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageListRow {
    pub name: String,
    pub locked: bool,
}

/// What the list reports, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PageListEvent {
    /// The picked rows changed (applied to the selection given), as `ListEvent::Selection`.
    Picked(SelectionChange<usize>),
    /// A click on a row, or the lead moved by a key: it becomes the active page or artboard.
    Activate(usize),
    /// A double-click on a row's number: fit it in the window (Illustrator).
    Fit(usize),
    /// A name typed in place (F2, a double-click on the name, the context menu's Rename), never
    /// unchanged; with several rows picked the app gives the name to all of them and numbers them
    /// (Illustrator).
    Rename { row: usize, text: String },
    /// A drag of rows or a stacking key (⌘/Ctrl+] and [), as `List` reports reorders.
    Reorder(EditEvent<Move<usize>>),
    /// A click or a sweep in the lock column, as `List` reports toggle columns.
    Lock(EditEvent<ToggleEdit<usize>>),
    /// New, or a row dropped on New (`duplicate`, the dragged rows' `grabbed` row); `options`
    /// with Alt held (open the options for the new one first).
    New { duplicate: Option<usize>, options: bool },
    /// Delete (the button, the Delete key, or rows dropped on it): the picked rows that aren't
    /// locked, or the dropped rows.
    Delete(Vec<usize>),
    /// The options icon: a click on the active row's, a double-click on another's.
    Options(usize),
    /// The app's menu is asked for (`List`'s context menu request).
    ContextMenu(ContextMenuRequest<usize>),
}

/// The list's result.
#[derive(Debug)]
#[must_use = "the app applies the list's events"]
pub struct PageListOutput {
    pub response: Response,
    pub events: Vec<PageListEvent>,
}

/// The list, built per pass (behaviour: Photoshop's Layers and Artboards panel rules where they
/// differ from Illustrator's, then VectorCraft).
///
/// - Rows are `ListRow`s: the number in the glyph slot (its width from the widest number, so
///   names line up), the name, the options icon as a trailing item, the lock as a toggle column.
///   The active row is the list's lead and shows the selected fill; picked rows the highlight.
///   Names wider than the row are elided by `ListRow`.
/// - Pointer, keys, rename and drag are `List`'s: click activates and picks; ⇧-click extends,
///   ⌘/Ctrl-click toggles; one Tab stop, ↑/↓ move the active row, F2 renames, Enter activates,
///   Delete deletes, ⌘/Ctrl+] and [ move the picked rows, the Menu key and ⇧F10 open the menu.
/// - Footer: New and Delete are `PanelFooter::drop_button`s taking this list's `ListDrag`
///   (a row dropped on New duplicates the grabbed row; on Delete deletes the dragged rows).
/// - AccessKit: `List`'s ListBox named "Artboards"; the rename field is named "Name".
pub struct PageList<'a> {
    id_salt: egui::IdSalt,
    rows: &'a [PageListRow],
    picked: &'a mut Selection<usize>,
    active: usize,
    rename: Option<usize>,
    labels: Option<PageListLabels>,
}

impl<'a> PageList<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, rows: &'a [PageListRow], picked: &'a mut Selection<usize>, active: usize) -> Self {
        PageList { id_salt: egui::IdSalt::new(id_salt), rows, picked, active, rename: None, labels: None }
    }

    /// The id the list keeps its memory under (`List::id_in` of the same salt).
    pub fn id_in(ui: &Ui, id_salt: impl egui::AsIdSalt) -> egui::Id {
        craft_ui::list::List::id_in(ui, id_salt)
    }

    /// Start renaming `row` in this pass (the app's Rename command), through `List::start_rename`.
    pub fn start_rename(mut self, row: usize) -> Self {
        self.rename = Some(row);
        self
    }

    pub fn labels(mut self, labels: PageListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // vectorcraft: crates/ui-egui/src/panels/artboards.rs:396 `show`, :297 `select_range`, :311
    // `toggle`, :243 `rename`, :371 `duplicate`.
    pub fn show(self, ui: &mut Ui) -> PageListOutput {
        todo!()
    }
}
