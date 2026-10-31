//! Context and panel menus built from the app's menu model: the same [`Item`] and [`Node`] rows
//! as the menu bar (commands, shortcuts from the keymap, check and radio states, the gate), drawn
//! by [`craft_ui::menu::ContextMenu`] or handed to a panel's menu
//! ([`craft_ui::docking::DockContent::panel_menu_entries`]).

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_keymap::{CommandRef, Keymap};
use craft_ui::menu::MenuEntry;
use egui::{Response, Ui};

use crate::bar::EnglishHook;
use crate::driver::Gate;
use crate::keymap::ItemState;
use crate::layout::Target;
use crate::modal::ModalRules;
use crate::model::Node;

/// A right-click menu of app commands, built from menu-model rows.
///
/// The app builds its rows once (or per target object) as [`Node`]s, the same type its menu bar
/// uses, so a context menu's Copy has the menu bar's label, shortcut and enabled state:
///
/// ```no_run
/// use craft_menu::{CommandContextMenu, Gate, Item, ItemState, Node};
/// # fn demo(ui: &mut egui::Ui, canvas: &egui::Response, keymap: &craft_keymap::Keymap, gate: Gate) {
/// let rows = vec![
///     Node::from(Item::new("edit.copy", "Copy")),
///     Node::from(Item::new("edit.paste", "Paste")),
///     Node::Separator,
///     Node::from(Item::new("layer.duplicate", "Duplicate Layer")),
/// ];
/// let state = |c: &craft_menu::CommandRef| ItemState { enabled: c.id != "edit.paste", checked: None };
/// if let Some(command) = CommandContextMenu::new(&rows).keymap(keymap).state(&state).gate(gate).show(ui, canvas) {
///     // Run `command` as a menu-bar click.
/// }
/// # }
/// ```
///
/// What the user sees and does is [`craft_ui::menu::ContextMenu`]'s: a secondary click opens it at
/// the pointer; the Menu key or Shift+F10 opens it under the focused widget with the first
/// enabled row highlighted; arrows, mnemonics, Enter and Esc work as in the menu bar's menus. Rows
/// the gate or the modal rules forbid are disabled. AppKit's own items ([`Node::Standard`]) are
/// left out, as in the in-window bar.
#[must_use = "show the menu with `show`, or take its rows with `entries`"]
pub struct CommandContextMenu<'a> {
    nodes: &'a [Node],
    gate: Gate,
    rules: Option<&'a ModalRules>,
    keymap: Option<&'a Keymap>,
    state: Option<&'a dyn Fn(&CommandRef) -> ItemState>,
    target: Target,
    english: Option<&'a EnglishHook<'a>>,
}

impl<'a> CommandContextMenu<'a> {
    /// The menu of `nodes`, every row enabled, shortcuts as the items carry them, on
    /// [`Target::current`].
    pub fn new(nodes: &'a [Node]) -> Self {
        todo!()
    }

    /// What may run: rows the gate forbids are disabled ([`Gate::allows`]).
    pub fn gate(mut self, gate: Gate) -> Self {
        todo!()
    }

    /// The per-command rules under the gate ([`Gate::allows_under`]).
    pub fn modal_rules(mut self, rules: &'a ModalRules) -> Self {
        todo!()
    }

    /// Shortcuts and toggle check items from the keymap, as [`crate::apply_keymap`] gives the
    /// menu bar.
    pub fn keymap(mut self, keymap: &'a Keymap) -> Self {
        todo!()
    }

    /// Enabled and checked per command, as [`crate::apply_state`] gives the menu bar.
    pub fn state(mut self, state: &'a dyn Fn(&CommandRef) -> ItemState) -> Self {
        todo!()
    }

    /// The platform, for the shortcut column (⌘ glyphs on a Mac, Ctrl names elsewhere).
    pub fn target(mut self, target: Target) -> Self {
        todo!()
    }

    /// English labels for rows the app shows translated, as [`crate::MenuBarUi::english`].
    pub fn english(mut self, english: &'a EnglishHook<'a>) -> Self {
        todo!()
    }

    /// The rows as library menu entries, with keymap, state, gate and rules applied, for a
    /// panel menu ([`craft_ui::docking::DockContent::panel_menu_entries`] maps the commands to
    /// ids) or a host's own [`craft_ui::menu::Menu`].
    // pc: crates/ui-egui/src/menus/render.rs (rows from catalog items with `is_enabled` and
    // `checked`); the conversion is `bar::Convert::entries`, reused here.
    pub fn entries(&self) -> Vec<MenuEntry<CommandRef>> {
        todo!()
    }

    /// Shows the menu on `widget` ([`craft_ui::menu::ContextMenu::show`]) and returns the command
    /// chosen this frame. A built-in command is handled as the menu bar handles it (the built-in
    /// Quit asks the window to close).
    // pc: crates/ui-egui/src/panels/layers_panel/context_menu.rs (the Layers panel's right-click
    // rows run through `menus::invoke`); vectorcraft: context menus over command ids.
    pub fn show(self, ui: &Ui, widget: &Response) -> Option<CommandRef> {
        todo!()
    }
}
