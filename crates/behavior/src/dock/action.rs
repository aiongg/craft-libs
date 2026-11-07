//! Where a panel goes ([`Placement`]), where it was ([`Location`]) and the transaction unit ([`Action`]).

use super::{
    groups::{Group, GroupLocation, Target},
    node::{Monitor, Zone},
    split::SplitSize,
};

/// Where a moved panel goes relative to an anchor panel.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Placement<P> {
    /// Insert before this existing destination member, or append. Also reorders a group.
    Tab {
        /// The member to insert before; `None` appends.
        before: Option<P>,
    },
    /// Center inserts after the anchor; sides split the anchor's frame into two halves, and
    /// every other frame keeps its size. Sides apply to `Tabs` and `Stack` nodes only.
    Split(Zone),
    /// A side [`Placement::Split`] that gives the new frame its own room instead of half the
    /// anchor's frame: the new split takes `size`, and `outer` replaces the size of the nearest
    /// enclosing split whose divider runs along the anchor frame's `zone` side, so the frames
    /// beyond that divider give the room. An anchor at the edge of its area on that side takes
    /// the nearest such split along its opposite side instead. The renderer offers it beside a
    /// frame too small to halve. `Center`, and an anchor without either divider or with a dock
    /// beyond it, are invalid.
    SplitSized {
        /// The side of the anchor frame.
        zone: Zone,
        /// Size of the new split.
        size: SplitSize,
        /// New size of the enclosing split that gives up the room.
        outer: SplitSize,
    },
    /// A new group above or below the anchor's group in a dock column. On an anchor in a
    /// floating window, a split of the anchor's group above or below.
    /// `height` is the inserted group's preferred height ([`Group::height`]); `None`
    /// keeps a moved group's own height, or shares the free height for a new group.
    Group {
        /// Places the group below the anchor's group; false places it above.
        below: bool,
        /// Preferred height in points; `None` shares the free height.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
    },
    /// A new column before (towards the window edge) or after the anchor's column.
    Column {
        /// Places the new column after the anchor's column; false places it before.
        after: bool,
    },
    /// A new frame along one edge of the docked layout, a quarter of its width or height: the
    /// root is split, or in a layout with docks, the part between them. The anchor names any
    /// docked panel. A protected anchor ([`Permissions::is_protected`], applied through
    /// [`Layout::apply_with_permissions`]) narrows the part to the canvas column: where neither
    /// side of a side-by-side split is a dock and only one holds a protected panel, that side;
    /// for `Top` (`Bottom`), also the first (last) row of a split of rows when only that row
    /// holds one. `Center` is invalid.
    Edge(Zone),
}

/// A closed panel's former placement. Applications may retain this beside visibility flags.
/// Anchors are stable panel identities rather than mutable tree indices.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "P: serde::Deserialize<'de>"))]
#[non_exhaustive]
pub struct Location<P> {
    /// The window rect `[x, y, width, height]` when the panel was floating.
    pub floating: Option<[f32; 4]>,
    /// The panel the placement is relative to.
    pub anchor: Option<P>,
    /// Where the panel sat relative to `anchor`.
    pub placement: Placement<P>,
    /// The size of the split the panel's frame was in, when it had one.
    pub size: Option<SplitSize>,
    /// Accordion state to restore if the destination is still an accordion.
    pub stack: Option<(bool, Option<f32>)>,
    /// A split's complete sibling subtree. This retains placement around a nested workspace,
    /// rather than accidentally inserting beside only its first leaf when restoring.
    #[serde(default)]
    pub sibling: Vec<P>,
    /// Group and column state when the panel was in a dock. Applies when
    /// restoring creates a new group, column or dock for the panel.
    #[serde(default)]
    pub group: Option<GroupLocation>,
}

/// Groups, columns and floating windows are named by a panel they contain.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Action<P> {
    /// Move one panel next to `anchor`.
    Move {
        /// The panel the action applies to.
        panel: P,
        /// The panel the destination is relative to.
        anchor: P,
        /// Where the panel goes relative to the anchor.
        placement: Placement<P>,
    },
    /// Float the panel's group in a window at `rect`.
    Float {
        /// The panel the action applies to.
        panel: P,
        /// The window rect `[x, y, width, height]` in logical points.
        rect: [f32; 4],
    },
    /// Close a panel.
    Close {
        /// The panel the action applies to.
        panel: P,
    },
    /// Select a panel in its group.
    Activate {
        /// The panel the action applies to.
        panel: P,
    },
    /// Reveal a registered, currently absent panel. Applications validate their panel registry.
    /// `anchor` names the group it joins as the last tab. Without one it becomes the root of an
    /// empty layout, else joins the first dock group that accepts tabs and is shown (not
    /// minimized, not in an iconic column; the end-side dock's first), else gets a new frame a
    /// quarter wide at the right edge of the docked area
    /// ([`Placement::Edge`]).
    Open {
        /// The panel the action applies to.
        panel: P,
        /// The panel the destination is relative to.
        anchor: Option<P>,
    },
    /// Reveal an absent panel beside a protected canvas or at another explicit destination.
    OpenAt {
        /// The panel the action applies to.
        panel: P,
        /// The panel the destination is relative to.
        anchor: P,
        /// Where the panel goes relative to the anchor.
        placement: Placement<P>,
    },
    /// Reposition/resize a floating window, identified by one of its panels.
    MoveFloating {
        /// The panel the action applies to.
        panel: P,
        /// The window rect `[x, y, width, height]` in logical points.
        rect: [f32; 4],
    },
    /// Path from the root: false is the first child, true the second.
    ResizeSplit {
        /// Path from the root: false is the first child, true the second.
        path: Vec<bool>,
        /// The new size.
        size: SplitSize,
    },
    /// [`Action::ResizeSplit`] in the floating window holding `panel`, with a path from its
    /// root.
    ResizeWindowSplit {
        /// The panel the action applies to.
        panel: P,
        /// Path from the root: false is the first child, true the second.
        path: Vec<bool>,
        /// The new size.
        size: SplitSize,
    },
    /// Open or close the panel's entry in its accordion stack.
    SetStackOpen {
        /// The panel the action applies to.
        panel: P,
        /// The section shows its body.
        open: bool,
    },
    /// Set the panel's accordion entry height in points; `None` clears it.
    ResizeStack {
        /// The panel the action applies to.
        panel: P,
        /// Preferred height in points; `None` shares the free height.
        height: Option<f32>,
    },
    /// Move the group containing `panel` with its order, active tab and minimized state.
    /// `Target::At` with `Tab` or `Split(Center)` merges every member. An accordion entry is a
    /// group of one. A target inside the group is a no-op.
    MoveGroup {
        /// The panel the action applies to.
        panel: P,
        /// The destination.
        to: Target<P>,
    },
    /// Move the dock group containing `panel` one place up or down its column, past the
    /// neighbouring group. Fails for a group that is not docked in a column
    /// ([`DockError::InvalidPlacement`]), at the column's end ([`DockError::MissingTarget`]), or
    /// beside a group that cannot move ([`DockError::NotAllowed`]; a canvas is never stepped past).
    StepGroup {
        /// The panel the action applies to.
        panel: P,
        /// Moves towards the start; false moves towards the end.
        up: bool,
    },
    /// Move the dock column containing `panel`. `Float` makes a window of its groups.
    MoveColumn {
        /// The panel the action applies to.
        panel: P,
        /// The destination.
        to: Target<P>,
    },
    /// Dock or merge the floating window containing `panel`, every group included.
    MoveWindow {
        /// The panel the action applies to.
        panel: P,
        /// The destination.
        to: Target<P>,
    },
    /// Close Tab Group.
    CloseGroup {
        /// The panel the action applies to.
        panel: P,
    },
    /// × on a floating window: closes all its panels.
    CloseWindow {
        /// The panel the action applies to.
        panel: P,
    },
    /// Close Other Panels in Group.
    CloseOthers {
        /// The panel the action applies to.
        panel: P,
    },
    /// Minimize a dock group to its tab strip, or the floating window holding the panel to its
    /// title bar (and the strip of its only group). On an accordion entry, closes it.
    SetMinimized {
        /// The panel the action applies to.
        panel: P,
        /// Only the title bar or tab strip shows.
        minimized: bool,
    },
    /// Collapse the panel's dock column to icons, or expand it.
    SetIconic {
        /// The panel the action applies to.
        panel: P,
        /// Collapses the column to an icon strip.
        iconic: bool,
    },
    /// Set the expanded width in points of the panel's dock column.
    ResizeColumn {
        /// The panel the action applies to.
        panel: P,
        /// Width in points.
        width: f32,
    },
    /// Preferred height of a dock group or of an accordion entry.
    ResizeGroup {
        /// The panel the action applies to.
        panel: P,
        /// Preferred height in points; `None` shares the free height.
        height: Option<f32>,
    },
    /// Set the width in points of the panel's dock column while iconic; `None` clears it.
    ResizeIconic {
        /// The panel the action applies to.
        panel: P,
        /// Width in points.
        width: Option<f32>,
    },
    /// Put the floating window in its own OS window at `rect`, or back in the app window.
    SetNative {
        /// The panel the action applies to.
        panel: P,
        /// The window rect `[x, y, width, height]` in logical points.
        rect: Option<[f32; 4]>,
    },
    /// Record the monitor of the native window holding `panel`: from the application's
    /// monitor list, or else only the size egui reported.
    SetMonitor {
        /// The panel the action applies to.
        panel: P,
        /// The monitor holding the window.
        monitor: Option<Monitor>,
        /// Size of the monitor holding the window, when no monitor is known.
        monitor_size: Option<[f32; 2]>,
    },
    /// Move one panel, without the rest of its group. `Target::At` is [`Action::Move`];
    /// `NewDock` and `Float` give the panel a group of its own.
    MovePanel {
        /// The panel the action applies to.
        panel: P,
        /// The destination.
        to: Target<P>,
    },
}

impl<P> Location<P> {
    /// A location relative to `anchor` by `placement`, with no recorded sizes or siblings.
    pub fn new(anchor: Option<P>, placement: Placement<P>) -> Self {
        Self { floating: None, anchor, placement, size: None, stack: None, sibling: Vec::new(), group: None }
    }
}
