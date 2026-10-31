//! Named workspaces: Photoshop's Window › Workspace and the switcher at the right end of the
//! options bar. A workspace is a saved [`Workspace`] layout plus which bars and the panel set
//! show; the set holds the app's built-in workspaces, the user's saved ones, the current one and
//! Lock Workspace.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::hash::Hash;

use egui::{Response, Ui};

use super::Workspace;
use crate::menu::MenuEntry;

/// Chrome a workspace remembers beside the dock layout: which bars show and the panel set.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct WorkspaceChrome {
    pub menu_bar: bool,
    pub tool_rail: bool,
    pub options_bar: bool,
    pub status_bar: bool,
}

/// One named workspace.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedWorkspace<P> {
    /// Stable id (`"essentials"`, or a slug of a saved name).
    pub id: String,
    /// What the menus and the switcher show.
    pub name: String,
    pub layout: Workspace<P>,
    pub chrome: WorkspaceChrome,
    /// Whether Keyboard Shortcuts and Menus were saved with it (Photoshop's New Workspace boxes).
    pub keeps_shortcuts: bool,
    pub keeps_menus: bool,
}

/// The app's workspaces: built-ins first, saved ones after, one current. The fill task gives it
/// serde (a settings section) once `Workspace` derives it unconditionally; the layouts of
/// built-ins are code, only their user edits are stored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkspaceSet<P> {
    pub builtin: Vec<NamedWorkspace<P>>,
    pub saved: Vec<NamedWorkspace<P>>,
    /// Id of the current workspace.
    pub current: String,
    /// Lock Workspace: panels can't be moved, closed or resized; the dock is shown locked.
    pub locked: bool,
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
    LockToggle,
}

impl<P: Clone + Eq + Hash> WorkspaceSet<P> {
    pub fn new(builtin: Vec<NamedWorkspace<P>>) -> Self {
        todo!()
    }

    pub fn current(&self) -> Option<&NamedWorkspace<P>> {
        todo!()
    }

    /// Switch to `id`; the current workspace's live layout is kept under its id first (Photoshop
    /// remembers the state of each workspace until Reset).
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:252 `select_workspace`.
    pub fn select(&mut self, id: &str, live: Workspace<P>) -> Result<(), WorkspaceError> {
        todo!()
    }

    /// Save `live` under `name` and make it current; a name a built-in has is refused.
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:207 `new_workspace`.
    pub fn save(&mut self, name: &str, live: Workspace<P>, chrome: WorkspaceChrome) -> Result<String, WorkspaceError> {
        todo!()
    }

    /// Delete a saved workspace; the current one can't be deleted (Photoshop greys it).
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:239 `delete_workspace`.
    pub fn delete(&mut self, id: &str) -> Result<(), WorkspaceError> {
        todo!()
    }

    /// Reset the current workspace to its stored layout (the built-in's code layout).
    pub fn reset(&mut self) -> Option<&Workspace<P>> {
        todo!()
    }

    /// The Window › Workspace rows: built-ins, saved, separator, Reset, New…, Delete…,
    /// separator, Lock Workspace (check). The current one carries a check mark.
    // pc: crates/ui-egui/src/menu_catalog.rs:730-742 and workspace_ui.rs:84 `checked`.
    pub fn menu_entries(&self, labels: &WorkspaceLabels) -> Vec<MenuEntry<WorkspaceCommand>> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceError {
    UnknownId,
    NameTaken,
    EmptyName,
    CurrentCannotBeDeleted,
}

/// The switcher at the right end of the options bar: a Sm select showing the current
/// workspace's name; its list has the same rows as the menu. Arrow keys move through the
/// rows, Enter picks, Escape closes.
pub struct WorkspaceSwitcher<'a, P> {
    set: &'a WorkspaceSet<P>,
    labels: &'a WorkspaceLabels,
}

impl<'a, P: Clone + Eq + Hash> WorkspaceSwitcher<'a, P> {
    pub fn new(set: &'a WorkspaceSet<P>, labels: &'a WorkspaceLabels) -> Self {
        Self { set, labels }
    }

    /// Returns the command picked this pass, if any.
    // pc: crates/ui-egui/src/menus/workspace_ui.rs:68 `switcher_options`; Photoshop's options bar switcher.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<WorkspaceCommand>) {
        todo!()
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
