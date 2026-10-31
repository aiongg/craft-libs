//! The panel-menu host: every panel's "…" menu (the ≡ at the right end of a group's tab strip
//! and a tab's context menu) is a library [`crate::menu::Menu`] built from the entries the host returns,
//! followed by the group items (Float, Move Panel To, Close). One host change serves every
//! panel of every app (gaps 14 and 65).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Response, Ui};

use crate::menu::{MenuEntry, MenuResponse};

/// A command of a panel menu. Apps use their command ids (`"swatches.newSwatch"`); the id goes
/// back to the host in [`super::DockEvent::PanelCommand`].
pub type PanelCommand = String;

/// The entries of one panel's menu in one pass: the host's rows first, then the dock's group
/// items. The host supplies the rows through [`super::DockContent::panel_menu_entries`]; the
/// renderer appends the group items and shows the whole as one [`Menu`](crate::menu::Menu).
pub struct PanelMenu<'a> {
    /// The host's rows (checks, radios, submenus, separators), already translated.
    pub entries: &'a [MenuEntry<PanelCommand>],
    /// The dock's own group items after a separator; empty when the group is locked.
    pub group_items: Vec<MenuEntry<GroupItem>>,
}

/// The dock's rows at the end of every panel menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupItem {
    /// Float the panel into its own frame.
    Float,
    /// Move Panel To › `index` into the group's move targets.
    MoveTo(usize),
    /// Close the panel.
    Close,
    /// Auto-Collapse Iconic Panels (column menus; the `[D]` rule of `DockEvent::ToggleAutoCollapse`).
    AutoCollapse,
}

/// What the user chose in a panel menu this pass.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelMenuChoice {
    /// One of the host's rows.
    Command(PanelCommand),
    /// One of the dock's rows.
    Group(GroupItem),
}

impl PanelMenu<'_> {
    /// Opens the menu under `button` (the ≡ button or a tab's right click) in its own popup,
    /// sized `ControlSize::Sm`, and returns what the user
    /// picked. Arrow keys, Enter, Escape and mnemonics are the library menu's; a check or
    /// radio row closes the menu (Photoshop's panel menus do), a submenu opens on hover and
    /// Right. Focus returns to the button on close.
    // pc: crates/ui-egui/src/panels/panel_docking.rs:826 `body` (the popup PhotoCraft fills
    // per panel) and crates/ui-egui/src/panels/swatches_panel.rs `panel_menu`; the dock rows
    // come from crates/ui/src/docking/render/group.rs:492 `panel_menu_items`.
    pub fn show(self, ui: &Ui, button: &Response) -> Option<PanelMenuChoice> {
        todo!()
    }

    /// The group items for `policy`: Float when movable and floatable, Move Panel To with one
    /// submenu row per target label, Close when closable and the config shows close items.
    // new: the rules of render/group.rs:492 `panel_menu_items` as data.
    pub fn group_items(policy: super::Permissions, targets: &[String], close_item: bool, labels: &PanelMenuLabels) -> Vec<MenuEntry<GroupItem>> {
        todo!()
    }
}

crate::words! {
    /// The dock's own panel-menu rows, English by default.
    #[non_exhaustive]
    pub struct PanelMenuLabels {
        float: "dock.panelMenu.float" = "Float";
        move_to: "dock.panelMenu.moveTo" = "Move Panel To";
        close: "dock.panelMenu.close" = "Close";
        auto_collapse: "dock.panelMenu.autoCollapse" = "Auto-Collapse Iconic Panels";
    }
}

/// Response of the host-side popup for hosts that draw a [`crate::menu::Menu`] over a widget of their own
/// (a preferences button's right-click menu, gap 65). Thin wrapper so the host needs no
/// `egui::Popup` of its own.
// new: the Popup bookkeeping of crate::menu::Menu::show for a caller that has only a Response.
pub fn menu_under<T: Clone>(ui: &Ui, widget: &Response, entries: &[MenuEntry<T>]) -> MenuResponse<T> {
    todo!()
}
