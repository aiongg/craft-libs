//! [`ListManagerDialog`]: a named list with a name box and a column of buttons that run, edit,
//! create or delete entries (N9.09, from GridCraft's Macros dialog, after Excel's).

use craft_host::Host;
use craft_theme::TypeRole;
use egui::{Context, Key, Modifiers, vec2};

use super::dialog::{Dialog, InitialFocus, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::{DialogLabels, Footer};
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::behavior::{ClaimedKey, KeyClaim, RowInfo, Selection, SelectionMode};
use crate::controls::{Button, TextField};
use crate::list::{List, ListEvent, ListRow};
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};
use crate::text::text_width;
use crate::theme::{ToEgui, Tokens, TokensOf, role_format};

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
pub struct ListManagerEntry {
    /// The app's id for the entry; the answer carries it.
    pub id: String,
    /// The name shown in the row and typed in the name box.
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
    OnEntry {
        /// Only entries of this kind, when given.
        kind: Option<u32>,
    },
    /// When the name box holds a name no entry has (Create).
    OnNewName,
}

/// One button of the column, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListManagerAction {
    /// The app's id ("run", "stepInto"); the answer carries it.
    pub id: String,
    /// The button's label.
    pub label: String,
    /// Tooltip text, if any.
    pub tooltip: Option<String>,
    /// When the button is enabled.
    pub enabled: ActionEnabled,
    /// Closes the dialog with the answer (Run, Edit, Create); otherwise the dialog stays open and
    /// the app reads it with [`ListManagerDialog::take_action`] (Delete asks first; Options…
    /// opens a child dialog).
    pub closes: bool,
}

impl ListManagerAction {
    /// An action with the app's `id`, a translated `label` and its enabled rule; it closes the dialog.
    pub fn new(id: impl Into<String>, label: impl Into<String>, enabled: ActionEnabled) -> Self {
        Self { id: id.into(), label: label.into(), tooltip: None, enabled, closes: true }
    }

    /// Run, enabled on an entry, closing; the default action (Enter, double-click).
    pub fn run(labels: &ListManagerLabels) -> Self {
        Self::new("run", labels.run.clone(), ActionEnabled::OnEntry { kind: None })
    }

    /// Edit, enabled on an entry, closing.
    pub fn edit(labels: &ListManagerLabels) -> Self {
        Self::new("edit", labels.edit.clone(), ActionEnabled::OnEntry { kind: None })
    }

    /// Create, enabled when the name box holds a new name, closing.
    pub fn create(labels: &ListManagerLabels) -> Self {
        Self::new("create", labels.create.clone(), ActionEnabled::OnNewName)
    }

    /// Delete, enabled on an entry, keeping the dialog open for the app's confirmation; also the
    /// Delete key in the list.
    pub fn delete(labels: &ListManagerLabels) -> Self {
        Self { closes: false, ..Self::new("delete", labels.delete.clone(), ActionEnabled::OnEntry { kind: None }) }
    }

    /// Sets the tooltip.
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

    /// Keeps the dialog open when pressed; the app reads the press with [`ListManagerDialog::take_action`].
    pub fn keeps_open(mut self) -> Self {
        self.closes = false;
        self
    }
}

/// A button pressed: which action, on which entry (`None` for a new name) and the name box's
/// text, trimmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListManagerChoice {
    /// The pressed action's id.
    pub action: String,
    /// The id of the selected entry; `None` for a new name.
    pub entry: Option<String>,
    /// The name box's text, trimmed.
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
/// The name box claims Up, Down and Enter ([`KeyClaim`](crate::behavior::KeyClaim)), so the app's
/// shortcuts never take them while it has focus. The list is the library's
/// [`List`](crate::list::List) with [`ListRow`](crate::list::ListRow)s (single selection, one Tab
/// stop, the list keys); its rows are AccessKit list items with the detail in their description.
///
/// The app replaces the rows while the dialog is open (a script deleted) with
/// `dialogs.get_mut::<ListManagerDialog>().map(|d| d.set_entries(rows))`.
pub struct ListManagerDialog {
    id: DialogId,
    title: String,
    name_label: String,
    entries: Vec<ListManagerEntry>,
    actions: Vec<ListManagerAction>,
    empty: String,
    note: Option<String>,
    searchable: bool,
    name: String,
    pressed: Option<ListManagerChoice>,
    state: DialogState<()>,
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
            state: DialogState::new(),
        }
    }

    /// The rows to list.
    pub fn entries(mut self, entries: Vec<ListManagerEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// The buttons, top to bottom.
    pub fn actions(mut self, actions: Vec<ListManagerAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Shown in the list, dim, when it has no rows ("This workbook has no scripts or
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

    /// Shows a search box above the list when true.
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
    pub fn set_entries(&mut self, entries: Vec<ListManagerEntry>) {
        self.entries = entries;
    }

    /// The last press of an action that keeps the dialog open.
    pub fn take_action(&mut self) -> Option<ListManagerChoice> {
        self.pressed.take()
    }
}

impl StackDialog for ListManagerDialog {
    type Result = ListManagerChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // gridcraft: crates/ui-egui/src/macro_ui.rs:367 macros_dialog
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ListManagerChoice>> {
        let cancel_label = DialogLabels::of_ctx(ctx).cancel;
        let id = self.id;
        let Self { title, name_label, entries, actions, empty, note, searchable, name, pressed, state, .. } = self;
        let sizes = crate::theme::current(ctx).component.list_manager;
        let list_size = vec2(sizes.list[0], sizes.list[1]);
        let (column_gap, button_width) = (sizes.column_gap, sizes.button_width);
        let width = list_size.x + column_gap + button_width;
        let mut chosen: Option<usize> = None;
        let mut cancelled = false;
        let response = Dialog::with_id(id, title.as_str())
            .stack(stack)
            .width(Width::Fixed(width))
            .footer(Footer::none())
            .initial_focus(InitialFocus::FirstField)
            .show(ctx, state, &mut (), |ui, ()| {
                let ctx = ui.ctx().clone();
                let field_id = id.with("name");
                let folder = folder_of(&ctx);
                let mut label_height = 0.0;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = column_gap;
                    ui.vertical(|ui| {
                        ui.set_width(list_size.x);
                        label_height = ui.add(Text::new(name_label.as_str())).rect.height() + ui.spacing().item_spacing.y;
                        // Up and Down move the selection while the box has the focus, Enter runs.
                        let boxed = ctx.memory(|m| m.has_focus(field_id));
                        if boxed {
                            KeyClaim::claim(
                                &ctx,
                                field_id,
                                &[Key::ArrowUp, Key::ArrowDown, Key::Enter].map(|key| ClaimedKey::exact(Modifiers::NONE, key)),
                            );
                        }
                        let shown = visible(entries, &folder, name, *searchable);
                        let mut reveal = None;
                        if boxed {
                            let step = ui.input_mut(|i| {
                                i32::from(i.consume_key(Modifiers::NONE, Key::ArrowDown)) - i32::from(i.consume_key(Modifiers::NONE, Key::ArrowUp))
                            });
                            if step != 0 && !shown.is_empty() {
                                let current = shown.iter().position(|&e| is_named(&entries[e], &folder, name));
                                let target = match (current, step > 0) {
                                    (Some(c), true) => (c + 1).min(shown.len() - 1),
                                    (Some(c), false) => c.saturating_sub(1),
                                    (None, true) => 0,
                                    (None, false) => shown.len() - 1,
                                };
                                *name = entries[shown[target]].name.clone();
                                reveal = Some(entries[shown[target]].id.clone());
                            }
                        }
                        let edit = TextField::new(id.with("name-box"), name_label.as_str(), name.as_str()).id(field_id).keep_focus(true).show(ui);
                        if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                            name.clone_from(value);
                        }
                        let shown = visible(entries, &folder, name, *searchable);
                        if edit.submitted {
                            chosen = Some(0).filter(|&first| actions.get(first).is_some_and(|a| is_enabled(a, entries, &folder, name)));
                        }
                        let mut selection =
                            entries.iter().find(|e| is_named(e, &folder, name)).map_or_else(Selection::new, |e| Selection::only(e.id.clone()));
                        let rows: Vec<RowInfo<String>> = shown.iter().map(|&e| RowInfo::leaf(entries[e].id.clone())).collect();
                        let tokens = Tokens::of(ui);
                        let detail_font = role_format(&tokens, TypeRole::Body, Content::Faint.color(&tokens).to_egui()).font_id;
                        let detail_widths: Vec<f32> = entries
                            .iter()
                            .map(|e| e.detail.as_deref().filter(|d| !d.is_empty()).map_or(0.0, |d| text_width(ui, d, &detail_font) + column_gap))
                            .collect();
                        ui.allocate_ui(list_size, |ui| {
                            if entries.is_empty() {
                                ui.add(Text::new(empty.as_str()).content(Content::Faint).italic());
                                return;
                            }
                            let mut list = List::new(id.with("list"), title.as_str(), rows.as_slice())
                                .selection(&mut selection)
                                .mode(SelectionMode::Single)
                                .allow_empty(true)
                                .max_height(list_size.y);
                            if let Some(key) = reveal {
                                list = list.reveal(key);
                            }
                            let out = list.show(ui, |_, key| {
                                let Some(index) = entries.iter().position(|e| e.id == *key) else { return ListRow::new(key.clone(), "") };
                                let entry = &entries[index];
                                let row = ListRow::new(key.clone(), entry.name.as_str());
                                match entry.detail.as_deref().filter(|d| !d.is_empty()) {
                                    Some(detail) => {
                                        let detail = detail.to_owned();
                                        row.description(detail.clone()).trailing_item(detail_widths[index], move |ui| {
                                            ui.add(Text::new(detail).content(Content::Faint));
                                        })
                                    }
                                    None => row,
                                }
                            });
                            for event in out.events {
                                let activated = matches!(event, ListEvent::Activate(_));
                                match event {
                                    ListEvent::Selection(change) => {
                                        if let Some(entry) = change.row.and_then(|row| entries.iter().find(|e| e.id == row)) {
                                            name.clone_from(&entry.name);
                                        }
                                    }
                                    ListEvent::Clicked(key) | ListEvent::Activate(key) => {
                                        if let Some(entry) = entries.iter().find(|e| e.id == key) {
                                            name.clone_from(&entry.name);
                                        }
                                        if chosen.is_none() && activated {
                                            chosen =
                                                Some(0).filter(|&first| actions.get(first).is_some_and(|a| is_enabled(a, entries, &folder, name)));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if out.response.has_focus()
                                && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete))
                                && let Some(index) = actions.iter().position(|a| a.id == "delete")
                                && is_enabled(&actions[index], entries, &folder, name)
                            {
                                chosen = Some(index);
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.add_space(label_height);
                        for (index, action) in actions.iter().enumerate() {
                            let enabled = is_enabled(action, entries, &folder, name);
                            let response =
                                ui.add(Button::new(action.label.as_str()).id_salt((id, "action", index)).min_width(button_width).enabled(enabled));
                            let response = match &action.tooltip {
                                Some(tip) => response.on_hover_text(tip.as_str()).on_disabled_hover_text(tip.as_str()),
                                None => response,
                            };
                            if response.clicked() {
                                chosen = Some(index);
                            }
                        }
                        ui.add_space(ui.spacing().item_spacing.y);
                        if ui.add(Button::new(cancel_label.as_str()).id_salt((id, "cancel")).min_width(button_width)).clicked() {
                            cancelled = true;
                        }
                    });
                });
                ui.add_space(ui.spacing().item_spacing.y);
                let description = entries.iter().find(|e| is_named(e, &folder, name)).map(|e| e.description.clone()).unwrap_or_default();
                ui.add(Text::new(description_label(&ctx)));
                ui.add(Text::new(if description.is_empty() { " ".to_owned() } else { description }).content(Content::Muted));
                if let Some(note) = note {
                    ui.add_space(ui.spacing().item_spacing.y);
                    ui.add(Text::new(note.as_str()).role(TypeRole::Caption).content(Content::Muted));
                }
            });
        if cancelled || matches!(response.outcome, Some(Outcome::Cancel | Outcome::Close)) {
            return Some(DialogResult::Cancel);
        }
        let action = actions.get(chosen?)?;
        let found = entries.iter().find(|e| is_named(e, &folder_of(ctx), name) && action_accepts(action, e));
        let choice = ListManagerChoice { action: action.id.clone(), entry: found.map(|e| e.id.clone()), name: name.trim().to_owned() };
        if action.closes {
            Some(DialogResult::Ok(choice))
        } else {
            *pressed = Some(choice);
            None
        }
    }
}

/// Whether the name box names `entry`: its trimmed text equals the entry's name, case ignored.
/// The folding of the Host's language ("en" without a Host or a catalog), as every search field
/// of the interface folds.
fn folder_of(ctx: &Context) -> craft_i18n::Folder {
    let code = Host::try_of_ctx(ctx).and_then(|host| Some(host.i18n()?.code(host.lang()).to_owned())).unwrap_or_else(|| "en".to_owned());
    craft_i18n::Folder::new(&code)
}

fn is_named(entry: &ListManagerEntry, folder: &craft_i18n::Folder, name: &str) -> bool {
    folder.fold(&entry.name).as_str() == folder.fold(name.trim()).as_str()
}

/// Whether `action` takes `entry` as its target.
fn action_accepts(action: &ListManagerAction, entry: &ListManagerEntry) -> bool {
    match action.enabled {
        ActionEnabled::OnEntry { kind } => kind.is_none_or(|k| k == entry.kind),
        ActionEnabled::Always | ActionEnabled::OnNewName => true,
    }
}

/// The action's button state for the name box's text.
fn is_enabled(action: &ListManagerAction, entries: &[ListManagerEntry], folder: &craft_i18n::Folder, name: &str) -> bool {
    match action.enabled {
        ActionEnabled::Always => true,
        ActionEnabled::OnEntry { .. } => entries.iter().any(|e| is_named(e, folder, name) && action_accepts(action, e)),
        ActionEnabled::OnNewName => !name.trim().is_empty() && !entries.iter().any(|e| is_named(e, folder, name)),
    }
}

/// The indices of the rows the list shows: all of them, or with `searchable` those whose folded
/// name contains the folded text of the box.
fn visible(entries: &[ListManagerEntry], folder: &craft_i18n::Folder, name: &str, searchable: bool) -> Vec<usize> {
    let query = folder.fold(name.trim());
    (0..entries.len())
        .filter(|&i| !searchable || query.as_str().is_empty() || folder.fold(&entries[i].name).as_str().contains(query.as_str()))
        .collect()
}

fn description_label(ctx: &Context) -> String {
    ListManagerLabels::of_ctx(ctx).description
}
