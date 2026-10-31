#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`ListManagerDialog`]: a named list with a name box and a column of buttons that run, edit,
//! create or delete entries (N9.09, from GridCraft's Macros dialog, after Excel's).

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The list manager's standard button words, English by default; the app's own actions
    /// bring their labels.
    #[non_exhaustive]
    pub struct ListManagerLabels {
        run: "listManager.run" = "Run";
        edit: "listManager.edit" = "Edit";
        create: "listManager.create" = "Create";
        delete: "listManager.delete" = "Delete";
        options: "listManager.options" = "Options…";
        description: "listManager.description" = "Description";
        /// The search field's placeholder, with [`ListManagerDialog::searchable`].
        search: "listManager.search" = "Search";
    }
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListEntry {
    /// The app's id for the entry; the answer carries it.
    pub id: String,
    pub name: String,
    /// After the name in the row, dimmer: a shortcut (`Ctrl+Shift+K`), "(script, 3 steps)".
    pub detail: Option<String>,
    /// Shown under the list while the entry is selected.
    pub description: String,
    /// The app's grouping (GridCraft: macro or script); actions can require one.
    pub kind: u32,
}

/// When an action's button is enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionEnabled {
    /// Always (Cancel-like actions).
    Always,
    /// When the name box names an entry (of `kind`, when given).
    OnEntry { kind: Option<u32> },
    /// When the name box holds a name no entry has (Create).
    OnNewName,
}

/// One button of the column, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListAction {
    /// The app's id ("run", "stepInto"); the answer carries it.
    pub id: String,
    pub label: String,
    pub tooltip: Option<String>,
    pub enabled: ActionEnabled,
    /// Closes the dialog with the answer (Run, Edit, Create); otherwise the dialog stays open and
    /// the app reads it with [`ListManagerDialog::take_action`] (Delete asks first; Options…
    /// opens a child dialog).
    pub closes: bool,
}

impl ListAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>, enabled: ActionEnabled) -> Self {
        Self { id: id.into(), label: label.into(), tooltip: None, enabled, closes: true }
    }

    /// Run, enabled on an entry, closing; the default action (Enter, double-click).
    pub fn run(labels: &ListManagerLabels) -> Self {
        Self::new("run", labels.run.clone(), ActionEnabled::OnEntry { kind: None })
    }

    pub fn edit(labels: &ListManagerLabels) -> Self {
        Self::new("edit", labels.edit.clone(), ActionEnabled::OnEntry { kind: None })
    }

    pub fn create(labels: &ListManagerLabels) -> Self {
        Self::new("create", labels.create.clone(), ActionEnabled::OnNewName)
    }

    /// Delete, enabled on an entry, keeping the dialog open for the app's confirmation; also the
    /// Delete key in the list.
    pub fn delete(labels: &ListManagerLabels) -> Self {
        Self { closes: false, ..Self::new("delete", labels.delete.clone(), ActionEnabled::OnEntry { kind: None }) }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Enabled only on entries of `kind`.
    pub fn kind(mut self, kind: u32) -> Self {
        if let ActionEnabled::OnEntry { kind: k } = &mut self.enabled {
            *k = Some(kind);
        }
        self
    }

    pub fn keeps_open(mut self) -> Self {
        self.closes = false;
        self
    }
}

/// A button pressed: which action, on which entry (`None` for a new name) and the name box's
/// text, trimmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListChoice {
    pub action: String,
    pub entry: Option<String>,
    pub name: String,
}

/// GridCraft's Macros dialog made general: the name box ("Macro name:") over a 330 × 200 list on
/// the left, the action buttons (100 × 24) in a column at the right with Cancel last, the
/// selected entry's description under them, and an optional note at the bottom.
///
/// Typing in the name box selects the entry with that name (case ignored); clicking a row puts
/// its name in the box. With [`searchable`](Self::searchable) the box also filters the rows to
/// names containing the text (folded, as `craft_i18n::Folder` folds). Keys: Up and Down move the
/// selection while the box or the list has focus, Enter runs the first action when enabled,
/// Delete in the list presses Delete, Esc cancels. Double-clicking a row runs the first action.
/// The list is a list box with one Tab stop; its rows are AccessKit list items with the detail
/// in their description.
///
/// The app replaces the rows while the dialog is open (a script deleted) with
/// `dialogs.get_mut::<ListManagerDialog>().map(|d| d.set_entries(rows))`.
pub struct ListManagerDialog {
    id: DialogId,
    title: String,
    name_label: String,
    entries: Vec<ListEntry>,
    actions: Vec<ListAction>,
    empty: String,
    note: Option<String>,
    searchable: bool,
    name: String,
    pressed: Option<ListChoice>,
}

impl ListManagerDialog {
    /// `key` names the dialog on the stack ("macros"); `title` ("Macros") and `name_label`
    /// ("Macro name:") are the app's words.
    pub fn new(key: &str, title: impl Into<String>, name_label: impl Into<String>) -> Self {
        Self {
            id: dialog_id(key),
            title: title.into(),
            name_label: name_label.into(),
            entries: Vec::new(),
            actions: Vec::new(),
            empty: String::new(),
            note: None,
            searchable: false,
            name: String::new(),
            pressed: None,
        }
    }

    pub fn entries(mut self, entries: Vec<ListEntry>) -> Self {
        self.entries = entries;
        self
    }

    pub fn actions(mut self, actions: Vec<ListAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Shown in the list, italic and dim, when it has no rows ("This workbook has no scripts or
    /// macros.").
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// A dim line at the bottom ("This workbook also has a VBA project…").
    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Opens with `name` in the box (and its entry selected).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Replaces the rows; the name box keeps its text.
    pub fn set_entries(&mut self, entries: Vec<ListEntry>) {
        self.entries = entries;
    }

    /// The last press of an action that keeps the dialog open.
    pub fn take_action(&mut self) -> Option<ListChoice> {
        self.pressed.take()
    }
}

impl StackDialog for ListManagerDialog {
    type Result = ListChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // gridcraft: crates/ui-egui/src/macro_ui.rs:367 macros_dialog
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ListChoice>> {
        todo!()
    }
}
