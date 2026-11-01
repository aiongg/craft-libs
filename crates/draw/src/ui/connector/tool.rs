//! [`ConnectorTool`]: the Connector tool on the tool host (task SH-T10).

use std::f64::consts::FRAC_PI_4;

use egui::{CursorIcon, Key, Modifiers, Painter, Pos2, Shape};
use kurbo::{Point, Rect};

use super::overlay::visible_path_handles;
use super::{
    ConnectionTarget, ConnectorDraft, ConnectorEnd, ConnectorEndKind, ConnectorEvent, ConnectorItem, ConnectorOverlay, ConnectorStyle,
    ConnectorToolLabels,
};
use crate::geometry::connector::{ConnectionSide, ConnectorPath, RouteEnd, Routing, reshaped, route, route_along};
use crate::ui::add_beside::{AddBesideHandles, AddSide};
use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::hit;
use craft_ui::canvas::{CanvasTool, CanvasView, Part, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode, ToolParts, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

/// The Connector tool (FigJam's, as FigCraft has it; DeckCraft's glue to numbered sites). The app
/// owns the value, sets the targets near the pointer, the routing and the selected connector each
/// pass, hands it to `ToolHost::show` and drains [`take_events`](Self::take_events). The tool
/// never changes the document.
///
/// - Hover: the target under the pointer shows its sites (attach points, `connector.attach` 4 pt
///   white discs with a `connector.line` 1.5 pt accent ring); the one an end would glue to is the
///   larger see-through ring (`connector.attach_active` 7, accent at `connector.attach_fill_alpha`
///   0.25).
/// - Draw: a press on a target or on empty canvas starts; the drag previews the route (an accent
///   polyline through [`route`](crate::geometry::connector::route), unless
///   [`custom_preview`](Self::custom_preview)); the release makes it. A drag under
///   `connector.click_slop` (4 screen points) makes nothing; one that ends on its start object
///   commits with `changed: false`. ⇧ turns a free end to 45° steps from the other end
///   ([`snap_free_end`](Self::snap_free_end), on by default).
/// - The selected connector ([`selected`](Self::selected)): its two ends are handles; dragging one
///   onto a target reattaches it there, onto empty canvas frees it (`Reattach`), with a rubber line
///   from the other end. One path handle per elbow run long enough to hold one, or one for a curve
///   ([`path_handles`](crate::geometry::connector::path_handles)); dragging one reports `Reshape`
///   ([`reshaped`](crate::geometry::connector::reshaped), always from the path the drag began
///   with). The selected connector's end objects must be among the [`targets`](Self::targets),
///   or its route is unknown and it has no handles.
/// - "+" buttons ([`connected`](Self::connected)): beside the given object, through
///   [`AddBesideHandles`](crate::ui::add_beside::AddBesideHandles); a press reports
///   `AddConnected`.
/// - Esc cancels the drag; Delete is the app's.
///
/// Keyboard parts: group [`ConnectorToolLabels::group`]; the end handles ("Start, attached to
/// 12", "End, free") and path handles ("Bend 1 of 2") are parts; Page keys pick, arrows
/// move a path handle by the nudge step or move a picked end to the next target in the group's
/// order.
#[derive(Debug, Default)]
pub struct ConnectorTool {
    targets: Vec<ConnectionTarget>,
    routing: Routing,
    selected: Option<ConnectorItem>,
    connected: Option<Rect>,
    snap_free_end: Option<bool>,
    custom_preview: bool,
    labels: Option<ConnectorToolLabels>,
    events: Vec<ConnectorEvent>,
    drag: Option<Drag>,
    /// The target and site under the pointer (hover and drag), for the attach points.
    hover: Option<(u64, Option<u32>)>,
    /// The picked part's key.
    picked: Option<u64>,
    /// The "+" button under the pointer, for the cursor and the hot look.
    plus: Option<AddSide>,
}

/// A gesture between passes.
#[derive(Clone, Debug)]
struct Drag {
    id: GestureId,
    kind: Kind,
    routing: Routing,
    /// The press on screen, for the click slop.
    press: Pos2,
    /// The drag has left the click slop and reported `Begin`.
    began: bool,
    /// The modifiers of the latest sample.
    modifiers: Modifiers,
    /// The latest value, `None` when the pointer is where no route exists.
    value: Option<Val>,
    /// The value differs from what the gesture started with.
    changed: bool,
    /// The last value reported, so a still pointer sends no update.
    sent: Option<Val>,
    /// The route (or rubber line) to draw, document units.
    preview: Vec<Point>,
}

#[derive(Clone, Debug)]
enum Kind {
    /// A new connector from `from`, which routes as `from_route`; `start_target` is the object it
    /// was pressed on.
    Draw { from: ConnectorEnd, from_route: RouteEnd, start_target: Option<u64> },
    /// The `end` of the selected connector, which was `start_end`; `fixed` is the other end's point
    /// and `other_target` its object.
    Reattach { end: ConnectorEndKind, fixed: Point, other_target: Option<u64>, start_end: ConnectorEnd },
    /// The path handle `portion`, of a connector whose path was `path` when the drag began.
    Reshape { portion: usize, from: RouteEnd, to: RouteEnd, path: Option<ConnectorPath> },
}

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Draw(ConnectorDraft),
    End(ConnectorEnd),
    Path(ConnectorPath),
}

#[derive(Clone, Copy)]
enum Phase {
    Begin,
    Update,
    Commit,
    Cancel,
}

/// What a press on the selected connector took.
#[derive(Clone, Copy)]
enum Grab {
    End(ConnectorEndKind),
    Path(usize),
}

/// What is at a point: the end a connector would have there.
#[derive(Clone, Debug)]
struct Placed {
    end: ConnectorEnd,
    route: RouteEnd,
    /// The target under the point (its sites show), whether or not the end glues to it.
    target: Option<u64>,
    /// The site the end glues to, as its index on the target.
    active: Option<u32>,
}

/// The outer part of a box where a point chooses a side, as a fraction of the half-size
/// (FigCraft's `side_at`: the inner half of the box chooses none).
const INNER: f64 = 0.5; // design: allow(a ratio, not a size)

/// A divisor floor for a box with no width or height.
const TINY: f64 = 1e-6; // design: allow(a numeric floor, not a size)

/// The side of box `b` a connector end at `p` chooses (FigCraft #1708): the one `p` is towards
/// anywhere in the box's outer half, whatever its proportions, so a whole edge picks its side. In
/// the middle of the box none is chosen ([`ConnectionSide::Auto`]).
fn side_at(b: Rect, p: Point) -> ConnectionSide {
    let c = b.center();
    let (x, y) = ((p.x - c.x) / (b.width() / 2.0).max(TINY), (p.y - c.y) / (b.height() / 2.0).max(TINY));
    if x.abs().max(y.abs()) < INNER {
        ConnectionSide::Auto
    } else if x.abs() >= y.abs() {
        if x > 0.0 { ConnectionSide::Right } else { ConnectionSide::Left }
    } else if y > 0.0 {
        ConnectionSide::Bottom
    } else {
        ConnectionSide::Top
    }
}

/// `to` turned about `from` to the nearest multiple of 45°, at the same distance.
fn snapped(from: Point, to: Point) -> Point {
    let d = to - from;
    let len = d.hypot();
    if len == 0.0 {
        return to;
    }
    let angle = (d.y.atan2(d.x) / FRAC_PI_4).round() * FRAC_PI_4;
    Point::new(from.x + len * angle.cos(), from.y + len * angle.sin())
}

/// The object `end` is glued to.
fn glued(end: &ConnectorEnd) -> Option<u64> {
    match end {
        ConnectorEnd::Glued { target, .. } => Some(*target),
        ConnectorEnd::Free(_) => None,
    }
}

/// Where a route end sits: the point, or the middle of the named side (the middle of the box for
/// `Auto`).
fn anchor(end: RouteEnd) -> Point {
    match end {
        RouteEnd::Point(p) | RouteEnd::On(_, _, p) => p,
        RouteEnd::Box(b, side) => side.point(b),
    }
}

/// The index of `side` among the default sites.
fn side_index(side: ConnectionSide) -> Option<u32> {
    ConnectionSide::ALL.iter().position(|&s| s == side).and_then(|i| u32::try_from(i).ok())
}

fn edit<T: Clone>(d: &Drag, phase: Phase, value: Option<&T>) -> Option<EditEvent<T>> {
    Some(match phase {
        Phase::Begin => EditEvent::begin(d.id),
        Phase::Update => EditEvent::update(d.id, value?.clone(), d.modifiers),
        Phase::Commit => EditEvent::commit(d.id, value?.clone(), d.changed, d.modifiers),
        Phase::Cancel => EditEvent::cancel(d.id, CancelReason::Escape),
    })
}

/// The event of `phase` for gesture `d`; none when it has no value to report.
fn event(d: &Drag, phase: Phase) -> Option<ConnectorEvent> {
    match (&d.kind, &d.value) {
        (Kind::Draw { .. }, Some(Val::Draw(v))) => Some(ConnectorEvent::Draw(edit(d, phase, Some(v))?)),
        (Kind::Draw { .. }, _) => Some(ConnectorEvent::Draw(edit::<ConnectorDraft>(d, phase, None)?)),
        (Kind::Reattach { end, .. }, Some(Val::End(v))) => Some(ConnectorEvent::Reattach { end: *end, event: edit(d, phase, Some(v))? }),
        (Kind::Reattach { end, .. }, _) => Some(ConnectorEvent::Reattach { end: *end, event: edit::<ConnectorEnd>(d, phase, None)? }),
        (Kind::Reshape { .. }, Some(Val::Path(v))) => Some(ConnectorEvent::Reshape(edit(d, phase, Some(v))?)),
        (Kind::Reshape { .. }, _) => Some(ConnectorEvent::Reshape(edit::<ConnectorPath>(d, phase, None)?)),
    }
}

impl ConnectorTool {
    /// A tool drawing straight connectors.
    pub fn new() -> Self {
        Self::default()
    }

    /// The objects an end may glue to this pass, topmost first: the app passes those under or near
    /// the pointer (or every visible one). A target with no sites offers its four side middles.
    pub fn targets(&mut self, targets: &[ConnectionTarget]) -> &mut Self {
        self.targets.clear();
        self.targets.extend_from_slice(targets);
        self
    }

    /// The routing new connectors get.
    pub fn routing(&mut self, routing: Routing) -> &mut Self {
        self.routing = routing;
        self
    }

    /// The selected connector, whose ends and path handles the tool edits.
    pub fn selected(&mut self, connector: Option<&ConnectorItem>) -> &mut Self {
        self.selected = connector.cloned();
        self
    }

    /// The object whose "+" buttons the tool shows (document bounds), `None` for none.
    pub fn connected(&mut self, object: Option<Rect>) -> &mut Self {
        self.connected = object;
        self
    }

    /// Whether ⇧ turns a free end to 45° steps from the other end; on by default (Illustrator and
    /// Figma; FigCraft turns it off).
    pub fn snap_free_end(&mut self, on: bool) -> &mut Self {
        self.snap_free_end = Some(on);
        self
    }

    /// The app previews the connector in the document from `Update` events; the tool draws no
    /// route. Off by default.
    pub fn custom_preview(&mut self, on: bool) -> &mut Self {
        self.custom_preview = on;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(&mut self, labels: ConnectorToolLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<ConnectorEvent> {
        std::mem::take(&mut self.events)
    }

    /// The target and site the hover shows (tests).
    #[cfg(test)]
    pub(super) fn hovered(&self) -> Option<(u64, Option<u32>)> {
        self.hover
    }

    /// The end at `at`: glued to the nearest site of the topmost target containing `at` within
    /// the reach (`connector.attach_reach` screen points, FigCraft's `ATTACH_PX`), else the side
    /// the point is towards, else free. With ⌘ or Ctrl held, a target with no sites takes the
    /// point itself.
    fn end_at(&self, at: Point, view: &CanvasView, style: &ConnectorStyle, modifiers: Modifiers) -> Placed {
        // inv-figcraft: crates/engine/src/boards/board/tools.rs:98-124 (the side within `ATTACH_PX`
        // :12, else the side the pointer is towards `side_at`, ⌘ glues at the point).
        // inv-deckcraft: crates/engine/src/connect.rs:1-45 `sites`, `nearest_site` (a target with
        // numbered sites glues only within the radius of one).
        let free = Placed { end: ConnectorEnd::Free(at), route: RouteEnd::Point(at), target: None, active: None };
        let reach = view.doc_len(style.attach_reach);
        let slack = view.doc_len(style.target_tolerance);
        let Some(t) = self.targets.iter().find(|t| t.bounds.abs().inflate(slack, slack).contains(at)) else { return free };
        let b = t.bounds.abs();
        let target = Some(t.id);
        let dist = |p: Point| (p - at).hypot();
        if !t.sites.is_empty() {
            let nearest = t.sites.iter().filter(|s| dist(s.point) <= reach).min_by(|x, y| dist(x.point).total_cmp(&dist(y.point)));
            return match nearest {
                Some(s) => Placed {
                    end: ConnectorEnd::Glued { target: t.id, side: s.side, site: Some(s.index), at: None },
                    route: RouteEnd::On(b, s.side, s.point),
                    target,
                    active: Some(s.index),
                },
                None => Placed { target, ..free },
            };
        }
        if modifiers.command && b.width() > 0.0 && b.height() > 0.0 {
            let frac = [((at.x - b.x0) / b.width()).clamp(0.0, 1.0), ((at.y - b.y0) / b.height()).clamp(0.0, 1.0)];
            let p = Point::new(b.x0 + frac[0] * b.width(), b.y0 + frac[1] * b.height());
            return Placed {
                end: ConnectorEnd::Glued { target: t.id, side: ConnectionSide::Auto, site: None, at: Some(frac) },
                route: RouteEnd::On(b, ConnectionSide::Auto, p),
                target,
                active: None,
            };
        }
        let near = ConnectionSide::ALL.iter().map(|&s| (s, dist(s.point(b)))).filter(|&(_, d)| d <= reach).min_by(|x, y| x.1.total_cmp(&y.1));
        let side = near.map_or_else(|| side_at(b, at), |(s, _)| s);
        Placed {
            end: ConnectorEnd::Glued { target: t.id, side, site: None, at: None },
            route: RouteEnd::Box(b, side),
            target,
            active: side_index(side),
        }
    }

    /// The end under the pointer for a gesture, with ⇧ turning a free end to 45° steps from
    /// `origin`.
    fn place(&self, pointer: &ToolPointer, view: &CanvasView, style: &ConnectorStyle, origin: Option<Point>) -> Placed {
        let placed = self.end_at(pointer.doc, view, style, pointer.modifiers);
        match (origin, placed.end) {
            (Some(o), ConnectorEnd::Free(at)) if pointer.modifiers.shift && self.snap_free_end.unwrap_or(true) => {
                let at = snapped(o, at);
                Placed { end: ConnectorEnd::Free(at), route: RouteEnd::Point(at), ..placed }
            }
            _ => placed,
        }
    }

    /// `end` as the geometry routes it; `None` when its object is not among the targets.
    fn route_end(&self, end: &ConnectorEnd) -> Option<RouteEnd> {
        match *end {
            ConnectorEnd::Free(p) => Some(RouteEnd::Point(p)),
            ConnectorEnd::Glued { target, side, site, at } => {
                let t = self.targets.iter().find(|t| t.id == target)?;
                let b = t.bounds.abs();
                if let Some(f) = at {
                    return Some(RouteEnd::On(b, side, Point::new(b.x0 + f[0] * b.width(), b.y0 + f[1] * b.height())));
                }
                match site.and_then(|i| t.side_sites().into_iter().find(|s| s.index == i)) {
                    Some(s) => Some(RouteEnd::On(b, side, s.point)),
                    None => Some(RouteEnd::Box(b, side)),
                }
            }
        }
    }

    /// The selected connector's ends as routed, and its points.
    fn geometry(&self) -> Option<(RouteEnd, RouteEnd, Vec<Point>)> {
        let item = self.selected.as_ref()?;
        let (from, to) = (self.route_end(&item.from)?, self.route_end(&item.to)?);
        let points = route_along(from, to, item.routing, item.path.as_ref())?;
        Some((from, to, points))
    }

    /// The handle of the selected connector under the screen point `at`: an end (the nearer one),
    /// else the nearest path handle.
    fn handle_at(&self, at: Pos2, cx: &ToolContext<'_>) -> Option<Grab> {
        // inv-figcraft: crates/engine/src/boards/board/connectors.rs:184-191 `end_handle_at` and
        // reshape.rs:73-79 `path_handle_at` (the nearest within `END_HANDLE_PX` 8).
        let item = self.selected.as_ref()?;
        let (_, _, points) = self.geometry()?;
        let style = Self::style(cx.tokens);
        let view = cx.view;
        let dist = |p: Point| hit::point(at, view.to_screen(p));
        let ends = [(ConnectorEndKind::Start, points.first()), (ConnectorEndKind::End, points.last())];
        let end =
            ends.into_iter().filter_map(|(k, p)| Some((k, dist(*p?)))).filter(|&(_, d)| d <= style.handle_reach).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((kind, _)) = end {
            return Some(Grab::End(kind));
        }
        visible_path_handles(&points, item.routing, view, &style)
            .into_iter()
            .map(|(portion, p)| (portion, dist(p)))
            .filter(|&(_, d)| d <= style.handle_reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(portion, _)| Grab::Path(portion))
    }

    /// The "+" button under the screen point `at`.
    fn plus_at(&self, at: Pos2, cx: &ToolContext<'_>) -> Option<AddSide> {
        let object = self.connected?;
        AddBesideHandles::new("connector-add-beside", object).at(cx.view, &AddBesideHandles::style(cx.tokens), at)
    }

    /// Follows the pointer: the value, the preview and the hover, and the `Begin` and `Update`
    /// events once the pointer is past the click slop.
    fn advance(&mut self, d: &mut Drag, pointer: &ToolPointer, view: &CanvasView, style: &ConnectorStyle, update: bool) {
        d.modifiers = pointer.modifiers;
        self.hover = None;
        match d.kind.clone() {
            Kind::Draw { from, from_route, start_target } => {
                let placed = self.place(pointer, view, style, Some(anchor(from_route)));
                let draft =
                    route(from_route, placed.route, d.routing).map(|points| ConnectorDraft { from, to: placed.end, routing: d.routing, points });
                // Ending on the object it started from makes nothing.
                let same = start_target.is_some() && glued(&placed.end) == start_target;
                d.changed = !same;
                d.preview = if same { Vec::new() } else { draft.as_ref().map(|x| x.points.clone()).unwrap_or_default() };
                d.value = draft.map(Val::Draw);
                self.hover = placed.target.map(|t| (t, placed.active));
            }
            Kind::Reattach { fixed, other_target, start_end, .. } => {
                let placed = self.place(pointer, view, style, Some(fixed));
                let onto_other = glued(&placed.end).is_some() && glued(&placed.end) == other_target;
                d.changed = !onto_other && placed.end != start_end;
                let tip = if let ConnectorEnd::Free(p) = placed.end { p } else { pointer.doc };
                d.preview = vec![fixed, tip];
                d.value = Some(Val::End(placed.end));
                self.hover = placed.target.filter(|&t| Some(t) != other_target).map(|t| (t, placed.active));
            }
            Kind::Reshape { portion, from, to, path } => {
                let now = reshaped(from, to, d.routing, path.as_ref(), portion, pointer.doc);
                d.changed = now.is_some() && now != path;
                d.preview = now.as_ref().and_then(|p| route_along(from, to, d.routing, Some(p))).unwrap_or_default();
                d.value = now.map(Val::Path);
            }
        }
        if !d.began && pointer.screen.distance(d.press) >= style.click_slop {
            d.began = true;
            self.events.extend(event(d, Phase::Begin));
        }
        if update && d.began && d.value.is_some() && d.value != d.sent {
            self.events.extend(event(d, Phase::Update));
            d.sent.clone_from(&d.value);
        }
    }

    /// The end `end` of the selected connector moved to the next (or previous) target in the
    /// targets' order, skipping the object the other end is on.
    fn stepped_end(&self, end: ConnectorEndKind, forward: bool) -> Option<ConnectorEnd> {
        let item = self.selected.as_ref()?;
        let (mine, other) = if end == ConnectorEndKind::Start { (&item.from, &item.to) } else { (&item.to, &item.from) };
        let skip = glued(other);
        let candidates: Vec<&ConnectionTarget> = self.targets.iter().filter(|t| Some(t.id) != skip).collect();
        let now = glued(mine).and_then(|id| candidates.iter().position(|t| t.id == id));
        let n = candidates.len();
        let next = match (now, forward) {
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
            (None, true) => 0,
            (None, false) => n.checked_sub(1)?,
        };
        let t = candidates.get(next)?;
        Some(match t.sites.first() {
            Some(s) => ConnectorEnd::Glued { target: t.id, side: s.side, site: Some(s.index), at: None },
            None => ConnectorEnd::Glued { target: t.id, side: ConnectionSide::Auto, site: None, at: None },
        })
    }

    fn words(&self, cx: &ToolContext<'_>) -> ConnectorToolLabels {
        self.labels.clone().unwrap_or_else(|| ConnectorToolLabels::of_ctx(&cx.canvas.ctx))
    }
}

impl CanvasTool for ConnectorTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the crosshair; the pointing hand over a "+" button; the move arrow over a handle.
        if let Some(d) = &self.drag {
            return ToolCursor::System(if matches!(d.kind, Kind::Draw { .. }) { CursorIcon::Crosshair } else { CursorIcon::Move });
        }
        let Some(p) = cx.pointer else { return ToolCursor::System(CursorIcon::Crosshair) };
        if self.plus_at(p.screen, cx).is_some() {
            return ToolCursor::System(CursorIcon::PointingHand);
        }
        if self.handle_at(p.screen, cx).is_some() {
            return ToolCursor::System(CursorIcon::Move);
        }
        ToolCursor::System(CursorIcon::Crosshair)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // inv-figcraft: crates/engine/src/boards/board/tools.rs:102-130 (press on an object or the
        // canvas), crates/engine/src/canvas/tools/drag.rs:27-32 (an end handle of the selected
        // connector first, then a path handle).
        self.drag = None;
        let style = Self::style(cx.tokens);
        let id = GestureId::next();
        let drag = |kind: Kind, routing: Routing| Drag {
            id,
            kind,
            routing,
            press: pointer.screen,
            began: false,
            modifiers: pointer.modifiers,
            value: None,
            changed: false,
            sent: None,
            preview: Vec::new(),
        };
        if let Some(side) = self.plus_at(pointer.screen, cx) {
            self.events.push(ConnectorEvent::AddConnected { side });
            return ToolAction::Done;
        }
        if let Some(grab) = self.handle_at(pointer.screen, cx)
            && let Some(item) = self.selected.clone()
            && let Some((from, to, points)) = self.geometry()
        {
            self.drag = Some(match grab {
                Grab::End(end) => {
                    let (start_end, other, fixed) = match end {
                        ConnectorEndKind::Start => (item.from, item.to, points.last()),
                        ConnectorEndKind::End => (item.to, item.from, points.first()),
                    };
                    self.picked = Some(if end == ConnectorEndKind::Start { 0 } else { 1 });
                    let kind = Kind::Reattach { end, fixed: fixed.copied().unwrap_or(pointer.doc), other_target: glued(&other), start_end };
                    drag(kind, item.routing)
                }
                Grab::Path(portion) => {
                    self.picked = Some(2 + portion as u64);
                    drag(Kind::Reshape { portion, from, to, path: item.path.clone() }, item.routing)
                }
            });
            return ToolAction::Claimed;
        }
        let placed = self.end_at(pointer.doc, cx.view, &style, pointer.modifiers);
        let kind = Kind::Draw { from: placed.end, from_route: placed.route, start_target: glued(&placed.end) };
        self.drag = Some(drag(kind, self.routing));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // inv-figcraft: crates/engine/src/boards/board/tools.rs:102-245 (the preview route, the
        // rubber line of a reattach).
        let Some(mut d) = self.drag.take() else { return };
        self.advance(&mut d, pointer, cx.view, &Self::style(cx.tokens), true);
        self.drag = Some(d);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // inv-figcraft: crates/engine/src/boards/board/tools.rs:129 and :218, reshape.rs:111 (a drag
        // under 4 points makes nothing; ending on the start object makes nothing).
        let Some(mut d) = self.drag.take() else { return };
        self.advance(&mut d, pointer, cx.view, &Self::style(cx.tokens), false);
        self.hover = None;
        if !d.began {
            return;
        }
        // A route that can't be placed there (a reshape the geometry refuses) makes nothing.
        let phase = if d.value.is_some() { Phase::Commit } else { Phase::Cancel };
        self.events.extend(event(&d, phase));
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // inv-figcraft: crates/engine/src/boards/board/tools.rs:171-174 `connector_hover`.
        // new: keeps `hover` for the attach points and `plus` for the "+" buttons' hot look.
        let placed = self.end_at(pointer.doc, cx.view, &Self::style(cx.tokens), pointer.modifiers);
        self.hover = placed.target.map(|t| (t, placed.active));
        self.plus = self.plus_at(pointer.screen, cx);
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: the running `Draw`, `Reattach` or `Reshape` reports `Cancel`; drops the drag.
        self.hover = None;
        if let Some(d) = self.drag.take()
            && d.began
        {
            self.events.extend(event(&d, Phase::Cancel));
        }
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: arrows move the picked path handle (`NudgeKeys::from_key`) or move the picked end to
        // the next target (AccessKit keyboard path).
        if !key.pressed || self.drag.is_some() || key.key == Key::Escape {
            return false;
        }
        let (Some(picked), Some(nudge)) = (self.picked, NudgeKeys::new().from_key(key)) else { return false };
        let Some(item) = self.selected.clone() else { return false };
        let m = key.modifiers;
        if picked < 2 {
            let end = if picked == 0 { ConnectorEndKind::Start } else { ConnectorEndKind::End };
            let forward = nudge.by.x + nudge.by.y > 0.0;
            if let Some(to) = self.stepped_end(end, forward) {
                self.events.push(ConnectorEvent::Reattach { end, event: EditEvent::set(to, m) });
            }
            return true;
        }
        let Some((from, to, points)) = self.geometry() else { return false };
        let portion = usize::try_from(picked - 2).unwrap_or(usize::MAX);
        let style = Self::style(cx.tokens);
        let Some((_, at)) = visible_path_handles(&points, item.routing, cx.view, &style).into_iter().find(|&(p, _)| p == portion) else {
            return false;
        };
        if let Some(path) = reshaped(from, to, item.routing, item.path.as_ref(), portion, at + nudge.by) {
            self.events.push(ConnectorEvent::Reshape(EditEvent::set(path, m)));
        }
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // inv-figcraft: crates/ui-egui/src/canvas/overlays.rs:407-418 `attach_points`, :364-376 (path
        // and end handles of the selected connector), :73-79 (the drop target outlined).
        // new: `ConnectorOverlay::paint` for the selected connector, the draft route, the "+"
        // buttons through `AddBesideHandles::paint`.
        let style = Self::style(cx.tokens);
        let view = cx.view;
        if let Some((id, active)) = self.hover
            && let Some(t) = self.targets.iter().find(|t| t.id == id)
        {
            painter.add(Shape::closed_line(view.quad(t.bounds.abs()).to_vec(), style.drop_outline));
            ConnectorOverlay::paint_sites(painter, view, &style, t, active);
        }
        let Some(d) = self.drag.as_ref().filter(|d| d.began) else {
            if self.drag.is_none() {
                if let Some(item) = &self.selected
                    && let Some((_, _, points)) = self.geometry()
                {
                    ConnectorOverlay::new(&points, item.routing).paint(painter, view, &style);
                }
                if let Some(object) = self.connected {
                    AddBesideHandles::new("connector-add-beside", object).paint(painter, view, &AddBesideHandles::style(cx.tokens), self.plus);
                }
            }
            return;
        };
        match d.kind {
            Kind::Draw { .. } if !self.custom_preview => ConnectorOverlay::new(&d.preview, d.routing).handles(false).paint(painter, view, &style),
            Kind::Draw { .. } => {}
            Kind::Reattach { .. } => {
                let line: Vec<Pos2> = d.preview.iter().map(|&p| view.to_screen(p)).collect();
                painter.add(Shape::line(line, style.line));
            }
            Kind::Reshape { .. } => ConnectorOverlay::new(&d.preview, d.routing).paint(painter, view, &style),
        }
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // new: `Point` (a free end and a path handle snap; a glued end takes its site).
        Some(SnapGesture::Point)
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: the selected connector's ends (keys 0, 1) and path handles (2 + portion), named by
        // `ConnectorToolLabels`.
        let item = self.selected.as_ref()?;
        let (_, _, points) = self.geometry()?;
        let labels = self.words(cx);
        let style = Self::style(cx.tokens);
        let view = cx.view;
        let reach = style.handle_reach;
        let around = |p: Point| egui::Rect::from_center_size(view.to_screen(p), egui::vec2(2.0 * reach, 2.0 * reach));
        let end_name = |end: &ConnectorEnd, attached: &str, free: &str| match end {
            ConnectorEnd::Glued { target, .. } => craft_i18n::fmt(attached, &[("target", &target.to_string())]),
            ConnectorEnd::Free(_) => free.to_owned(),
        };
        let mut parts = Vec::new();
        for (key, p, name) in [
            (0, points.first(), end_name(&item.from, &labels.start_attached, &labels.start_free)),
            (1, points.last(), end_name(&item.to, &labels.end_attached, &labels.end_free)),
        ] {
            if let Some(&p) = p {
                parts.push(Part { key, name, value: None, rect: around(p), selected: self.picked == Some(key) });
            }
        }
        let handles = visible_path_handles(&points, item.routing, view, &style);
        let count = handles.len().to_string();
        for (n, (portion, p)) in handles.into_iter().enumerate() {
            let key = 2 + portion as u64;
            let name = craft_i18n::fmt(&labels.bend, &[("index", &(n + 1).to_string()), ("count", &count)]);
            parts.push(Part { key, name, value: None, rect: around(p), selected: self.picked == Some(key) });
        }
        Some(ToolParts::new(labels.group.clone(), parts))
    }

    fn picked(&self) -> Option<u64> {
        self.picked
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.picked = key;
    }

    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: the connector being drawn, as one graphics object over the route's bounds.
        let d = self.drag.as_ref().filter(|d| d.began && matches!(d.kind, Kind::Draw { .. }) && d.preview.len() >= 2)?;
        let bounds = Rect::from_points(d.preview[0], d.preview[1]);
        let bounds = d.preview.iter().fold(bounds, |r, &p| r.union_pt(p));
        Some(ToolNode::new(self.words(cx).group.clone()).bounds(bounds))
    }
}
