//! `PanelMenu`: the rows of a dock panel's menu (the button at the end of a group's tab strip and a tab's context
//! menu): the host's rows, then the dock's group rows.

use super::words::PanelMenuWords;
use super::{Menu, MenuItem};
use crate::contract::{Show as _, ShowContainer as _};
use crate::display::Divider;
use crate::runtime::commands::CommandRef;
use crate::runtime::env::Label;
use egui::Ui;

/// A row of the dock itself at the end of a panel menu; it goes back to the host as the pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroupItem {
    /// Float the panel into its own frame.
    Float,
    /// Move Panel To, the target at this index.
    MoveTo(usize),
    /// Close the panel.
    Close,
    /// Auto-Collapse Iconic Panels.
    AutoCollapse,
    /// Close Tab Group.
    CloseGroup,
    /// Minimize the group, or expand it when it is minimized.
    ToggleMinimized,
    /// Collapse to Icons.
    CollapseToIcons,
    /// Float Tab Group.
    FloatGroup,
    /// Move Group Up (`true`) or Down (`false`) within its column.
    StepGroup(bool),
    /// Move Group To, the target at this index.
    MoveGroupTo(usize),
    /// Close Other Panels in Group.
    CloseOthers,
    /// Maximize Frame, or restore it when the frame is maximized.
    ToggleMaximize,
}

/// One row of a panel menu.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PanelRow {
    /// A command row: its label, chord, check and enabled state come from the command, and it runs by itself.
    Command(CommandRef),
    /// A row of the dock, reported through `PanelMenu::show`.
    Item(GroupItem, Label),
    /// A row that opens a submenu.
    Submenu(Label, Vec<PanelRow>),
    /// A separator.
    Separator,
}

/// The rows of one panel's menu: the host's rows, a separator, then the dock's rows.
#[derive(crate::contract::Pattern)]
pub struct PanelMenu {
    host: Vec<PanelRow>,
    group: Vec<PanelRow>,
}

impl PanelMenu {
    /// A menu of the host's `host` rows followed by the dock's `group` rows.
    pub fn new(host: Vec<PanelRow>, group: Vec<PanelRow>) -> Self {
        PanelMenu { host, group }
    }

    /// Shows the rows in the menu body `ui` and returns the dock row picked this pass.
    pub fn show(&self, ui: &mut Ui) -> Option<GroupItem> {
        let mut picked = None;
        Self::rows(ui, &self.host, &mut picked);
        if !self.host.is_empty() && !self.group.is_empty() {
            Divider::horizontal().show(ui);
        }
        Self::rows(ui, &self.group, &mut picked);
        picked
    }

    fn rows(ui: &mut Ui, rows: &[PanelRow], picked: &mut Option<GroupItem>) {
        for row in rows {
            match row {
                PanelRow::Command(command) => {
                    MenuItem::command(command.clone()).show(ui);
                }
                PanelRow::Item(item, label) => {
                    if MenuItem::new(label.clone()).show(ui).clicked() {
                        *picked = Some(*item);
                    }
                }
                PanelRow::Submenu(label, rows) => {
                    let head = MenuItem::submenu(label.clone()).show(ui);
                    Menu::submenu(&head).show(ui, |ui, _| Self::rows(ui, rows, picked));
                }
                PanelRow::Separator => {
                    Divider::horizontal().show(ui);
                }
            }
        }
    }

    /// The dock's rows for a panel: Float when it can move and float, Move Panel To with one submenu row per target
    /// label, Close when it can close and the dock shows close rows.
    pub fn group_rows(movable: bool, floatable: bool, closable: bool, close_row: bool, targets: &[Label], words: &PanelMenuWords) -> Vec<PanelRow> {
        let mut rows = Vec::new();
        if movable && floatable {
            rows.push(PanelRow::Item(GroupItem::Float, Label::from(words.float.as_str().to_owned())));
        }
        if movable && !targets.is_empty() {
            let targets = targets.iter().enumerate().map(|(at, label)| PanelRow::Item(GroupItem::MoveTo(at), label.clone())).collect();
            rows.push(PanelRow::Submenu(Label::from(words.move_to.as_str().to_owned()), targets));
        }
        if closable && close_row {
            rows.push(PanelRow::Item(GroupItem::Close, Label::from(words.close.as_str().to_owned())));
        }
        rows
    }
}
