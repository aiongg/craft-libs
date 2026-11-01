//! The Brush Settings section list ([`BrushSectionList`]).
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::labels::BrushSettingsLabels;

/// What a section row does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrushSectionKind {
    /// A section with a pane of controls (Brush Tip Shape, Shape Dynamics, …, Brush Pose): a click
    /// shows its pane.
    #[default]
    Pane,
    /// A switch with no pane (Noise, Wet Edges, Build-up, Smoothing, Protect Texture): a click
    /// toggles it and the shown pane stays.
    Switch,
}

/// One row of the section list, built by the app each pass from its brush settings.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushSection<K> {
    /// The key events name the section by.
    pub key: K,
    /// The section's name, shown and read by a screen reader.
    pub name: String,
    /// Pane or switch.
    pub kind: BrushSectionKind,
    /// The enable box's state (a switch's state); `None`: no box (Brush Tip Shape).
    pub enabled: Option<bool>,
    /// The lock's state; `None`: the row has no lock.
    pub locked: Option<bool>,
}

impl<K> BrushSection<K> {
    /// A pane section without an enable box or lock.
    pub fn pane(key: K, name: impl Into<String>) -> Self {
        Self { key, name: name.into(), kind: BrushSectionKind::Pane, enabled: None, locked: None }
    }

    /// A switch row, on or off.
    pub fn switch(key: K, name: impl Into<String>, on: bool) -> Self {
        Self { key, name: name.into(), kind: BrushSectionKind::Switch, enabled: Some(on), locked: None }
    }

    /// Gives the row an enable box in this state.
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = Some(on);
        self
    }

    /// Gives the row a lock in this state.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = Some(locked);
        self
    }

    /// Whether the section is off: it has a box and the box is clear. Its pane shows greyed.
    pub fn is_off(&self) -> bool {
        self.enabled == Some(false)
    }
}

/// What the section list reports: one-shot changes, in the order they happened. The app applies
/// them to its brush settings as tool options (no history state).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrushSectionEvent<K> {
    /// A pane row was clicked (or Enter, or the lead moved onto it): show its pane.
    Select(K),
    /// A pane section's enable box changed, or a click on its row turned it on.
    Enable {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// A section's lock changed: a locked section keeps its settings when a preset is picked.
    Lock {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// A switch row changed.
    Switch {
        /// The section.
        key: K,
        /// The new state.
        on: bool,
    },
    /// The "Brushes" link row was clicked: open the Brushes panel.
    OpenLink,
}

/// Photoshop's Brush Settings section list: an optional "Brushes" link row, the pane rows with
/// their enable boxes and locks, and after a gap the switch rows. A
/// [`List`](craft_ui::list::List) of [`ListRow`](craft_ui::list::ListRow)s with a leading
/// [`ToggleColumn::check_box`](craft_ui::list::ToggleColumn::check_box) and a lock column
/// [`at_end`](craft_ui::list::ToggleColumn::at_end) and
/// [`always_visible`](craft_ui::list::ToggleColumn::always_visible).
///
/// - **Look.** `brush.section_list_width` (150) wide, rows `brush.section_row` (23) high; the
///   enable box `brush.section_box` (13); names from `brush.section_name_inset` (20), truncated
///   with a tooltip, Brush Tip Shape in the strong weight; the lock icon (`brush.section_lock`:
///   12 in an 18 pt hit area) at the row's end, `text.default` when locked and `text.faint` when
///   open; 4 pt after the first row and `brush.section_switch_gap` (6) before the first switch
///   row; the link row, a hairline and 3 pt gaps above the sections. Selected row
///   `list.selected`, hover `list.hover`.
/// - **Pointer.** A click on a pane row reports `Select` and, if its box is clear, `Enable { on:
///   true }`; the box toggles only the flag; the lock toggles only the lock; a click on a switch row
///   toggles it (`Switch`). A press on the row (not its lock) shows the box pressed.
/// - **Keys.** One Tab stop. Up and Down, Home and End move the lead (onto a pane row: also
///   `Select`); Space toggles the lead's box or switch; Shift+Space its lock; Enter selects a pane
///   row and turns it on.
/// - **AccessKit.** A list named "Brush settings sections"; a pane row is a `Button` with
///   `selected`; its box a `CheckBox` "Enable {name}"; its lock a `Button` with `selected` named
///   "Lock: {name}"; a switch row one `CheckBox`.
/// - **RTL.** Mirrors: the box at the reading start, the lock at the reading end.
#[must_use = "show the list with `show`"]
pub struct BrushSectionList<'a, K> {
    id_salt: IdSalt,
    sections: &'a [BrushSection<K>],
    selected: Option<&'a K>,
    link: bool,
    labels: Option<BrushSettingsLabels>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> BrushSectionList<'a, K> {
    /// The list of `sections`, in order (pane rows first, then switch rows).
    pub fn new(id_salt: impl AsIdSalt, sections: &'a [BrushSection<K>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), sections, selected: None, link: false, labels: None }
    }

    /// The pane shown: its row draws selected.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Shows the "Brushes" link row at the top (Photoshop; reports `OpenLink`).
    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the list.
    pub fn show(self, ui: &mut Ui) -> BrushSectionListResponse<K> {
        // pc: crates/ui-egui/src/panels/brush_panel.rs:247-349 `section_list` (row 23, box 13 at +6, names at +20, lock 18 at the end, 6 pt gap, link row)
        todo!()
    }
}

/// What [`BrushSectionList::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct BrushSectionListResponse<K> {
    /// The list.
    pub response: Response,
    /// This pass's changes, in order.
    pub events: Vec<BrushSectionEvent<K>>,
}
