//! Drop resolution for a dock: what a dragged panel or group becomes when released on the spot under the pointer.
//!
//! A dock registers `DockSpot`s with `DockTargets`; [`resolve_drop`] reads the spot under the pointer and maps it to
//! the dock model's [`Action`], naming groups by an anchor panel. [`DockDropTarget`] runs the same resolution inside
//! `DropTarget::accept`, so the drag session arbitrates between docks and windows, and returns the mark the ghost
//! paints.

use super::{DockSpot, DockTargets, FloatSpace, GroupDrag, PanelDrag, PanelKey};
use crate::contract::{DropMark, DropOperation, DropTarget, Pointer};
use crate::runtime::drag::{self, DragPreview};
use crate::runtime::env::Label;
use craft_behavior::dock::{Action, DockEdge, Layout, Node, PanelPlace, Permissions, Placement, Target};
use craft_behavior::{CancelReason, DockSide};
use egui::{Context, Id, Pos2, Rect, Vec2};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// What a dock drag moves: one panel, or the whole group of a panel.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DragUnit<P> {
    /// The panel alone.
    Panel(P),
    /// The group that holds the panel, named by it.
    Group(P),
}

impl<P> DragUnit<P> {
    /// The panel that names the unit.
    pub fn lead(&self) -> &P {
        match self {
            DragUnit::Panel(panel) | DragUnit::Group(panel) => panel,
        }
    }
}

/// A drag payload a dock resolves drops for.
pub trait DockDrag<P> {
    /// What the drag moves in `layout`; `None` when the dragged panel is not in it.
    fn unit(&self, layout: &Layout<P>) -> Option<DragUnit<P>>;
}

/// The key a panel has in drag payloads and ids: the hash of the panel.
pub fn panel_key<P: Hash>(panel: &P) -> PanelKey {
    let mut hasher = DefaultHasher::new();
    panel.hash(&mut hasher);
    PanelKey::new(hasher.finish())
}

impl<P: Clone + Eq + Hash> DockDrag<P> for PanelDrag {
    fn unit(&self, layout: &Layout<P>) -> Option<DragUnit<P>> {
        layout.panels().into_iter().find(|panel| panel_key(*panel) == self.panel).cloned().map(DragUnit::Panel)
    }
}

impl<P: Clone> DockDrag<P> for GroupDrag<P> {
    fn unit(&self, _layout: &Layout<P>) -> Option<DragUnit<P>> {
        Some(DragUnit::Group(self.anchor.clone()))
    }
}

/// What a drop does and what the ghost paints for it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DockDropped<P> {
    /// The change the drop makes to the layout.
    pub action: Action<P>,
    /// Where the dragged unit would land.
    pub mark: DropMark,
}

/// The panels of the group, tab node or window frame that holds `anchor`.
fn members<P: Clone + Eq + Hash>(layout: &Layout<P>, anchor: &P) -> Vec<P> {
    let Some(place) = layout.place(anchor) else { return vec![anchor.clone()] };
    if let Some(group) = layout.group(&place) {
        return group.panels.clone();
    }
    match layout.node_of(&place) {
        Some(Node::Tabs { panels, .. }) => panels.clone(),
        _ => vec![anchor.clone()],
    }
}

fn moving_panels<P: Clone + Eq + Hash>(layout: &Layout<P>, unit: &DragUnit<P>) -> Vec<P> {
    match unit {
        DragUnit::Panel(panel) => vec![panel.clone()],
        DragUnit::Group(anchor) => members(layout, anchor),
    }
}

/// A panel of the group of `named` that is not moving, so a drop on the unit's own group names a destination the
/// move does not take away.
fn outside<P: Clone + Eq + Hash>(layout: &Layout<P>, named: &P, moved: &[P]) -> Option<P> {
    if !moved.contains(named) {
        return Some(named.clone());
    }
    members(layout, named).into_iter().find(|panel| !moved.contains(panel))
}

fn beside<P: Clone>(unit: &DragUnit<P>, anchor: P, placement: Placement<P>) -> Action<P> {
    match unit {
        DragUnit::Panel(panel) => Action::move_beside(panel.clone(), anchor, placement),
        DragUnit::Group(lead) => Action::move_group(lead.clone(), Target::at(anchor, placement)),
    }
}

fn to_target<P: Clone>(unit: &DragUnit<P>, target: Target<P>) -> Action<P> {
    match unit {
        DragUnit::Panel(panel) => Action::move_panel(panel.clone(), target),
        DragUnit::Group(lead) => Action::move_group(lead.clone(), target),
    }
}

/// The action a drop of `unit` on `spot` makes, or `None` when the unit cannot move there: the permissions refuse,
/// the destination is the unit itself, or the spot has no placement.
fn action_for<P: Clone + Eq + Hash>(
    layout: &Layout<P>,
    spot: &DockSpot<P>,
    unit: &DragUnit<P>,
    permissions: &dyn Fn(&P) -> Permissions,
) -> Option<Action<P>> {
    if !permissions(unit.lead()).movable {
        return None;
    }
    let moved = moving_panels(layout, unit);
    let place = |anchor: P, placement: Placement<P>| {
        let floating = matches!(layout.place(&anchor), Some(PanelPlace::Floating { .. }));
        placement.allowed_by(permissions(&anchor), floating).then(|| beside(unit, anchor, placement))
    };
    match spot {
        DockSpot::Tab { anchor, index } => {
            let destination = outside(layout, anchor, &moved)?;
            let before = members(layout, anchor).into_iter().skip(*index).find(|panel| !moved.contains(panel));
            place(destination, Placement::tab(before))
        }
        DockSpot::Group { anchor, side } => {
            if *side == DockSide::Center && members(layout, anchor).iter().any(|panel| moved.contains(panel)) {
                return None;
            }
            let destination = outside(layout, anchor, &moved)?;
            let placement = Placement::for_zone(layout, &destination, *side).ok()?;
            place(destination, placement)
        }
        DockSpot::Edge(side) => {
            if *side == DockSide::Center {
                return None;
            }
            let protected = |panel: &P| permissions(panel).is_protected();
            let skip = |panel: &P| moved.contains(panel);
            let anchor = layout.edge_anchor(layout.edge_narrows(*side, &protected), &protected, &skip)?;
            Some(beside(unit, anchor, Placement::Edge(*side)))
        }
        DockSpot::NewDock(side) => {
            let edge = match side {
                DockSide::Start => DockEdge::Start,
                DockSide::End => DockEdge::End,
                _ => return None,
            };
            Some(to_target(unit, Target::new_dock(edge)))
        }
        DockSpot::Seam { before, after } => match outside(layout, before, &moved) {
            Some(anchor) => place(anchor, Placement::group(true, None)),
            None => place(outside(layout, after, &moved)?, Placement::group(false, None)),
        },
        DockSpot::Column { anchor, side } => {
            let destination = outside(layout, anchor, &moved)?;
            let placement =
                Placement::for_zone(layout, &destination, *side).ok().filter(|placement| matches!(placement, Placement::Column { .. }))?;
            place(destination, placement)
        }
        DockSpot::Blocked => None,
    }
}

/// The action that floats `unit` in a window under `at`, inside the area of `space`, at the size `space` recorded for
/// it, and the outline the ghost paints for it. The pointer sits on the window's title bar.
pub fn float_drop<P: Clone + PartialEq>(unit: &DragUnit<P>, space: &FloatSpace<P>, at: Pos2) -> DockDropped<P> {
    let recorded = match unit {
        DragUnit::Panel(panel) => space.panel(panel),
        DragUnit::Group(anchor) => space.group(anchor),
    };
    let size = recorded.unwrap_or(space.default_size).min(space.area.size());
    let min = (at - Vec2::new(size.x * 0.5, space.inset)).min(space.area.max - size).max(space.area.min);
    let rect = Rect::from_min_size(min, size);
    let action = match unit {
        DragUnit::Panel(panel) => Action::float(panel.clone(), rect),
        DragUnit::Group(anchor) => Action::move_group(anchor.clone(), Target::float(rect)),
    };
    DockDropped { action, mark: DropMark::Into(rect) }
}

/// The action a release of `drag` at `at` makes in the dock `dock`, by the spots of the previous pass; `None` where
/// no spot takes the drag. Call it from the dock's `DropTarget::accept` only (see `DockTargets::resolve`).
pub fn resolve_drop<P, D>(
    ctx: &Context,
    dock: Id,
    layout: &Layout<P>,
    at: Pos2,
    drag: &D,
    permissions: &dyn Fn(&P) -> Permissions,
) -> Option<Action<P>>
where
    P: Clone + Eq + Hash + Send + 'static,
    D: DockDrag<P>,
{
    let unit = drag.unit(layout)?;
    let spot = DockTargets::<P>::resolve(ctx, dock, at)?;
    action_for(layout, &spot, &unit, permissions)
}

/// The drop target of one dock over one layout, for a tab drag ([`PanelDrag`]) and a group drag ([`GroupDrag`]).
///
/// It accepts a move only. Fields are assigned by the dock after [`Self::new`].
#[non_exhaustive]
pub struct DockDropTarget<'a, P> {
    ctx: &'a Context,
    /// The dock whose spots resolve the pointer.
    pub dock: Id,
    layout: &'a Layout<P>,
    permissions: &'a dyn Fn(&P) -> Permissions,
    rects: &'a dyn Fn(&DockSpot<P>) -> Option<Rect>,
    /// The layout direction, which mirrors side marks.
    pub rtl: bool,
    /// Set while Ctrl or Cmd is held: every drop floats the unit in this area instead of docking it.
    pub floating: Option<&'a FloatSpace<P>>,
}

impl<'a, P> DockDropTarget<'a, P> {
    /// The target of `dock` over `layout`. `rects` gives the rect a spot applies to for its mark: the group's rect
    /// for a group spot, the registered hit rect for the others.
    pub fn new(
        ctx: &'a Context,
        dock: Id,
        layout: &'a Layout<P>,
        permissions: &'a dyn Fn(&P) -> Permissions,
        rects: &'a dyn Fn(&DockSpot<P>) -> Option<Rect>,
    ) -> Self {
        DockDropTarget { ctx, dock, layout, permissions, rects, rtl: false, floating: None }
    }
}

impl<P: Clone + Eq + Hash + Send + 'static> DockDropTarget<'_, P> {
    fn take<D: DockDrag<P>>(&self, drag: &D, at: Pos2, op: DropOperation) -> Option<DockDropped<P>> {
        if !matches!(op, DropOperation::Move) {
            return None;
        }
        let unit = drag.unit(self.layout)?;
        if let Some(space) = self.floating {
            return (self.permissions)(unit.lead()).floatable.then(|| float_drop(&unit, space, at));
        }
        let spot = DockTargets::<P>::resolve(self.ctx, self.dock, at)?;
        let action = action_for(self.layout, &spot, &unit, self.permissions)?;
        let mark = spot.mark((self.rects)(&spot)?, self.rtl)?;
        Some(DockDropped { action, mark })
    }
}

impl<P: Clone + Eq + Hash + Send + 'static> DropTarget<PanelDrag> for DockDropTarget<'_, P> {
    type Drop = DockDropped<P>;

    fn accept(&self, payload: &PanelDrag, at: Pos2, op: DropOperation) -> Option<DockDropped<P>> {
        self.take(payload, at, op)
    }

    fn mark(&self, drop: &DockDropped<P>) -> DropMark {
        drop.mark
    }
}

impl<P: Clone + Eq + Hash + Send + 'static> DropTarget<GroupDrag<P>> for DockDropTarget<'_, P> {
    type Drop = DockDropped<P>;

    fn accept(&self, payload: &GroupDrag<P>, at: Pos2, op: DropOperation) -> Option<DockDropped<P>> {
        self.take(payload, at, op)
    }

    fn mark(&self, drop: &DockDropped<P>) -> DropMark {
        drop.mark
    }
}

/// Starts the drag of a whole group named by `anchor`, whose shown panel is `active`: the ghost is a frame of `size`
/// labelled `label`.
pub fn start_group_drag<P, C>(cx: &mut C, anchor: P, active: PanelKey, size: Vec2, label: Label)
where
    P: Clone + Send + Sync + 'static,
    C: Pointer,
{
    cx.start_drag(GroupDrag { anchor, active }, DragPreview::frame(size, Some(label)));
}

/// Ends a running tab or group drag when the layout it was started on is replaced by another arrangement: the
/// panel or group it names may be gone or elsewhere. Size-only edits keep the drag.
pub fn cancel_on_replacement<P>(ctx: &Context, before: &Layout<P>, after: &Layout<P>)
where
    P: Clone + Eq + Hash + Send + Sync + 'static,
{
    if before.same_arrangement(after) {
        return;
    }
    if drag::active::<PanelDrag>(ctx).is_some() || drag::active::<GroupDrag<P>>(ctx).is_some() {
        drag::cancel(ctx, CancelReason::HostChanged);
    }
}
