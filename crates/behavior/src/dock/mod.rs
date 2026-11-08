//! The geometry of a dock: a tree of splits over groups, laid out in a rect, with the split handles between
//! children and the side of a group a drop lands on (20.6).
//!
//! The tree and its rects are in reading order: a split's first child is its start (or top) child. A caller under
//! right-to-left maps each rect with `Flow::to_physical`, which mirrors the workspace.
//!
//! The panel layout of a dock ([`Layout`]) is the persisted model: docked tree, floating windows, the actions that
//! edit it and the rules that validate it.
//!
//! The query surface a renderer reads: [`Layout::place`] and [`Layout::places`] (where each panel sits),
//! [`Layout::node_of`], [`Layout::column`] and [`Layout::group`] (the container at a place), [`Layout::dock_path`],
//! [`Layout::edge_path`], [`Layout::edge_narrows`] and [`Layout::edge_anchor`] (the outer-edge frames),
//! [`Layout::shape`] and [`Layout::same_arrangement`] (the structure without sizes), and the tree functions
//! [`node_at`], [`first_active`] and [`tree_panels`]. Everything else in the editing files is private to the model.

use crate::flow::Axis;
use emath::{Pos2, Rect};

mod action;
mod edit;
mod groups;
mod inner;
mod layout;
mod limits;
mod node;
mod policy;
mod split;
mod zones;

mod apply;
mod arrange;
mod placement;
mod resize;
mod windows;
mod workspace;
mod workspaces;

pub use action::{Action, Location, Placement};
pub use edit::{first_active, node_at, tree_panels};
pub use groups::{Column, ColumnLocation, DockEdge, Group, GroupLocation, PanelPlace, PanelState, Target};
pub use layout::Layout;
pub use limits::{MAX_DEPTH, MAX_PANELS};
pub use node::{Floating, Node, StackEntry};
pub use policy::{DockError, Permissions};
pub use split::SplitSize;
pub use zones::{SIDE_ZONE_DEPTH_SHARE, SIDE_ZONE_FRACTION, ZoneSize, zones};

pub use arrange::{Arrangement, arrange_rects};
pub use placement::{NativeSpace, kept_reachable, restore_rect, sanitized};
pub use resize::AnchoredResize;
pub use windows::{DocumentGroup, DocumentWindows};
pub use workspace::Workspace;
pub use workspaces::{NamedWorkspace, WorkspaceChrome, WorkspaceCommand, WorkspaceError, WorkspaceSet};

/// The id of a dock group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct GroupId(pub u32);

impl GroupId {
    /// The group numbered `id`.
    pub const fn new(id: u32) -> Self {
        GroupId(id)
    }
}

/// A tree of splits over groups.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DockTree {
    /// One group.
    Group(GroupId),
    /// Children side by side.
    Split(Split),
}

/// Children laid out along an axis, each with a share of the extent.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Split {
    /// The axis the children follow: `X` puts them side by side, `Y` stacks them.
    pub axis: Axis,
    /// Each child with its share; shares are relative, so only their ratios matter.
    pub children: Vec<(f32, DockTree)>,
}

impl DockTree {
    /// A tree of one group.
    pub const fn group(id: GroupId) -> Self {
        DockTree::Group(id)
    }

    /// Children along `axis`, each with its share.
    pub fn split(axis: Axis, children: Vec<(f32, DockTree)>) -> Self {
        DockTree::Split(Split { axis, children })
    }

    /// The groups, in layout order.
    pub fn groups(&self) -> Vec<GroupId> {
        match self {
            DockTree::Group(id) => vec![*id],
            DockTree::Split(split) => split.children.iter().flat_map(|(_, child)| child.groups()).collect(),
        }
    }

    fn last_group(&self) -> Option<GroupId> {
        match self {
            DockTree::Group(id) => Some(*id),
            DockTree::Split(split) => split.children.last().and_then(|(_, child)| child.last_group()),
        }
    }

    fn node_mut(&mut self, path: &[usize]) -> Option<&mut DockTree> {
        match path.split_first() {
            None => Some(self),
            Some((&at, rest)) => match self {
                DockTree::Split(split) => split.children.get_mut(at)?.1.node_mut(rest),
                DockTree::Group(_) => None,
            },
        }
    }

    /// Where each group sits in `rect`.
    pub fn layout(&self, rect: Rect) -> Vec<(GroupId, Rect)> {
        let mut out = Vec::new();
        self.place(rect, &mut out);
        out
    }

    fn place(&self, rect: Rect, out: &mut Vec<(GroupId, Rect)>) {
        match self {
            DockTree::Group(id) => out.push((*id, rect)),
            DockTree::Split(split) => {
                for ((_, child), cell) in split.children.iter().zip(split.cells(rect)) {
                    child.place(cell, out);
                }
            }
        }
    }

    /// The handle between each pair of neighbouring children of every split in `rect`, `reach` thick across the
    /// boundary it straddles.
    pub fn handles(&self, rect: Rect, reach: f32) -> Vec<DockHandle> {
        let mut out = Vec::new();
        self.collect_handles(rect, reach, &mut Vec::new(), &mut out);
        out
    }

    fn collect_handles(&self, rect: Rect, reach: f32, path: &mut Vec<usize>, out: &mut Vec<DockHandle>) {
        let DockTree::Split(split) = self else { return };
        let cells = split.cells(rect);
        for (at, pair) in cells.windows(2).enumerate() {
            let boundary = split.axis.max_of(pair[0]);
            let bar = match split.axis {
                Axis::X => Rect::from_min_max(Pos2::new(boundary - reach / 2.0, rect.top()), Pos2::new(boundary + reach / 2.0, rect.bottom())),
                _ => Rect::from_min_max(Pos2::new(rect.left(), boundary - reach / 2.0), Pos2::new(rect.right(), boundary + reach / 2.0)),
            };
            if let Some(key) = split.children.get(at).and_then(|(_, child)| child.last_group()) {
                out.push(DockHandle { key, path: path.clone(), index: at, axis: split.axis, bar, split: rect });
            }
        }
        for (at, ((_, child), cell)) in split.children.iter().zip(cells).enumerate() {
            path.push(at);
            child.collect_handles(cell, reach, path, out);
            path.pop();
        }
    }

    /// Moves the boundary between children `index` and `index + 1` of the split at `path` to `at`, a coordinate
    /// along the split's axis in the physical-free logical space of `split_rect`. Neither neighbour shrinks below
    /// `min` points. Returns whether the tree changed.
    pub fn move_boundary(&mut self, path: &[usize], index: usize, split_rect: Rect, at: f32, min: f32) -> bool {
        let Some(DockTree::Split(split)) = self.node_mut(path) else { return false };
        let cells = split.cells(split_rect);
        let (Some(first), Some(second)) = (cells.get(index), cells.get(index + 1)) else { return false };
        let axis = split.axis;
        let (start, end) = (axis.min_of(*first), axis.max_of(*second));
        let total = end - start;
        if total <= 2.0 * min {
            return false;
        }
        let first_extent = (at - start).clamp(min, total - min);
        let shares = split.children[index].0 + split.children[index + 1].0;
        let first_share = shares * first_extent / total;
        if (split.children[index].0 - first_share).abs() <= f32::EPSILON {
            return false;
        }
        split.children[index].0 = first_share;
        split.children[index + 1].0 = shares - first_share;
        true
    }

    /// Docks the new group `new` on `side` of `target`. `DockSide::Center` joins the group's tabs, which is data
    /// the tree does not hold, so it changes nothing. Returns whether the tree changed.
    pub fn dock(&mut self, target: GroupId, side: DockSide, new: GroupId) -> bool {
        let Some(axis) = side.axis() else { return false };
        match self {
            DockTree::Group(id) if *id == target => {
                let pair = if side.leads() { [new, target] } else { [target, new] };
                *self = DockTree::split(axis, pair.into_iter().map(|id| (1.0, DockTree::Group(id))).collect());
                true
            }
            DockTree::Group(_) => false,
            DockTree::Split(split) => {
                let beside = split.axis == axis;
                let found = split.children.iter().position(|(_, child)| matches!(child, DockTree::Group(id) if *id == target));
                if let (true, Some(at)) = (beside, found) {
                    let half = split.children[at].0 / 2.0;
                    split.children[at].0 = half;
                    let to = if side.leads() { at } else { at + 1 };
                    split.children.insert(to, (half, DockTree::Group(new)));
                    return true;
                }
                split.children.iter_mut().any(|(_, child)| child.dock(target, side, new))
            }
        }
    }
}

impl Split {
    /// The rect of each child in `rect`.
    pub fn cells(&self, rect: Rect) -> Vec<Rect> {
        let total: f32 = self.children.iter().map(|(share, _)| share.max(0.0)).sum();
        let extent = self.axis.extent_of(rect);
        let mut at = self.axis.min_of(rect);
        let mut cells = Vec::with_capacity(self.children.len());
        for (share, _) in &self.children {
            let size = if total > 0.0 { extent * share.max(0.0) / total } else { extent / self.children.len().max(1) as f32 };
            cells.push(self.axis.span_of(rect, at, at + size));
            at += size;
        }
        cells
    }
}

/// The handle between two neighbouring children of a split.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DockHandle {
    /// A stable key: the last group of the child before the handle.
    pub key: GroupId,
    /// The child indices from the root to the split.
    pub path: Vec<usize>,
    /// The handle sits between children `index` and `index + 1`.
    pub index: usize,
    /// The axis the split's children follow.
    pub axis: Axis,
    /// The strip the handle occupies.
    pub bar: Rect,
    /// The rect of the split.
    pub split: Rect,
}

/// The side of a group a dropped panel docks on, in reading order: `Start` is the left half in a left-to-right
/// window and the right half in a right-to-left one. Saved layouts and actions written with the physical names
/// `Left` and `Right` load as `Start` and `End`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DockSide {
    /// Before the group along the reading direction.
    #[serde(alias = "Left", alias = "left")]
    Start,
    /// After the group along the reading direction.
    #[serde(alias = "Right", alias = "right")]
    End,
    /// Above the group.
    Top,
    /// Below the group.
    Bottom,
    /// Into the group's tabs.
    Center,
}

impl DockSide {
    /// The axis the side splits along; `None` for `Center`.
    pub const fn axis(self) -> Option<Axis> {
        match self {
            DockSide::Start | DockSide::End => Some(Axis::X),
            DockSide::Top | DockSide::Bottom => Some(Axis::Y),
            DockSide::Center => None,
        }
    }

    fn leads(self) -> bool {
        matches!(self, DockSide::Start | DockSide::Top)
    }

    /// The part of `group` a panel docked on this side takes: the half on that side, or the whole group.
    pub fn area(self, group: Rect) -> Rect {
        let center = group.center();
        match self {
            DockSide::Start => Rect::from_min_max(group.min, Pos2::new(center.x, group.bottom())),
            DockSide::End => Rect::from_min_max(Pos2::new(center.x, group.top()), group.max),
            DockSide::Top => Rect::from_min_max(group.min, Pos2::new(group.right(), center.y)),
            DockSide::Bottom => Rect::from_min_max(Pos2::new(group.left(), center.y), group.max),
            DockSide::Center => group,
        }
    }
}

impl Axis {
    fn min_of(self, rect: Rect) -> f32 {
        match self {
            Axis::X => rect.left(),
            _ => rect.top(),
        }
    }

    fn max_of(self, rect: Rect) -> f32 {
        match self {
            Axis::X => rect.right(),
            _ => rect.bottom(),
        }
    }

    fn extent_of(self, rect: Rect) -> f32 {
        self.max_of(rect) - self.min_of(rect)
    }

    fn span_of(self, rect: Rect, from: f32, to: f32) -> Rect {
        match self {
            Axis::X => Rect::from_min_max(Pos2::new(from, rect.top()), Pos2::new(to, rect.bottom())),
            _ => Rect::from_min_max(Pos2::new(rect.left(), from), Pos2::new(rect.right(), to)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::pos2;

    fn area() -> Rect {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 200.0))
    }

    fn two_columns() -> DockTree {
        DockTree::split(Axis::X, vec![(1.0, DockTree::group(GroupId(0))), (2.0, DockTree::group(GroupId(1)))])
    }

    #[test]
    fn shares_divide_the_extent() {
        let rects = two_columns().layout(area());
        assert_eq!(rects[0], (GroupId(0), Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 200.0))));
        assert_eq!(rects[1], (GroupId(1), Rect::from_min_max(pos2(100.0, 0.0), pos2(300.0, 200.0))));
    }

    #[test]
    fn a_dragged_boundary_keeps_the_shares_total_and_the_least_extent() {
        let mut tree = two_columns();
        assert!(tree.move_boundary(&[], 0, area(), 150.0, 20.0));
        assert_eq!(tree.layout(area())[0].1.width(), 150.0);
        assert!(tree.move_boundary(&[], 0, area(), -50.0, 20.0));
        assert_eq!(tree.layout(area())[0].1.width(), 20.0);
        assert!(tree.move_boundary(&[], 0, area(), 1000.0, 20.0));
        assert_eq!(tree.layout(area())[1].1.width(), 20.0);
    }

    #[test]
    fn docking_beside_a_group_of_the_same_axis_joins_its_split() {
        let mut tree = two_columns();
        assert!(tree.dock(GroupId(1), DockSide::Start, GroupId(2)));
        assert_eq!(tree.groups(), vec![GroupId(0), GroupId(2), GroupId(1)]);
        let widths: Vec<f32> = tree.layout(area()).iter().map(|(_, rect)| rect.width()).collect();
        assert_eq!(widths, vec![100.0, 100.0, 100.0]);
    }

    #[test]
    fn docking_across_the_axis_wraps_the_group_in_a_split() {
        let mut tree = two_columns();
        assert!(tree.dock(GroupId(0), DockSide::Bottom, GroupId(2)));
        assert_eq!(tree.groups(), vec![GroupId(0), GroupId(2), GroupId(1)]);
        assert_eq!(tree.layout(area())[1].1.top(), 100.0);
        assert!(!tree.dock(GroupId(0), DockSide::Center, GroupId(3)));
    }

    #[test]
    fn handles_straddle_each_boundary_and_are_keyed_by_the_group_before_them() {
        let handles = two_columns().handles(area(), 8.0);
        assert_eq!(handles.len(), 1);
        assert_eq!(handles[0].key, GroupId(0));
        assert_eq!(handles[0].bar, Rect::from_min_max(pos2(96.0, 0.0), pos2(104.0, 200.0)));
    }
}
