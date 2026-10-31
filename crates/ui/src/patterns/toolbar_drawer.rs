//! [`ToolbarDrawer`]: Edit Toolbar's All Tools drawer beside the tool rail, where tools move
//! between the toolbar and the drawer by dragging.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::hash::Hash;

use egui::{AsIdSalt, Id, Rect, Ui};

use super::rail::{RailTool, SlotId, ToolSlot};

/// A heading in the drawer and the tools under it ("Select", "Paint").
#[derive(Clone, Debug, PartialEq)]
pub struct DrawerGroup<T> {
    pub label: String,
    pub tools: Vec<RailTool<T>>,
}

/// A toolbar edit the user made in the drawer. The host applies it to its toolbar layout (the
/// [`ToolSlot`]s it passes to [`super::ToolRail`]) and stores the layout with its preferences.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ToolbarEdit<T> {
    /// A tool dragged from the drawer onto the toolbar: into slot `slot` at `index` of its tools
    /// (dropped onto a tool: joins its group), or as a new slot before slot index `before` (dropped
    /// between two slots).
    Add { tool: T, at: DropAt },
    /// A tool dragged from the toolbar into the drawer.
    Remove { tool: T },
    /// A tool dragged to another place in the toolbar.
    Move { tool: T, at: DropAt },
    /// "Restore Defaults" at the drawer's foot.
    Reset,
}

/// Where a dragged tool lands in the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAt {
    /// Into an existing slot's group, at this position of its tools.
    Slot { slot: SlotId, index: usize },
    /// As a new slot before the slot at this index (the slot count for the end).
    NewSlot { before: usize },
}

/// What [`ToolbarDrawer::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolbarDrawerOutput<T> {
    pub edits: Vec<ToolbarEdit<T>>,
    /// A tool clicked in the drawer: the host selects it, as a rail click would.
    pub select: Option<T>,
    /// Esc, a click outside, or the close button: the host stops showing the drawer.
    pub closed: bool,
}

/// The All Tools drawer, opened by the rail's Edit Toolbar button ([`super::ToolRail::edit_toolbar`],
/// [`super::ToolRailOutput::edit_toolbar`]) and drawn beside the rail while the host keeps it
/// open.
///
/// ```no_run
/// use craft_ui::patterns::{DrawerGroup, ToolbarDrawer, ToolSlot};
/// # fn demo(ui: &mut egui::Ui, rail_rect: egui::Rect, groups: &[DrawerGroup<&'static str>], slots: &[ToolSlot<&'static str>], open: &mut bool) {
/// let out = ToolbarDrawer::new("edit-toolbar").show(ui, rail_rect, groups, slots);
/// for edit in out.edits { /* apply to the toolbar layout, save it */ }
/// if out.closed { *open = false; }
/// # }
/// ```
///
/// - **Look:** a popup surface beside the rail (on its far side from the window edge), one column
///   of rows (icon, name, shortcut) under group headings; tools already in the toolbar are dimmed.
///   Restore Defaults at the foot.
/// - **Editing:** only while the drawer is open, as in Illustrator and Photoshop: a drawer row
///   dragged onto the rail adds the tool (between slots: a new slot; onto a slot: into its group);
///   a rail tool dragged into the drawer removes it; rail tools drag to new places. The drop
///   target shows as an insertion line or a slot highlight. The rail reads the drag from egui's
///   `DragAndDrop` payload, so the rail draws the targets.
/// - **Keyboard:** the drawer takes focus when it opens (first row); arrows move, Enter selects the
///   tool, Space picks a row up and arrows then choose its place on the rail, Enter drops, Esc
///   cancels the move or closes the drawer. Focus returns to the Edit Toolbar button on close.
/// - **AccessKit:** a `Dialog` named "Edit Toolbar"; rows are `ListItem`s with "in toolbar" as a
///   description when dimmed.
#[must_use = "show the drawer with `show`"]
pub struct ToolbarDrawer {
    id: Id,
    labels: Option<ToolbarDrawerLabels>,
}

impl ToolbarDrawer {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        todo!()
    }

    pub fn labels(mut self, labels: ToolbarDrawerLabels) -> Self {
        todo!()
    }

    /// Draws the drawer beside `rail` (the rail's rect from [`super::ToolRailOutput::rect`]) with
    /// every tool in `groups`, dimming those in `toolbar`.
    // vectorcraft: crates/ui-egui/src/edit_toolbar.rs:409 `drawer`, :165 `add`, :206 `remove`,
    // :274 `drop_target`, :223 `command` (`toolbar.addTool`, `toolbar.removeTool`, `toolbar.reset`).
    pub fn show<T: Clone + PartialEq + Hash + Send + Sync + 'static>(
        self,
        ui: &mut Ui,
        rail: Rect,
        groups: &[DrawerGroup<T>],
        toolbar: &[ToolSlot<T>],
    ) -> ToolbarDrawerOutput<T> {
        todo!()
    }
}

/// The host's toolbar layout `slots` after `edit`; [`ToolbarEdit::Reset`] gives `defaults`. A
/// slot left empty goes; a new slot gets the next free [`SlotId`].
// vectorcraft: crates/ui-egui/src/state.rs:697 `ToolbarEdits` (edits kept per toolbar; None is
// as shipped).
pub fn apply_toolbar_edit<T: Clone + PartialEq>(slots: &[ToolSlot<T>], edit: &ToolbarEdit<T>, defaults: &[ToolSlot<T>]) -> Vec<ToolSlot<T>> {
    todo!()
}

crate::words! {
    /// The drawer's labels, English by default.
    #[non_exhaustive]
    pub struct ToolbarDrawerLabels {
        title: "toolbarDrawer.title" = "Edit Toolbar";
        hint: "toolbarDrawer.hint" = "Drag tools to or from the toolbar.";
        in_toolbar: "toolbarDrawer.inToolbar" = "In toolbar";
        restore: "toolbarDrawer.restore" = "Restore Defaults";
        close: "toolbarDrawer.close" = "Close";
    }
}
