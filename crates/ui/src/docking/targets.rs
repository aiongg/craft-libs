//! The drop-zone registry of a dock and the payload of a whole-group drag.
//!
//! Group and column rendering write `DockSpot`s with their hit rects while a pass runs; drop resolution reads the
//! previous pass's registry, because a drop is decided against what was on screen when the pointer moved. Every
//! zone is a rect: previews are rects and lines, never polygons.

use super::PanelKey;
use crate::runtime::egui::pass;
use crate::runtime::frame::{PassStep, PerViewport, Stamped};
use craft_behavior::{DockSide, GroupId};
use egui::{Context, Pos2, Rect};

/// Depth of a group's side zones with `DockConfig::side_edge_zones`.
pub const SIDE_EDGE_ZONE_DEPTH: f32 = 40.0;

/// Share of a group's body, from each edge, that means "beside" without `DockConfig::side_edge_zones`: of its width
/// from the start and end edges, of its height from the top and bottom edges.
pub const SIDE_ZONE_FRACTION: f32 = 1.0 / 3.0;

/// The most of a body's smaller side that a side zone with `SIDE_EDGE_ZONE_DEPTH` reaches, so a body always keeps a
/// middle.
pub const SIDE_ZONE_DEPTH_SHARE: f32 = 0.25;

/// Width of the band along the layout's outer edge that splits at the root, at most `EDGE_BAND_SHARE` of the
/// layout's width and height.
pub const EDGE_BAND: f32 = 28.0;

/// The most of the layout's width and height the outer edge band takes.
pub const EDGE_BAND_SHARE: f32 = 0.25;

/// Width of the outer edge band with `DockConfig::side_edge_zones`.
pub const SIDE_EDGE_BAND: f32 = 8.0;

/// Height of the band centred on the seam between two stacked groups in a dock column.
pub const SEAM_BAND: f32 = 28.0;

/// Width of the band centred on each edge of a dock column, where a drop makes a new column.
pub const COLUMN_EDGE_BAND: f32 = 28.0;

/// Width of the band along the docked area's start and end edges, and inside an empty dock, that makes a new dock.
pub const NEW_DOCK_BAND: f32 = 28.0;

/// Thickness of the line that previews a drop on an edge band, a seam or a tab insertion.
pub const DROP_LINE_WIDTH: f32 = 4.0;

/// Where a drop would land.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum DockSpot {
    /// A tab insertion in `group` before the tab at `index` (the tab count for the end), previewed by `line`.
    Tab {
        /// The group whose strip takes the tab.
        group: GroupId,
        /// The position the tab takes among the group's tabs.
        index: usize,
        /// The insertion marker.
        line: Rect,
    },
    /// A side of `group`: beside it, or into its tabs for `DockSide::Center`.
    Group {
        /// The group.
        group: GroupId,
        /// The side.
        side: DockSide,
    },
    /// The layout's outer edge on `side`: a new root split.
    Edge(DockSide),
    /// The seam between two stacked groups of a dock column: a new group between them.
    Seam {
        /// The group above the seam.
        before: GroupId,
        /// The group below it.
        after: GroupId,
    },
    /// An edge of the dock column that holds `group`: a new column beside it.
    Column {
        /// A group of the column.
        group: GroupId,
        /// The column's side.
        side: DockSide,
    },
    /// A new dock along the docked area's start or end edge.
    NewDock(DockSide),
    /// A rect that takes no drop and hides the spots registered below it, such as a canvas that takes none.
    Blocked,
}

impl DockSpot {
    /// Which spot wins where rects of one layer overlap: the outer bands, then seams and columns, then a tab
    /// strip, then the sides of a group, then its middle. A border band reaches only `DROP_LINE_WIDTH` into a
    /// tab strip, so the strip keeps the rest of its height.
    const fn priority(&self) -> u8 {
        match self {
            DockSpot::Blocked => 0,
            DockSpot::Group { side: DockSide::Center, .. } => 1,
            DockSpot::Group { .. } => 2,
            DockSpot::Tab { .. } => 3,
            DockSpot::Seam { .. } | DockSpot::Column { .. } => 4,
            DockSpot::Edge(_) | DockSpot::NewDock(_) => 5,
        }
    }
}

/// How a group's body divides into side zones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ZoneRule {
    /// Zones `SIDE_EDGE_ZONE_DEPTH` deep, at most `SIDE_ZONE_DEPTH_SHARE` of the body's smaller side.
    Depth,
    /// Zones `SIDE_ZONE_FRACTION` of the body's width (start, end) and height (top, bottom).
    Fraction,
}

impl ZoneRule {
    /// The zones of `body`, the part of a group below its tab strip: the start and end zones along its full height,
    /// the top and bottom zones between them, and the middle. The rects do not overlap. A body too small for the
    /// zones gives them an empty rect.
    pub fn zones(self, body: Rect) -> [(DockSide, Rect); 5] {
        let (across, down) = match self {
            ZoneRule::Depth => {
                let depth = SIDE_EDGE_ZONE_DEPTH.min(body.width().min(body.height()) * SIDE_ZONE_DEPTH_SHARE).max(0.0);
                (depth, depth)
            }
            ZoneRule::Fraction => (body.width() * SIDE_ZONE_FRACTION, body.height() * SIDE_ZONE_FRACTION),
        };
        let start = Rect::from_min_max(body.min, Pos2::new(body.left() + across, body.bottom()));
        let end = Rect::from_min_max(Pos2::new(body.right() - across, body.top()), body.max);
        let (left, right) = (start.right(), end.left());
        let top = Rect::from_min_max(Pos2::new(left, body.top()), Pos2::new(right, body.top() + down));
        let bottom = Rect::from_min_max(Pos2::new(left, body.bottom() - down), Pos2::new(right, body.bottom()));
        let middle = Rect::from_min_max(Pos2::new(left, top.bottom()), Pos2::new(right, bottom.top()));
        [(DockSide::Start, start), (DockSide::End, end), (DockSide::Top, top), (DockSide::Bottom, bottom), (DockSide::Center, middle)]
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Target {
    spot: DockSpot,
    rect: Rect,
    layer: u32,
}

/// The spots of one pass with their hit rects. Spots are added back to front; a layer started with
/// [`Self::raise`] (a floating window) beats every earlier one wherever it covers the pointer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockTargets {
    targets: Vec<Target>,
    layer: u32,
}

impl DockTargets {
    /// An empty registry.
    pub fn new() -> Self {
        DockTargets::default()
    }

    /// Adds `spot`, taking drops over `rect`, to the current layer. An empty or non-finite rect takes none.
    pub fn add(&mut self, spot: DockSpot, rect: Rect) {
        if rect.is_finite() && rect.is_positive() {
            self.targets.push(Target { spot, rect, layer: self.layer });
        }
    }

    /// Starts a layer above every spot added so far.
    pub fn raise(&mut self) {
        self.layer += 1;
    }

    /// The spot a drop at `at` lands on: among the spots whose rect holds `at`, those of the highest layer, then
    /// the highest priority (see [`DockSpot`]), then the one added last. `None` where no spot holds `at` and where
    /// the winner is [`DockSpot::Blocked`].
    pub fn at(&self, at: Pos2) -> Option<DockSpot> {
        let winner = self
            .targets
            .iter()
            .enumerate()
            .filter(|(_, target)| target.rect.contains(at))
            .max_by_key(|(order, target)| (target.layer, target.spot.priority(), *order))?;
        match winner.1.spot {
            DockSpot::Blocked => None,
            spot => Some(spot),
        }
    }

    /// Empties the registry.
    pub fn clear(&mut self) {
        self.targets.clear();
        self.layer = 0;
    }

    /// Whether no spot is registered.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

/// The registry of the running pass and the one of the pass before.
#[derive(Default)]
struct Pass {
    now: DockTargets,
    before: DockTargets,
}

impl DockTargets {
    fn with_pass<R>(ctx: &Context, f: impl FnOnce(&mut Pass) -> R) -> R {
        let viewport = pass::current_viewport(ctx);
        PerViewport::<Stamped<Pass>>::with_pass(
            ctx,
            viewport,
            pass::pass_nr(ctx, viewport),
            |pass, step| {
                pass.before = match step {
                    PassStep::Next => std::mem::take(&mut pass.now),
                    PassStep::Skipped => DockTargets::new(),
                };
                pass.now.clear();
            },
            f,
        )
    }

    /// Registers `spot` over `rect` in the running pass of this viewport. Group and column rendering call it.
    pub fn register(ctx: &Context, spot: DockSpot, rect: Rect) {
        Self::with_pass(ctx, |pass| pass.now.add(spot, rect));
    }

    /// Starts a layer above the spots registered so far in the running pass: a floating window calls it before it
    /// registers its own.
    pub fn raise_layer(ctx: &Context) {
        Self::with_pass(ctx, |pass| pass.now.raise());
    }

    /// The spot a drop at `at` lands on, by the spots of the previous pass. Drop resolution calls it.
    pub fn resolve(ctx: &Context, at: Pos2) -> Option<DockSpot> {
        Self::with_pass(ctx, |pass| pass.before.at(at))
    }
}

/// What a dragged group (the bar or top border of a group) carries, beside `PanelDrag` for one tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GroupDrag {
    /// The dragged group.
    pub group: GroupId,
    /// The panel the group shows, for the drag's preview.
    pub active: PanelKey,
}

impl GroupDrag {
    /// A drag of `group` showing `active`. Only a group's drag bar starts one.
    pub const fn new(group: GroupId, active: PanelKey) -> Self {
        GroupDrag { group, active }
    }
}
