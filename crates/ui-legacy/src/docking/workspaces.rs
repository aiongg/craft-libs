//! Named workspaces: Photoshop's Window › Workspace and the switcher at the right end of the
//! options bar. A workspace is a saved [`Workspace`] layout plus which bars and the panel set
//! show; the set holds the app's built-in workspaces, the user's saved ones, the current one and
//! Lock Workspace.

use std::{collections::BTreeMap, hash::Hash};

use egui::{AsIdSalt, Id, Response, Ui};

use super::Workspace;
use crate::controls::{ControlSize, Select};
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem};

/// Chrome a workspace remembers beside the dock layout: which bars show and the panel set.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct WorkspaceChrome {
    /// The menu bar shows.
    pub menu_bar: bool,
    /// The tool rail shows.
    pub tool_rail: bool,
    /// The options bar shows.
    pub options_bar: bool,
    /// The status bar shows.
    pub status_bar: bool,
}

/// One named workspace.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "P: serde::Deserialize<'de>"))]
pub struct NamedWorkspace<P> {
    /// Stable id (`"essentials"`, or a slug of a saved name).
    pub id: String,
    /// What the menus and the switcher show.
    pub name: String,
    /// The panel layout.
    pub layout: Workspace<P>,
    /// Which bars show.
    pub chrome: WorkspaceChrome,
    /// Whether Keyboard Shortcuts and Menus were saved with it (Photoshop's New Workspace boxes).
    pub keeps_shortcuts: bool,
    /// Whether menu customizations were saved with it.
    pub keeps_menus: bool,
}

/// The app's workspaces: built-ins first, saved ones after, one current. It is a settings
/// section (serde with the `serde` feature, as [`Workspace`]); the layouts of built-ins are
/// code, and only their user edits ([`edited`](Self::edited)) are stored.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, bound(deserialize = "P: serde::Deserialize<'de> + Eq + Hash + Clone"))]
pub struct WorkspaceSet<P> {
    /// Workspaces the app defines in code.
    pub builtin: Vec<NamedWorkspace<P>>,
    /// Workspaces the user saved.
    pub saved: Vec<NamedWorkspace<P>>,
    /// Id of the current workspace.
    pub current: String,
    /// Lock Workspace: panels can't be moved, closed or resized; the dock is shown locked.
    pub locked: bool,
    /// The live layout each workspace was left with, by id, until Reset or Delete. Photoshop
    /// remembers the state of each workspace this way.
    pub edited: BTreeMap<String, Workspace<P>>,
}

impl<P> Default for WorkspaceSet<P> {
    fn default() -> Self {
        Self { builtin: Vec::new(), saved: Vec::new(), current: String::new(), locked: false, edited: BTreeMap::new() }
    }
}

/// What a workspace menu or switcher row does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WorkspaceCommand {
    /// Switch to the workspace with this id.
    Select(String),
    /// Reset the current workspace to its saved layout.
    Reset,
    /// Open the New Workspace dialog.
    New,
    /// Open the Delete Workspace dialog.
    Delete,
    /// Toggle Lock Workspace.
    LockToggle,
}

impl<P: Clone + Eq + Hash> WorkspaceSet<P> {
    /// A set of `builtin` workspaces with the first one current.
    pub fn new(builtin: Vec<NamedWorkspace<P>>) -> Self {
        let current = builtin.first().map(|w| w.id.clone()).unwrap_or_default();
        Self { builtin, current, ..Self::default() }
    }

    /// The current workspace, or `None` when `current` names none.
    pub fn current(&self) -> Option<&NamedWorkspace<P>> {
        self.find(&self.current)
    }

    /// The workspace with this id.
    fn find(&self, id: &str) -> Option<&NamedWorkspace<P>> {
        self.builtin.iter().chain(&self.saved).find(|w| w.id == id)
    }

    /// `base`, or `base-2`, `base-3`… when another workspace has the id.
    fn free_id(&self, base: &str) -> String {
        let taken = |id: &str| self.find(id).is_some();
        if !taken(base) {
            return base.to_string();
        }
        (2..).map(|n| format!("{base}-{n}")).find(|id| !taken(id)).unwrap_or_default()
    }

    /// The layout the current workspace shows: what it was left with, else its stored one.
    pub fn layout(&self) -> Option<&Workspace<P>> {
        self.edited.get(&self.current).or_else(|| self.current().map(|w| &w.layout))
    }

    /// Reset, New…, Delete…, a separator and Lock Workspace, with `wrap` turning each command into
    /// the menu's command type. Delete is greyed while there is no saved workspace.
    fn command_rows<T>(&self, labels: &WorkspaceLabels, mut wrap: impl FnMut(WorkspaceCommand) -> T) -> Vec<MenuEntry<T>> {
        vec![
            MenuItem::new(labels.reset.clone(), wrap(WorkspaceCommand::Reset)).into(),
            MenuItem::new(labels.new.clone(), wrap(WorkspaceCommand::New)).into(),
            MenuItem::new(labels.delete.clone(), wrap(WorkspaceCommand::Delete)).enabled(!self.saved.is_empty()).into(),
            MenuEntry::Separator,
            MenuItem::new(labels.lock.clone(), wrap(WorkspaceCommand::LockToggle)).check(self.locked).into(),
        ]
    }

    /// Switch to `id`; the current workspace's live layout is kept under its id first (Photoshop
    /// remembers the state of each workspace until Reset).
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:252 `select_workspace`.
    pub fn select(&mut self, id: &str, live: Workspace<P>) -> Result<(), WorkspaceError> {
        if self.find(id).is_none() {
            return Err(WorkspaceError::UnknownId);
        }
        if self.find(&self.current).is_some() {
            self.edited.insert(self.current.clone(), live);
        }
        self.current = id.to_string();
        Ok(())
    }

    /// Save `live` under `name` and make it current; a name a built-in has is refused.
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:207 `new_workspace`.
    pub fn save(&mut self, name: &str, live: Workspace<P>, chrome: WorkspaceChrome) -> Result<String, WorkspaceError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        if self.builtin.iter().any(|w| w.name.to_lowercase() == name.to_lowercase() || w.id.to_lowercase() == name.to_lowercase()) {
            return Err(WorkspaceError::NameTaken);
        }
        let existing = self.saved.iter().position(|w| w.name == name);
        let id = match existing {
            Some(i) => self.saved[i].id.clone(),
            None => self.free_id(&slug(name)),
        };
        let mut layout = live;
        layout.saved = None;
        let (keeps_shortcuts, keeps_menus) = existing.map_or((false, false), |i| (self.saved[i].keeps_shortcuts, self.saved[i].keeps_menus));
        let workspace = NamedWorkspace { id: id.clone(), name: name.to_string(), layout, chrome, keeps_shortcuts, keeps_menus };
        match existing {
            Some(i) => self.saved[i] = workspace,
            None => self.saved.push(workspace),
        }
        self.edited.remove(&id);
        self.current = id.clone();
        Ok(id)
    }

    /// Delete a saved workspace; the current one can't be deleted (Photoshop greys it).
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:239 `delete_workspace`.
    pub fn delete(&mut self, id: &str) -> Result<(), WorkspaceError> {
        let Some(i) = self.saved.iter().position(|w| w.id == id) else {
            return Err(WorkspaceError::UnknownId);
        };
        if self.current == id {
            return Err(WorkspaceError::CurrentCannotBeDeleted);
        }
        self.saved.remove(i);
        self.edited.remove(id);
        Ok(())
    }

    /// Reset the current workspace to its stored layout (the built-in's code layout).
    pub fn reset(&mut self) -> Option<&Workspace<P>> {
        self.edited.remove(&self.current);
        self.current().map(|w| &w.layout)
    }

    /// The Window › Workspace rows: built-ins, saved, separator, Reset, New…, Delete…,
    /// separator, Lock Workspace (check). The current one carries a check mark.
    // pc: crates/ui-egui/src/menu_catalog.rs:730-742 and workspace_ui.rs:84 `checked`.
    pub fn menu_entries(&self, labels: &WorkspaceLabels) -> Vec<MenuEntry<WorkspaceCommand>> {
        let mut entries: Vec<MenuEntry<WorkspaceCommand>> = self
            .builtin
            .iter()
            .chain(&self.saved)
            .map(|w| MenuItem::new(w.name.clone(), WorkspaceCommand::Select(w.id.clone())).check(w.id == self.current).into())
            .collect();
        entries.push(MenuEntry::Separator);
        entries.extend(self.command_rows(labels, |command| command));
        entries
    }
}

/// Lower-case alphanumerics joined by single dashes; the name itself when it has none.
fn slug(name: &str) -> String {
    let words: Vec<String> = name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect();
    if words.is_empty() { name.to_string() } else { words.join("-") }
}

/// Why a [`WorkspaceSet`] edit was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// No workspace has the id.
    UnknownId,
    /// A built-in workspace already has the name.
    NameTaken,
    /// The name is empty after trimming.
    EmptyName,
    /// The current workspace cannot be deleted.
    CurrentCannotBeDeleted,
}

/// The switcher at the right end of the options bar: a Sm select showing the current
/// workspace's name; its list has the same rows as the menu. Arrow keys move through the
/// rows, Enter picks, Escape closes.
///
/// The labels default to [`WorkspaceLabels::of`]; [`labels`](Self::labels) overrides them.
pub struct WorkspaceSwitcher<'a, P> {
    id: Id,
    set: &'a WorkspaceSet<P>,
    labels: Option<&'a WorkspaceLabels>,
}

/// What [`WorkspaceSwitcher::show`] returns.
pub struct WorkspaceSwitcherResponse {
    /// The select's response.
    pub response: Response,
    /// The command picked this pass, if any.
    pub command: Option<WorkspaceCommand>,
}

impl<'a, P: Clone + Eq + Hash> WorkspaceSwitcher<'a, P> {
    /// A switcher over `set`; `id_salt` keeps its state apart from other switchers.
    pub fn new(id_salt: impl AsIdSalt, set: &'a WorkspaceSet<P>) -> Self {
        Self { id: Id::unique("craft_ui::docking::WorkspaceSwitcher").with(id_salt), set, labels: None }
    }

    /// Uses these labels instead of the installed words.
    pub fn labels(mut self, labels: &'a WorkspaceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the switcher and returns the command the user picked, if any.
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:68 `switcher_options`; Photoshop's options bar switcher.
    pub fn show(self, ui: &mut Ui) -> WorkspaceSwitcherResponse {
        let Self { id, set, labels } = self;
        let installed;
        let labels = match labels {
            Some(labels) => labels,
            None => {
                installed = WorkspaceLabels::of(ui);
                &installed
            }
        };
        let mut value = set.current.clone();
        let commands: Vec<WorkspaceCommand> = set
            .command_rows(labels, |command| command)
            .into_iter()
            .filter_map(|entry| entry.item().and_then(|item| item.command().cloned()))
            .collect();
        let mut index = 0;
        let rows: Vec<MenuEntry<usize>> = set
            .command_rows(labels, |_| {
                index += 1;
                index - 1
            })
            .into_iter()
            .collect();
        let mut picked = None;
        let mut select = Select::new(id, labels.switcher.clone(), &mut value).size(ControlSize::Sm).fit(true);
        for w in set.builtin.iter().chain(&set.saved) {
            select = select.option(w.id.clone(), w.name.clone());
        }
        let edit = select.header_menu(labels.workspace.clone(), &rows, &mut picked).show(ui);
        let command = edit.committed().cloned().map(WorkspaceCommand::Select).or_else(|| picked.and_then(|i| commands.get(i).cloned()));
        WorkspaceSwitcherResponse { response: edit.response, command }
    }
}

crate::words! {
    /// Window › Workspace labels, English by default.
    #[non_exhaustive]
    pub struct WorkspaceLabels {
        workspace: "window.workspace" = "Workspace";
        reset: "window.workspace.resetWorkspace" = "Reset Workspace";
        new: "window.workspace.newWorkspace" = "New Workspace…";
        delete: "window.workspace.deleteWorkspace" = "Delete Workspace…";
        lock: "window.workspace.lockWorkspace" = "Lock Workspace";
        switcher: "window.workspace.switcher" = "Choose a workspace";
    }
}
