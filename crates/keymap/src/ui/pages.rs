//! What the Keyboard Shortcuts dialog adds for Photoshop's "Keyboard Shortcuts and Menus" (gap
//! 81): the Shortcuts For areas, the app's own tabs beside the shortcuts (PhotoCraft's Menus and
//! Toolbar), and the dialog's state as automation reads it.
//!
//! Undo, Take It and Go to Conflict, the Set row and Summarize… are the dialog's already; the Set
//! row's `.kys` export is an [`super::Options::export_formats`] entry the app writes with
//! [`crate::import::kys::write`].

#![expect(unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use serde::Serialize;

use super::{Area, DialogState, ShortcutsDialog};
use crate::command::CommandRef;

impl Area {
    /// Photoshop's Shortcuts For entries over the catalog: Application Menus (commands with a
    /// menu path in a top menu), Panel Menus (commands whose home scope is a panel) and Tools
    /// ([`crate::Kind::Tool`]). With [`super::Options::area_tabs`] off they are the Show
    /// dropdown's entries, as Photoshop's dropdown.
    // pc: crates/ui-egui/src/prefs/prefs_ui/shortcuts.rs:182 (its tabs); Photoshop's "Shortcuts
    // For: Application Menus / Panel Menus / Tools".
    pub fn photoshop() -> Vec<Area> {
        todo!()
    }
}

/// A tab of the app's own beside Keyboard Shortcuts (PhotoCraft's Menus tab, which hides and
/// colours menu items, and its Toolbar tab). The dialog draws a tab strip when there is one;
/// OK and Cancel apply to every tab, so the body edits the app's own draft and the app commits
/// it on [`super::Outcome::Ok`].
pub struct DialogTab<'a> {
    /// The automation id (`menus`, `toolbar`).
    pub id: &'static str,
    /// The tab's label, translated by the app.
    pub label: String,
    /// Draws the tab's body in the dialog's body rect; returns true when it changed something
    /// (the dialog then reports [`super::Outcome::Changed`]).
    pub body: Box<dyn FnMut(&mut egui::Ui) -> bool + 'a>,
}

impl<'a> ShortcutsDialog<'a> {
    /// The app's tabs after Keyboard Shortcuts, in this order. The selected tab is kept in the
    /// [`DialogState`]; Ctrl+Tab and Ctrl+Shift+Tab move between tabs (Ctrl+Page Down/Up on
    /// Windows and Linux too).
    // pc: crates/ui-egui/src/prefs/prefs_ui/shortcuts.rs:182 (Keyboard Shortcuts, Menus, Toolbar).
    pub fn tabs(mut self, tabs: &'a mut [DialogTab<'a>]) -> Self {
        todo!()
    }
}

/// The dialog's state as automation and tests read it: what a control channel reports for
/// `ui.shortcuts.state` without reading pixels.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DialogSnapshot {
    /// The tab shown: `shortcuts` or an app tab's id.
    pub tab: String,
    /// The Show entry or Shortcuts For area.
    pub area: String,
    pub query: String,
    pub selected: Option<CommandRef>,
    /// Commands whose bindings differ from the defaults, with their binding texts.
    pub overrides: Vec<(CommandRef, Vec<String>)>,
    /// The conflict bar's chord and the commands holding it, while it shows.
    pub conflict: Option<(String, Vec<CommandRef>)>,
    /// Whether a chord is being recorded, and the command it is for.
    pub recording: Option<CommandRef>,
    /// The active set's name, with sets.
    pub set: Option<String>,
    pub can_undo: bool,
}

impl DialogState {
    /// The state as [`DialogSnapshot`]; `None` before the dialog's first frame.
    // pc: crates/ui-egui/src/prefs/prefs_ui/shortcuts.rs:53 `conflict_state` (overrides,
    // alternates, selection and set, hashed for its tests).
    pub fn snapshot(&self, keymap: &crate::Keymap) -> Option<DialogSnapshot> {
        todo!()
    }
}
