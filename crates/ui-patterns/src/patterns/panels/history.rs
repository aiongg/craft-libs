//! `HistoryPanel`: the undo history as a list of states and named snapshots, with a footer of snapshot buttons.

use crate::contract::{NoValue, Outcome, Pattern};
use crate::primitives::textures::{ThumbSource, Thumbnails};
use crate::runtime::env::Label;
use craft_behavior::IconId;
use craft_keymap::Mods;
use egui::{Response, Ui};

/// What a history row stands for: a named snapshot or a state of the undo stack, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HistoryKey {
    /// The snapshot with this number.
    Snapshot(u64),
    /// The state with this number.
    State(u64),
}

/// One row of the history list.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HistoryRow {
    /// What the row stands for.
    pub key: HistoryKey,
    /// The row's name.
    pub label: Label,
    /// The icon before the name.
    pub icon: Option<IconId>,
    /// The state is one that was undone and can be redone.
    pub redo: bool,
}

impl HistoryRow {
    /// A row for the snapshot `id` named `label`.
    pub fn snapshot(id: u64, label: impl Into<Label>) -> Self {
        let _ = (id, label.into());
        todo!("pn.C34")
    }

    /// A row for the state `id` named `label`.
    pub fn state(id: u64, label: impl Into<Label>) -> Self {
        let _ = (id, label.into());
        todo!("pn.C34")
    }

    /// Shows `icon` before the name.
    #[must_use]
    pub fn icon(mut self, icon: IconId) -> Self {
        let _ = (&mut self, icon);
        todo!("pn.C34")
    }

    /// Marks the state as one that can be redone.
    #[must_use]
    pub fn redo(mut self, redo: bool) -> Self {
        let _ = (&mut self, redo);
        todo!("pn.C34")
    }
}

/// Which buttons the history footer shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoryFooter {
    /// The Delete button.
    pub delete: bool,
    /// The New Snapshot button.
    pub new_snapshot: bool,
    /// The New Document button.
    pub new_document: bool,
}

/// What the user did in the history panel.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HistoryEvent {
    /// The user picked a row: the document goes to it.
    Select(HistoryKey),
    /// The user set the row as the source of the history brush.
    SetSource(HistoryKey),
    /// The user renamed a snapshot.
    Rename {
        /// The snapshot's number.
        snapshot: u64,
        /// The new name.
        text: String,
    },
    /// The user deleted the row, or the picked one when `None`.
    Delete(Option<HistoryKey>),
    /// The user asked for a new snapshot; `mods` were held.
    NewSnapshot {
        /// The modifiers held with the press.
        mods: Mods,
    },
    /// The user asked for a new document from the picked row.
    NewDocument,
}

/// What [`HistoryPanel`] reports.
pub struct HistoryPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// What the user did this pass, in order.
    pub events: Vec<HistoryEvent>,
}

impl Outcome for HistoryPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The history panel: `rows` with the `current` one marked, thumbnails of the snapshots and a footer of buttons.
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(panels-w8): pn.C34
pub struct HistoryPanel<'a> {
    rows: &'a [HistoryRow],
    current: Option<HistoryKey>,
}

impl<'a> HistoryPanel<'a> {
    /// A panel over `rows`, with `current` marked.
    pub fn new(rows: &'a [HistoryRow], current: Option<HistoryKey>) -> Self {
        let _ = (rows, current);
        todo!("pn.C34")
    }

    /// Marks the row that is the source of the history brush.
    #[must_use]
    pub fn source(mut self, source: Option<HistoryKey>) -> Self {
        let _ = (&mut self, source);
        todo!("pn.C34")
    }

    /// Shows the pictures of the rows from `source`.
    #[must_use]
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<u64>, source: &'a dyn ThumbSource<u64>) -> Self {
        let _ = (&mut self, thumbnails, source);
        todo!("pn.C34")
    }

    /// Which rows the Delete button and key take; the others refuse.
    #[must_use]
    pub fn can_delete(mut self, can: impl Fn(&HistoryKey) -> bool + 'a) -> Self {
        let _ = (&mut self, can);
        todo!("pn.C34")
    }

    /// Which footer buttons show.
    #[must_use]
    pub fn footer(mut self, footer: HistoryFooter) -> Self {
        let _ = (&mut self, footer);
        todo!("pn.C34")
    }

    /// Scrolls to `key` and shows it.
    #[must_use]
    pub fn reveal(mut self, key: HistoryKey) -> Self {
        let _ = (&mut self, key);
        todo!("pn.C34")
    }

    /// Opens the rename field of the snapshot `snapshot`.
    #[must_use]
    pub fn start_rename(mut self, snapshot: u64) -> Self {
        let _ = (&mut self, snapshot);
        todo!("pn.C34")
    }

    /// Whether the panel takes input.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        let _ = (&mut self, enabled);
        todo!("pn.C34")
    }

    /// Shows the panel.
    pub fn show(self, ui: &mut Ui) -> HistoryPanelOutput {
        let _ = ui;
        todo!("pn.C34")
    }
}
