//! The panel-menu host: every panel's "…" menu (the ≡ at the right end of a group's tab strip
//! and a tab's context menu) is a library [`crate::menu::Menu`] built from the entries the host returns,
//! followed by the group items (Float, Move Panel To, Close). One host change serves every
//! panel of every app (gaps 14 and 65).

use craft_theme::ControlSize;
use egui::{Response, Ui};

use crate::menu::{ContextMenu, MenuEntry, MenuItem, MenuResponse, PressMenu};

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
    pub group_entries: Vec<MenuEntry<GroupItem>>,
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
    /// Close Tab Group.
    CloseGroup,
    /// Minimize the group, or expand it when it is minimized.
    ToggleMinimized,
    /// Collapse to Icons (docked groups).
    CollapseToIcons,
    /// Float Tab Group.
    FloatGroup,
    /// Move Group Up (`true`) or Down (`false`) within its column.
    StepGroup(bool),
    /// Move Group To › `index` into the group's targets.
    MoveGroupTo(usize),
    /// Close Other Panels in Group.
    CloseOthers,
    /// Maximize Frame, or restore it when the frame is maximized.
    ToggleMaximize,
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
        let entries = self.combined();
        PressMenu::new(&entries).size(ControlSize::Sm).show(ui, button).chosen
    }

    /// The same menu as a context menu of `widget` (a tab): a secondary click, the Menu key or
    /// Shift+F10 opens it, and it returns what the user picked.
    pub fn show_context(self, ui: &Ui, widget: &Response) -> Option<PanelMenuChoice> {
        let entries = self.combined();
        ContextMenu::new(&entries).size(ControlSize::Sm).show(ui, widget).chosen
    }

    /// The host's rows, a separator, then the dock's rows.
    fn combined(&self) -> Vec<MenuEntry<PanelMenuChoice>> {
        let mut all: Vec<_> =
            self.entries.iter().map(|entry| entry.map(&|command: &PanelCommand| PanelMenuChoice::Command(command.clone()))).collect();
        all.push(MenuEntry::Separator);
        all.extend(self.group_entries.iter().map(|entry| entry.map(&|item: &GroupItem| PanelMenuChoice::Group(*item))));
        all
    }

    /// The group items for `policy`: Float when movable and floatable, Move Panel To with one
    /// submenu row per target label, Close when closable and the config shows close items.
    // new: the rules of render/group.rs:492 `panel_menu_items` as data.
    pub fn group_entries(policy: super::Permissions, targets: &[String], close_item: bool, labels: &PanelMenuLabels) -> Vec<MenuEntry<GroupItem>> {
        let mut items = Vec::new();
        if policy.movable && policy.floatable {
            items.push(MenuItem::new(labels.float.clone(), GroupItem::Float).into());
        }
        if policy.movable && !targets.is_empty() {
            let rows = targets.iter().enumerate().map(|(index, label)| MenuItem::new(label.clone(), GroupItem::MoveTo(index)).into()).collect();
            items.push(MenuItem::submenu(labels.move_to.clone(), rows).into());
        }
        if policy.closable && close_item {
            items.push(MenuItem::new(labels.close.clone(), GroupItem::Close).into());
        }
        items
    }
}

crate::words! {
    /// The dock's own panel, tab-group, column and frame menu rows, English by default. The only
/// source of these labels.
    #[non_exhaustive]
    pub struct PanelMenuLabels {
        float: "dock.panelMenu.float" = "Float";
        move_to: "dock.panelMenu.moveTo" = "Move Panel To";
        close: "dock.panelMenu.close" = "Close";
        auto_collapse: "dock.panelMenu.autoCollapse" = "Auto-Collapse Iconic Panels";
        close_group: "dock.panelMenu.closeGroup" = "Close Tab Group";
        minimize: "dock.panelMenu.minimize" = "Minimize";
        expand: "dock.panelMenu.expand" = "Expand";
        float_group: "dock.panelMenu.floatGroup" = "Float Tab Group";
        move_group_up: "dock.panelMenu.moveGroupUp" = "Move Group Up";
        move_group_down: "dock.panelMenu.moveGroupDown" = "Move Group Down";
        move_group_to: "dock.panelMenu.moveGroupTo" = "Move Group To";
        collapse_to_icons: "dock.panelMenu.collapseToIcons" = "Collapse to Icons";
        expand_panels: "dock.panelMenu.expandPanels" = "Expand Panels";
        close_others: "dock.panelMenu.closeOthers" = "Close Other Panels in Group";
        maximize: "dock.panelMenu.maximize" = "Maximize Frame";
        restore_frame: "dock.panelMenu.restoreFrame" = "Restore Frame Size";
    }
}

impl PanelMenuLabels {
    /// The labels of the installed words, shared between frames: the one source of every dock
    /// menu row.
    pub(crate) fn in_dock(ui: &Ui) -> std::sync::Arc<Self> {
        <Self as crate::localized::SharedLabels>::shared(ui)
    }
}

/// Response of the host-side popup for hosts that draw a [`crate::menu::Menu`] over a widget of their own
/// (a preferences button's right-click menu, gap 65). Thin wrapper so the host needs no
/// `egui::Popup` of its own.
// new: the Popup bookkeeping of crate::menu::Menu::show for a caller that has only a Response.
pub fn menu_under<T: Clone>(ui: &Ui, widget: &Response, entries: &[MenuEntry<T>]) -> MenuResponse<T> {
    PressMenu::new(entries).size(ControlSize::Sm).show(ui, widget)
}
