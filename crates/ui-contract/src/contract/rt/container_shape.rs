//! What a container adds to its node's run at the driver's steps (`ShapeHooks`): the move band and the resize grips of
//! a layer mount and their drags, the grip of an edge pane and its drag, the reorder band, and the placement each
//! gesture leaves, reported to `Container::placement_changed`.
//!
//! A layer's placement lives in the container's saved slot when its props give a `SaveKey`, else in session memory
//! (`placement_of`): the layer mount reads it when it places the layer (`show_on_layer`), before the container's run.
//! An edge pane's extent lives in session memory (`pane_extent_of`); while its grip is dragged the gesture's extent
//! wins (`pane_shown`). A run that moves either asks for another build round of the frame (`ContainerShape::moved`),
//! so the layer or pane stands where the step put it in the frame of the step.

use craft_behavior::{CursorIcon, GesturePhase, HitLevel, HitPriority, PartKey, PressPhase, Sense};
use craft_ui_rt::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::contract::overlay::{Place, PlacementChange, PlacementKind, ResizeEdges};
use crate::contract::run::ShapeHooks;
use crate::contract::{Container, ContainerProps, Context, Cx, DragBand, Id, MOVE_BAND, Mount, Overlay, PartsCx, REORDER_BAND, ShellPane};
use crate::runtime::env::PaneEdge;
use crate::runtime::memory::{MemoryStage, SaveKey, sub};
use crate::runtime::windows::ResizeEdge;

/// A resize grip of a layer: indexed by its place in `GRIPS`.
const GRIP: PartKey = PartKey::new("resize_grip");

/// The grip on an edge pane's inner edge.
const PANE_GRIP: PartKey = PartKey::new("pane_grip");

/// The eight edge and corner grips of a resizable layer: which edges each moves (`-1` the low edge, `1` the high one).
const GRIPS: [(i8, i8, CursorIcon); 8] = [
    (-1, 0, CursorIcon::ResizeWest),
    (1, 0, CursorIcon::ResizeEast),
    (0, -1, CursorIcon::ResizeNorth),
    (0, 1, CursorIcon::ResizeSouth),
    (-1, -1, CursorIcon::ResizeNorthWest),
    (1, 1, CursorIcon::ResizeSouthEast),
    (1, -1, CursorIcon::ResizeNorthEast),
    (-1, 1, CursorIcon::ResizeSouthWest),
];

/// Where a container on a layer sits and how large the user made it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Placement {
    pub pos: Option<[f32; 2]>,
    pub size: Option<[f32; 2]>,
}

impl Placement {
    /// The placement that puts the layer at `rect`.
    fn at(rect: Rect) -> Placement {
        Placement { pos: Some(rect.min.into()), size: Some(rect.size().into()) }
    }

    /// The rect it names, taking the position and size it lacks from `rect`.
    fn rect_over(&self, rect: Rect) -> Rect {
        Rect::from_min_size(self.pos.map_or(rect.min, Pos2::from), self.size.map_or(rect.size(), Vec2::from))
    }
}

fn placement_id(id: Id) -> Id {
    sub(id, "placement")
}

/// The placement of the layer container `id` as its last run left it, read without its run: the layer mount places the
/// layer by it. A layer the caller places (`Overlay::placed`, `placed_at`) is placed by the caller's rect instead.
pub(crate) fn placement_of(ctx: &Context, id: Id, save: Option<&SaveKey>) -> Placement {
    match save {
        Some(key) => MemoryStage::peek_saved(ctx, key),
        None => MemoryStage::peek_session(ctx, placement_id(id)),
    }
}

/// The one store of a layer's placement (`placement_of`), seen from the container's run. A layer the caller places
/// keeps the caller's rect: the slot writes nothing, and the caller learns of the user's changes from the reports. A
/// layer hung from its anchor keeps only its size: each opening places it from its anchor at that size.
struct PlacementSlot<'s> {
    stage: &'s mut MemoryStage,
    id: Id,
    save: Option<&'s SaveKey>,
    controlled: Option<Placement>,
    anchored: bool,
}

impl<'s> PlacementSlot<'s> {
    fn new(stage: &'s mut MemoryStage, id: Id, save: Option<&'s SaveKey>, overlay: &Overlay) -> Self {
        let controlled = overlay
            .placed
            .map(|rect| Placement::at(kept_in(rect, overlay.bounds)))
            .or_else(|| overlay.placed_at.map(|pos| Placement { pos: Some(pos.into()), size: None }));
        PlacementSlot { stage, id, save, controlled, anchored: matches!(overlay.place, Place::Anchored { .. }) }
    }

    fn stored(&mut self) -> &mut Placement {
        match self.save {
            Some(key) => self.stage.saved::<Placement>(key),
            None => self.stage.session::<Placement>(placement_id(self.id)),
        }
    }

    fn read(&mut self) -> Placement {
        match &self.controlled {
            Some(placement) => placement.clone(),
            None => self.stored().clone(),
        }
    }

    /// Writes `placement`; whether the stored placement changed.
    fn write(&mut self, placement: Placement) -> bool {
        if self.controlled.is_some() {
            return false;
        }
        let pos = if self.anchored { None } else { placement.pos };
        let next = Placement { pos, ..placement };
        let stored = self.stored();
        let changed = *stored != next;
        *stored = next;
        changed
    }
}

/// `rect` moved, at its size, the least distance that puts it inside `bounds`; on an axis where it is longer than the
/// bounds, its low edge sits on theirs.
fn keep_inside(rect: Rect, bounds: Rect) -> Rect {
    let keep = |low: f32, high: f32, bounds_low: f32, bounds_high: f32| low.min(bounds_high - (high - low)).max(bounds_low) - low;
    rect.translate(Vec2::new(keep(rect.min.x, rect.max.x, bounds.min.x, bounds.max.x), keep(rect.min.y, rect.max.y, bounds.min.y, bounds.max.y)))
}

fn kept_in(rect: Rect, bounds: Option<Rect>) -> Rect {
    bounds.map_or(rect, |bounds| keep_inside(rect, bounds))
}

/// What every change of a layer's rect keeps to: the least and most size, and the rect it stays inside.
struct Limits {
    min: Vec2,
    max: Vec2,
    bounds: Option<Rect>,
}

impl Limits {
    fn new(overlay: &Overlay) -> Limits {
        let min = overlay.size_limits.min;
        Limits { min, max: overlay.size_limits.max.unwrap_or(Vec2::INFINITY).max(min), bounds: overlay.bounds }
    }

    fn moved(&self, rect: Rect, delta: Vec2) -> Rect {
        kept_in(rect.translate(delta), self.bounds)
    }

    /// `rect` with the edges `dx` and `dy` moved by `delta`, within the size limits and the bounds; the opposite edge
    /// stays.
    fn resized(&self, rect: Rect, dx: i8, dy: i8, delta: Vec2) -> Rect {
        let axis = |low: f32, high: f32, edge: i8, delta: f32, min: f32, max: f32, bounds: Option<(f32, f32)>| {
            let (bounds_low, bounds_high) = bounds.unwrap_or((f32::NEG_INFINITY, f32::INFINITY));
            match edge {
                -1 => ((low + delta).max((high - max).max(bounds_low)).min(high - min), high),
                1 => (low, (high + delta).min((low + max).min(bounds_high)).max(low + min)),
                _ => (low, high),
            }
        };
        let (left, right) = axis(rect.min.x, rect.max.x, dx, delta.x, self.min.x, self.max.x, self.bounds.map(|b| (b.min.x, b.max.x)));
        let (top, bottom) = axis(rect.min.y, rect.max.y, dy, delta.y, self.min.y, self.max.y, self.bounds.map(|b| (b.min.y, b.max.y)));
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))
    }
}

/// The edge or corner the grip with direction (`dx`, `dy`) sits on.
fn edge_of(dx: i8, dy: i8) -> ResizeEdge {
    match (dx, dy) {
        (-1, 0) => ResizeEdge::Left,
        (1, 0) => ResizeEdge::Right,
        (0, -1) => ResizeEdge::Top,
        (-1, -1) => ResizeEdge::TopLeft,
        (1, -1) => ResizeEdge::TopRight,
        (-1, 1) => ResizeEdge::BottomLeft,
        (1, 1) => ResizeEdge::BottomRight,
        _ => ResizeEdge::Bottom,
    }
}

/// The edges of a layer its placement holds, by screen side (index 0 the left or top edge, 1 the right or bottom one):
/// a layer hung from its anchor that the user cannot move keeps the edge that faces the anchor and the edge it aligns
/// to the anchor's, so a grip on such an edge would move the opposite one.
#[derive(Clone, Copy)]
struct Pinned {
    x: [bool; 2],
    y: [bool; 2],
}

impl Pinned {
    const NONE: Pinned = Pinned { x: [false; 2], y: [false; 2] };

    fn of(overlay: &Overlay, rtl: bool) -> Pinned {
        use craft_behavior::overlay::{Align, Side};
        let Place::Anchored { side, align, .. } = &overlay.place else { return Pinned::NONE };
        if overlay.movable {
            return Pinned::NONE;
        }
        let (start, end) = if rtl { (1, 0) } else { (0, 1) };
        let edge = |at: usize| {
            let mut held = [false; 2];
            held[at] = true;
            held
        };
        let aligned = |start: usize, end: usize| match align {
            Align::Start => edge(start),
            Align::End => edge(end),
            _ => [true; 2],
        };
        match side {
            Side::Below => Pinned { x: aligned(start, end), y: edge(0) },
            Side::Above => Pinned { x: aligned(start, end), y: edge(1) },
            Side::Start => Pinned { x: edge(end), y: aligned(0, 1) },
            Side::End => Pinned { x: edge(start), y: aligned(0, 1) },
            _ => Pinned::NONE,
        }
    }

    fn holds(self, dx: i8, dy: i8) -> bool {
        let held = |edges: [bool; 2], d: i8| d != 0 && edges[usize::from(d > 0)];
        held(self.x, dx) || held(self.y, dy)
    }
}

fn moved_edges(dx: i8, dy: i8) -> ResizeEdges {
    let x = match dx {
        -1 => ResizeEdges::LEFT,
        1 => ResizeEdges::RIGHT,
        _ => ResizeEdges::NONE,
    };
    let y = match dy {
        -1 => ResizeEdges::TOP,
        1 => ResizeEdges::BOTTOM,
        _ => ResizeEdges::NONE,
    };
    x.union(y)
}

/// The grip with direction (`dx`, `dy`) stands when every edge it moves is in `edges` and the placement leaves it free.
fn grip_stands(edges: ResizeEdges, pinned: Pinned, dx: i8, dy: i8) -> bool {
    edges.contains(moved_edges(dx, dy)) && !pinned.holds(dx, dy)
}

/// A layer container's pointer gesture, kept in its node memory: the rect the layer had when a press went down on its
/// move band or a grip, the rect a running drag runs from, the placement Escape restores, and the grip a drag holds
/// (kept declared until its release, so the drag keeps its result when the layer flips sides).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WindowMode {
    pressed: Option<Rect>,
    began: Option<Rect>,
    restore: Placement,
    held: Option<usize>,
}

impl WindowMode {
    /// A drag of the move band runs: the body's `BodyCx::moving`.
    pub(crate) fn moving(&self) -> bool {
        self.began.is_some() && self.held.is_none()
    }
}

pub(crate) fn window_mode_id(id: Id) -> Id {
    sub(id, "window_mode")
}

/// An edge pane's grip gesture: the extent the pane had when it began and the extent the pointer gives it. Session
/// memory, so the pane mount reads it before the run that writes it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PaneGrip {
    began: Option<f32>,
    extent: Option<f32>,
}

fn pane_grip_id(id: Id) -> Id {
    sub(id, "pane_grip")
}

fn pane_extent_id(id: Id) -> Id {
    sub(id, "pane_extent")
}

/// The extent an edge pane shows at along its edge: a running grip gesture's, else the caller's fixed extent, else the
/// pane's session memory (the size it first took, or the extent the user gave it). `None` before its first show.
pub(crate) fn pane_shown(ctx: &Context, id: Id, pane: &ShellPane) -> Option<f32> {
    MemoryStage::peek_session::<PaneGrip>(ctx, pane_grip_id(id)).extent.or(pane.extent).or_else(|| pane_extent_of(ctx, id))
}

fn pane_extent_of(ctx: &Context, id: Id) -> Option<f32> {
    MemoryStage::peek_session::<Option<f32>>(ctx, pane_extent_id(id))
}

/// Records the extent an edge pane with no fixed extent first took (`pane_shown` was `None`).
pub(crate) fn note_pane_extent(ctx: &Context, id: Id, extent: f32) {
    let mut stage = MemoryStage::open(ctx, id);
    *stage.session::<Option<f32>>(pane_extent_id(id)) = Some(extent);
    stage.write_back();
}

/// The screen edge of an edge pane's grip (its inner edge, toward the window's centre); `None` for the central pane and
/// a pane its caller lays out.
fn pane_grip_edge(pane: &ShellPane, rtl: bool) -> Option<ResizeEdge> {
    if pane.at.is_some() {
        return None;
    }
    Some(match pane.edge {
        PaneEdge::Start if rtl => ResizeEdge::Left,
        PaneEdge::Start => ResizeEdge::Right,
        PaneEdge::End if rtl => ResizeEdge::Right,
        PaneEdge::End => ResizeEdge::Left,
        PaneEdge::Top => ResizeEdge::Bottom,
        PaneEdge::Bottom => ResizeEdge::Top,
        PaneEdge::Center => return None,
    })
}

/// Where a container's header lies in this build (window coordinates), from its layout: the move band and the reorder
/// band cover it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HeaderGeometry {
    /// The header's line (header and controls); `None` without a header.
    pub(crate) line: Option<Rect>,
    /// Where the body starts.
    pub(crate) body_top: f32,
    /// The rect of the header, body and end region of an inline header's line.
    pub(crate) inline_line: Option<Rect>,
}

/// The container's hooks on its node's run.
pub(crate) struct ContainerShape<'p> {
    props: &'p ContainerProps,
    geometry: HeaderGeometry,
    /// The depth of a grip (`size.handle.sm`) and an edge pane's least extent (`size.pane.least`).
    handle: f32,
    least: f32,
    /// The run moved or resized the layer, or gave the pane a new extent: the build that placed it read the old one.
    pub(crate) moved: bool,
    reports: Vec<PlacementChange>,
}

impl<'p> ContainerShape<'p> {
    pub(crate) fn new(props: &'p ContainerProps, geometry: HeaderGeometry, tokens: &craft_theme::Tokens) -> Self {
        let (handle, least) = (tokens.size.handle.sm, tokens.size.pane.least);
        ContainerShape { props, geometry, handle, least, moved: false, reports: Vec::new() }
    }

    /// The overlay of a layer mount the user can move or size.
    fn layer(&self) -> Option<&'p Overlay> {
        match &self.props.mount {
            Mount::Layer(overlay) if !overlay.moves_window => Some(overlay),
            _ => None,
        }
    }

    fn has_parts(&self) -> bool {
        let reorder = self.props.header.is_some_and(|header| header.band == DragBand::Reorder);
        let layer = self.layer().is_some_and(|overlay| overlay.movable || overlay.resize.edges != ResizeEdges::NONE);
        let pane = matches!(&self.props.mount, Mount::Pane(pane) if pane.at.is_none() && pane.edge != PaneEdge::Center);
        reorder || layer || pane
    }

    /// The move band: a header on a line of its own drags from the frame's top and side edges down to where the body
    /// starts, as a title bar does; an inline header's band spans its whole line.
    fn move_band(&self, rect: Rect) -> Option<Rect> {
        let line = self.geometry.line?;
        if let Some(inline) = self.geometry.inline_line {
            return Some(inline.intersect(rect));
        }
        let bottom = line.max.y.max(self.geometry.body_top.min(rect.max.y));
        Some(Rect::from_min_max(rect.min, Pos2::new(rect.max.x, bottom)).intersect(rect))
    }

    fn layer_parts<C: Container>(&self, overlay: &Overlay, rect: Rect, held: Option<usize>, pcx: &mut PartsCx<'_, C>) {
        let reason = "the Move and Size commands of the window command set do the same";
        if overlay.movable
            && let Some(band) = self.move_band(rect).filter(|band| band.is_positive())
        {
            // The band lies under the header's own parts and children, so they keep their presses.
            let hand = pcx.measure.env.get::<crate::contract::MoveBandCursor>() == crate::contract::MoveBandCursor::Hand;
            let part = pcx.part(MOVE_BAND, band).sense(Sense::DRAG).pointer_only(reason);
            if hand {
                part.cursor(CursorIcon::Grab).drag_cursor(CursorIcon::Grabbing);
            }
        }
        let edges = overlay.resize.edges;
        if edges == ResizeEdges::NONE {
            return;
        }
        // Each grip lies inside the frame: an edge grip `handle` deep, a corner grip twice that along each of its
        // edges, the edge grips between the corners. They stack above the body and take exactly their rects.
        let handle = self.handle;
        let corner = 2.0 * handle;
        let pinned = Pinned::of(overlay, pcx.flow.is_rtl());
        let name = self.props.resize_name.as_ref().map(|name| pcx.measure.env.resolve(name).as_str().to_owned());
        for (index, (dx, dy, cursor)) in GRIPS.into_iter().enumerate() {
            if held != Some(index) && !grip_stands(edges, pinned, dx, dy) {
                continue;
            }
            let span = |edge: i8, low: f32, high: f32, deep: f32| match edge {
                -1 => (low, low + deep),
                1 => (high - deep, high),
                _ => (low + corner, high - corner),
            };
            let at_corner = dx != 0 && dy != 0;
            let deep = if at_corner { corner } else { handle };
            let (x0, x1) = span(dx, rect.min.x, rect.max.x, deep);
            let (y0, y1) = span(dy, rect.min.y, rect.max.y, deep);
            let level = if at_corner { HitLevel::Handle } else { HitLevel::Edge };
            let part = pcx
                .part(GRIP.at(index as u64), Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)))
                .sense(Sense::DRAG)
                .priority(HitPriority::new(level, 0))
                .cursor(cursor)
                .pointer_only(reason)
                .exact_hit()
                .over();
            if let Some(name) = &name {
                part.node(crate::runtime::access::Role::SPLITTER, name.as_str());
            }
        }
    }

    fn pane_part<C: Container>(&self, pane: &ShellPane, rect: Rect, pcx: &mut PartsCx<'_, C>) {
        let Some(edge) = pane_grip_edge(pane, pcx.flow.is_rtl()) else { return };
        let handle = self.handle;
        let (grip, cursor) = match edge {
            ResizeEdge::Left => (Rect::from_min_max(rect.min, Pos2::new(rect.min.x + handle, rect.max.y)), CursorIcon::ResizeHorizontal),
            ResizeEdge::Right => (Rect::from_min_max(Pos2::new(rect.max.x - handle, rect.min.y), rect.max), CursorIcon::ResizeHorizontal),
            ResizeEdge::Top => (Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + handle)), CursorIcon::ResizeVertical),
            _ => (Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - handle), rect.max), CursorIcon::ResizeVertical),
        };
        pcx.part(PANE_GRIP, grip)
            .sense(Sense::DRAG)
            .priority(HitPriority::new(HitLevel::Edge, 0))
            .cursor(cursor)
            .exact_hit()
            .pointer_only("no command resizes a docked pane; its extent is the app's layout");
    }

    /// A drag of the move band or a grip of a layer: the layer follows the pointer within the limits, each step is
    /// reported, and the placement keeps the step's rect. The drag runs from the rect the layer had when the press went
    /// down. Escape, which cancels the press, restores the placement from before.
    fn layer_drag<C: Container>(&mut self, overlay: &Overlay, cx: &mut Cx<'_, C>) {
        let (id, rect) = (cx.core.id, cx.core.rect);
        let phase = |key: (PartKey, Option<u64>)| cx.core.results.get(&key).map(|result| result.press.phase);
        let mut pressed = vec![(None, phase((MOVE_BAND, None)))];
        pressed.extend((0..GRIPS.len()).map(|index| (Some(index), phase((GRIP, Some(index as u64))))));
        let Some((grip, phase)) = pressed.into_iter().find_map(|(grip, phase)| match phase? {
            phase @ (PressPhase::Down { .. }
            | PressPhase::DragStart { .. }
            | PressPhase::Drag { .. }
            | PressPhase::Release { .. }
            | PressPhase::Cancel { .. }) => Some((grip, phase)),
            _ => None,
        }) else {
            return;
        };
        let limits = Limits::new(overlay);
        let mut mode = cx.core.memory.recall::<WindowMode>(window_mode_id(id));
        let save = self.props.save.as_ref();
        let mut slot = PlacementSlot::new(&mut *cx.core.memory, id, save, overlay);
        let current = slot.read();
        let (start, pos, gesture) = match phase {
            PressPhase::Down { .. } => {
                mode.pressed = Some(current.rect_over(rect));
                *cx.core.memory.shown::<WindowMode>(window_mode_id(id)) = mode;
                return;
            }
            PressPhase::DragStart { start, pos, .. } => {
                mode.began = Some(mode.pressed.unwrap_or_else(|| current.rect_over(rect)));
                mode.restore = current.clone();
                mode.held = grip;
                (start, Some(pos), GesturePhase::Begin)
            }
            PressPhase::Drag { start, pos, .. } => (start, Some(pos), GesturePhase::Update),
            PressPhase::Release { start, pos, .. } => (start, Some(pos), GesturePhase::Commit),
            PressPhase::Cancel { start, .. } => (start, None, GesturePhase::Cancel),
            _ => return,
        };
        let Some(began) = mode.began else { return };
        let (dx, dy, _) = grip.map_or((0, 0, CursorIcon::Grab), |index| GRIPS[index]);
        let kind = if grip.is_some() { PlacementKind::Size(edge_of(dx, dy)) } else { PlacementKind::Move };
        let next = match pos {
            Some(pos) if grip.is_some() => limits.resized(began, dx, dy, pos - start.origin),
            Some(pos) => limits.moved(began, pos - start.origin),
            None => began,
        };
        // A move band press another component handed over (`Overlay::continue_move`) continues that component's drag.
        let continued = grip.is_none() && start.handed;
        let change = |rect: Rect, phase: GesturePhase| PlacementChange { rect, phase, pointer: pos, kind, continued };
        if gesture == GesturePhase::Begin {
            self.reports.push(change(began, GesturePhase::Begin));
        }
        self.reports.push(change(next, if gesture == GesturePhase::Begin { GesturePhase::Update } else { gesture }));
        // A move leaves a content-sized layer its size; a resize fixes both.
        let placed = match (gesture, grip) {
            (GesturePhase::Cancel, _) => mode.restore.clone(),
            (_, Some(_)) => Placement::at(next),
            (_, None) => Placement { pos: Some(next.min.into()), size: current.size },
        };
        self.moved |= slot.write(placed);
        if matches!(gesture, GesturePhase::Commit | GesturePhase::Cancel) {
            mode = WindowMode::default();
        }
        *cx.core.memory.shown::<WindowMode>(window_mode_id(id)) = mode;
    }

    /// A drag of an edge pane's grip: the pane's extent follows the pointer, at least the least extent, and each step
    /// is reported as a `PlacementKind::Size` change of the pane's rect. While the gesture runs the pane shows its
    /// extent; at the commit a pane with no fixed extent keeps it in its session memory, once, and one whose caller
    /// fixes the extent shows the caller's from the next build. Escape, which cancels the press, restores the extent.
    fn pane_drag<C: Container>(&mut self, pane: &ShellPane, cx: &mut Cx<'_, C>) {
        let Some(edge) = pane_grip_edge(pane, cx.core.env.direction().is_rtl()) else { return };
        let Some(phase) = cx.core.results.get(&(PANE_GRIP, None)).map(|result| result.press.phase) else { return };
        let (origin, pos, gesture) = match phase {
            PressPhase::DragStart { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Begin),
            PressPhase::Drag { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Update),
            PressPhase::Release { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Commit),
            PressPhase::Cancel { start, .. } => (start.origin, None, GesturePhase::Cancel),
            _ => return,
        };
        let (id, rect) = (cx.core.id, cx.core.rect);
        let least = self.least;
        let horizontal = matches!(edge, ResizeEdge::Left | ResizeEdge::Right);
        let along = |size: Vec2| if horizontal { size.x } else { size.y };
        let slot = *cx.core.memory.session::<PaneGrip>(pane_grip_id(id));
        let began = match slot.began {
            Some(began) if gesture != GesturePhase::Begin => began,
            _ => along(rect.size()),
        };
        let moved = pos.map_or(0.0, |pos| match edge {
            ResizeEdge::Right => pos.x - origin.x,
            ResizeEdge::Left => origin.x - pos.x,
            ResizeEdge::Bottom => pos.y - origin.y,
            _ => origin.y - pos.y,
        });
        let extent = (began + moved).max(least);
        let resized = |extent: f32| match edge {
            ResizeEdge::Right => Rect::from_min_size(rect.min, Vec2::new(extent, rect.height())),
            ResizeEdge::Left => Rect::from_min_max(Pos2::new(rect.max.x - extent, rect.min.y), rect.max),
            ResizeEdge::Bottom => Rect::from_min_size(rect.min, Vec2::new(rect.width(), extent)),
            _ => Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - extent), rect.max),
        };
        let change =
            |rect: Rect, phase: GesturePhase| PlacementChange { rect, phase, pointer: pos, kind: PlacementKind::Size(edge), continued: false };
        if gesture == GesturePhase::Begin {
            self.reports.push(change(resized(began), GesturePhase::Begin));
        }
        let (phase, extent) = match gesture {
            GesturePhase::Begin => (GesturePhase::Update, extent),
            GesturePhase::Cancel => (GesturePhase::Cancel, began),
            phase => (phase, extent),
        };
        self.reports.push(change(resized(extent), phase));
        let ended = matches!(phase, GesturePhase::Commit | GesturePhase::Cancel);
        *cx.core.memory.session::<PaneGrip>(pane_grip_id(id)) =
            if ended { PaneGrip::default() } else { PaneGrip { began: Some(began), extent: Some(extent) } };
        if phase == GesturePhase::Commit && pane.extent.is_none() {
            *cx.core.memory.session::<Option<f32>>(pane_extent_id(id)) = Some(extent);
        }
        // The pane showed in this build at the extent of the last step (or its rest extent).
        self.moved |= (along(rect.size()) - extent).abs() > 0.5;
    }
}

impl<C: Container> ShapeHooks<C> for ContainerShape<'_> {
    /// Only a container with a band or a grip adds steps; its body runs in its own layout either way.
    fn shaped(&self) -> bool {
        self.has_parts()
    }

    fn parts(&mut self, _comp: &mut C, rect: Rect, pcx: &mut PartsCx<'_, C>) {
        // The reorder band lies under the header's own parts and children: its default priority loses to theirs, so
        // their buttons keep their clicks.
        if self.props.header.is_some_and(|header| header.band == DragBand::Reorder)
            && let Some(line) = self.geometry.line.filter(|line| line.height() > 0.0)
        {
            pcx.part(REORDER_BAND, line)
                .sense(Sense::DRAG)
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only("Cmd+] and Cmd+[ move the section, with Shift to the ends");
        }
        if let Some(overlay) = self.layer() {
            let held = pcx.stage.and_then(|(stage, id)| stage.recall::<WindowMode>(window_mode_id(id)).held);
            self.layer_parts(overlay, rect, held, pcx);
        }
        let props = self.props;
        if let Mount::Pane(pane) = &props.mount {
            self.pane_part(pane, rect, pcx);
        }
    }

    fn updated(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        if let Some(overlay) = self.layer() {
            self.layer_drag(overlay, cx);
        }
        let props = self.props;
        if let Mount::Pane(pane) = &props.mount {
            self.pane_drag(pane, cx);
        }
        for change in std::mem::take(&mut self.reports) {
            comp.placement_changed(&change);
        }
    }
}
