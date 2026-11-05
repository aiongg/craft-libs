//! [`PerspectiveWarp`]: Perspective Warp's planes on the canvas, drawn, linked, snapped and dragged
//! by pointer and keyboard (task TR-T13).

use egui::{AsIdSalt, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Response, Shape, StrokeKind, Ui, vec2};
use kurbo::{Point, Vec2};

use craft_ui::behavior::{ClaimedKey, KeyClaim, nudge};
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, RulerUnits};
use craft_ui::composites::curve_editor::{fill, take_exact};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{CancelReason, EditPolicy, Gesture};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::popup_list;
use craft_ui::primitives::Handle;
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::words;

use super::{PLANE_MINIMUM, PerspectiveEdit, PerspectiveSession, Plane, PlaneCorner, PlaneMode, linked_corners};
use crate::ui::warp::PerspectiveWarpStyle;

words! {
    /// The AccessKit words of [`PerspectiveWarp`]. `Default` is English.
    pub struct PerspectiveWarpLabels {
        group: "perspectiveWarp.group" = "Perspective Warp";
        corner: "perspectiveWarp.corner" = "Plane {plane}, corner {corner}";
        linked: "perspectiveWarp.linked" = "linked";
        position: "perspectiveWarp.position" = "{x}, {y}";
    }
}

/// An open Perspective Warp's planes over the picture (the warped picture is drawn by the
/// transform preview through the app's map).
///
/// Pointer (strict `<` on `transform.hit`):
/// - Layout: a drag on the picture away from corners draws a plane (a rubber band), added on
///   release when at least [`PLANE_MINIMUM`](super::PLANE_MINIMUM) each way, its corners snapped
///   to other planes' corners within `transform.hit`; a drag on a corner moves it and every corner
///   linked to it, snapping to another plane's corner, which links them.
/// - Warp: a drag on a corner moves its target and the linked corners'; the picture follows; the
///   layout is kept.
/// - A drag holds its corners by value: when the session's
///   [generation](PerspectiveSession::generation) changes during it, the drag ends (Cancel).
/// - Esc during a drag cancels it; with no drag it discards the session.
///
/// Keys (one Tab stop, `PartFocus`): Page Down and Up pick the next and previous corner (plane by
/// plane, clockwise); arrows move the picked corner and its linked corners (⇧ ten); in Layout,
/// Insert adds [`PerspectiveSession::inserted_plane`] and Delete or Backspace removes the picked
/// corner's plane; Enter applies.
///
/// AccessKit: a group; each corner a graphics object "Plane 2, corner 3" with value "x, y",
/// "linked" when linked. Nothing mirrors in a right-to-left interface.
pub struct PerspectiveWarp<'a> {
    id_salt: IdSalt,
    session: &'a PerspectiveSession,
    labels: Option<PerspectiveWarpLabels>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

/// What [`PerspectiveWarp::show`] reported this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct PerspectiveWarpResponse {
    /// The overlay's widget id.
    pub id: Id,
    /// The edits in order; the app applies each with [`PerspectiveSession::apply`].
    pub edits: Vec<PerspectiveEdit>,
    /// Enter: the app commits ([`PerspectiveSession::result`]).
    pub apply: bool,
    /// Esc with no drag running.
    pub discard: bool,
    /// Whether the overlay owns the canvas's pointer claim this pass.
    pub claimed: bool,
    /// The corner under the pointer.
    pub hovered: Option<PlaneCorner>,
}

impl<'a> PerspectiveWarp<'a> {
    /// The planes of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PerspectiveSession) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, labels: None, units: None, modifiers: None, policy: EditPolicy::default() }
    }

    /// Sets the words; the installed language by default.
    pub fn labels(mut self, labels: PerspectiveWarpLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The ruler units of the corners' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The modifiers every test reads in place of the input's.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Sets how the overlay reports gestures (live, committed, cancelled).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The look and reach in `tokens`.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PerspectiveWarpStyle {
        PerspectiveWarpStyle::of(tokens)
    }

    /// Draws the planes (thirds, outline, corners), the rubber band and the picked corner's focus
    /// ring over `view`, and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PerspectiveWarpResponse {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:202 (`pointer`), :284 (`draw`), :82
        // (`corner_at`), :97 (`snap`), :238 (the 4-unit minimum).
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = PerspectiveWarp::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let labels = self.labels.clone().unwrap_or_else(|| PerspectiveWarpLabels::of(ui));
        let enabled = ui.is_enabled();
        let mods = self.modifiers.unwrap_or_else(|| ctx.input(|i| i.modifiers));
        let planes = self.session.planes();
        let warp = self.session.mode() == PlaneMode::Warp;
        let mut out = PerspectiveWarpResponse { id, edits: Vec::new(), apply: false, discard: false, claimed: false, hovered: None };
        let mut picked: Option<PlaneCorner> =
            ctx.data(|d| d.get_temp(picked_key(id))).filter(|c: &PlaneCorner| c.plane < planes.len() && c.corner < 4);

        // A drag whose press ended while the overlay was not shown is dropped, and its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.as_ref().is_some_and(|d| d.waits);
        if let Some(d) = &mut drag
            && d.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                Turn::Runs => d.waits = false,
                Turn::Now => drag = None,
            }
        }
        if enabled && drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            let pressed = self.press_pass(&ctx, canvas, view, &style, id);
            if let Some(Pressed { drag: started, pick }) = pressed {
                picked = pick.or(picked);
                drag = Some(started);
            }
        }

        let reported = Gesture::<Vec<Plane>>::reported(ui, id).map(|(_, value)| value);
        let host: Vec<Plane> = match &drag {
            Some(d) if d.began && !d.cancelled && (planes == d.base || reported.as_deref() == Some(planes)) => {
                reported.clone().unwrap_or_else(|| d.base.clone())
            }
            _ => planes.to_vec(),
        };
        let mut gesture = Gesture::<Vec<Plane>>::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(enabled);
        if let Some(d) = &mut drag
            && !d.cancelled
        {
            if d.generation != self.session.generation() {
                // The planes were replaced from outside: the indices the drag holds are stale.
                d.cancelled = true;
                gesture.cancel(CancelReason::External);
            } else if d.began && !gesture.is_active() {
                d.cancelled = true;
            }
        }
        // Esc before a corner's first move, or during a new plane's rubber band, ends the press.
        if let Some(d) = &mut drag
            && !d.cancelled
            && (!d.began || matches!(d.kind, Kind::Plane { .. }))
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            d.cancelled = true;
        }
        if let Some(d) = &mut drag
            && !d.waits
        {
            self.track_pass(&ctx, view, &style, d, mods, &mut gesture, &mut picked);
        }

        let shown: Vec<Plane> = gesture.shown().clone();
        let picked = picked.filter(|c| c.plane < shown.len());
        let parts = self.parts(ui, view, &labels, &shown);
        let focus =
            PartFocus::new(id, labels.group.clone()).parts(&parts).picked(picked.map(|c| key_of(c) as u64)).focusable(enabled).show(ui, canvas);
        let mut picked = match focus.pick {
            Some(Some(key)) => Some(PlaneCorner { plane: key as usize / 4, corner: key as usize % 4 }),
            Some(None) => None,
            None => picked,
        };
        if focus.focused {
            KeyClaim::claim(&ctx, id, &CLAIMED);
        }
        let idle = drag.as_ref().is_none_or(|d| d.cancelled);
        self.keys(ui, idle, focus.focused, &mut gesture, &shown, &mut picked, &mut out);

        let dragging = drag.as_ref().filter(|d| !d.cancelled && (d.began || matches!(d.kind, Kind::Plane { .. })));
        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if dragging.is_some() {
            ctx.set_cursor_icon(if matches!(dragging.map(|d| &d.kind), Some(Kind::Plane { .. })) { CursorIcon::Crosshair } else { CursorIcon::Move });
        } else if drag.is_none()
            && !other
            && let Some(pointer) = canvas.hover_pos()
        {
            out.hovered = corner_at(view, planes, warp, pointer, style.hit);
            ctx.set_cursor_icon(if out.hovered.is_some() {
                CursorIcon::Move
            } else if warp {
                CursorIcon::Default
            } else {
                CursorIcon::Crosshair
            });
        }

        if let Some(event) = gesture.finish() {
            out.edits.push(PerspectiveEdit::Planes(event));
        }
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        let band = drag.as_ref().and_then(|d| match (&d.kind, d.cur) {
            (Kind::Plane { start }, Some(cur)) if !d.cancelled => Some(Rect::from_two_pos(view.to_screen(*start), cur)),
            _ => None,
        });
        let held = drag.filter(|d| d.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|data| {
            match held {
                Some(d) => {
                    data.insert_temp(drag_key(id), d);
                }
                None => {
                    data.remove::<Drag>(drag_key(id));
                }
            }
            match picked {
                Some(c) => {
                    data.insert_temp(picked_key(id), c);
                }
                None => {
                    data.remove::<PlaneCorner>(picked_key(id));
                }
            }
        });

        let painter = ui.painter_at(view.rect);
        paint_planes(&painter, &style, view, &shown, warp, band, picked.filter(|_| focus.keyboard));
        out
    }

    /// A press at `origin` on a corner, or (in Layout) on the picture: the drag to track and the
    /// corner it picks. `None` where the press is the tool's.
    fn press_pass(&self, ctx: &Context, canvas: &Response, view: &CanvasView, style: &PerspectiveWarpStyle, id: Id) -> Option<Pressed> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let planes = self.session.planes();
        let warp = self.session.mode() == PlaneMode::Warp;
        let (kind, pick) = if let Some(hit) = corner_at(view, planes, warp, press.origin, style.hit) {
            let at = corners(&planes[hit.plane], warp)[hit.corner];
            // Every corner linked to this one moves with it.
            let group = linked_corners(planes).into_iter().find(|g| g.contains(&hit)).unwrap_or_else(|| vec![hit]);
            (Kind::Corner { group, offset: at - view.to_doc(press.origin) }, Some(hit))
        } else if warp {
            return None;
        } else {
            (Kind::Plane { start: snap(view, planes, view.to_doc(press.origin), style.hit, &[]) }, None)
        };
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        let drag = Drag {
            press,
            kind,
            origin: press.origin,
            cur: None,
            began: false,
            cancelled: false,
            waits,
            generation: self.session.generation(),
            base: planes.to_vec(),
        };
        Some(Pressed { drag, pick })
    }

    /// One pass of a drag read from the input: begins a corner drag on the first move, updates it,
    /// ends it on the release; a new plane follows the rubber band and is added on the release.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn track_pass(
        &self,
        ctx: &Context,
        view: &CanvasView,
        style: &PerspectiveWarpStyle,
        d: &mut Drag,
        mods: Modifiers,
        gesture: &mut Gesture<Vec<Plane>>,
        picked: &mut Option<PlaneCorner>,
    ) {
        if d.cancelled {
            return;
        }
        let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
        let layout = self.session.mode() == PlaneMode::Layout;
        match d.kind.clone() {
            Kind::Plane { start } => {
                if let Some(p) = pointer.filter(|&p| p != d.origin) {
                    d.cur = Some(p);
                }
                if down {
                    return;
                }
                d.cancelled = true;
                let Some(cur) = d.cur else { return };
                let planes = self.session.planes();
                let end = snap(view, planes, view.to_doc(cur), style.hit, &[]);
                let (x0, y0, x1, y1) = (start.x.min(end.x), start.y.min(end.y), start.x.max(end.x), start.y.max(end.y));
                if x1 - x0 >= PLANE_MINIMUM && y1 - y0 >= PLANE_MINIMUM {
                    let corners = [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)];
                    let mut grown = planes.to_vec();
                    grown.push(Plane::identity(corners.map(|c| snap(view, planes, c, style.hit, &[]))));
                    *picked = Some(PlaneCorner { plane: grown.len() - 1, corner: 0 });
                    gesture.set(grown);
                }
            }
            Kind::Corner { group, offset } => {
                if !d.began {
                    if pointer.is_none_or(|p| p == d.origin) {
                        return;
                    }
                    gesture.begin_with_value();
                    if !gesture.is_active() {
                        d.cancelled = true;
                        return;
                    }
                    d.began = true;
                }
                if let Some(pointer) = pointer {
                    let mut at = view.to_doc(pointer) + offset;
                    if layout {
                        at = snap(view, &d.base, at, style.hit, &group);
                    }
                    let mut moved = d.base.clone();
                    place(&mut moved, &group, at, layout);
                    gesture.update(moved, mods);
                }
                if !down {
                    gesture.commit();
                }
            }
        }
    }

    /// The keys: Enter and Esc always; with the group focused and no press held, the arrows move
    /// the picked corner and the corners linked to it, and in Layout Insert adds a plane and
    /// Delete and Backspace remove the picked corner's plane.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn keys(
        &self,
        ui: &Ui,
        idle: bool,
        focused: bool,
        gesture: &mut Gesture<Vec<Plane>>,
        shown: &[Plane],
        picked: &mut Option<PlaneCorner>,
        out: &mut PerspectiveWarpResponse,
    ) {
        let ctx = ui.ctx();
        if !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        out.apply |= enter;
        out.discard |= escape && !enter;
        if !idle || !focused {
            return;
        }
        let layout = self.session.mode() == PlaneMode::Layout;
        let on = picked.filter(|c| c.plane < shown.len());
        if let (Some(by), Some(corner)) = (nudge::read(ui, nudge::STEP, nudge::BIG_STEP), on) {
            let group = linked_corners(shown).into_iter().find(|g| g.contains(&corner)).unwrap_or_else(|| vec![corner]);
            let mut moved = shown.to_vec();
            for c in &group {
                let at = corners(&moved[c.plane], !layout)[c.corner] + by;
                place(&mut moved, &[*c], at, layout);
            }
            gesture.set(moved);
        }
        if layout {
            if take_exact(ui, Modifiers::NONE, Key::Insert) > 0 {
                let mut grown = shown.to_vec();
                grown.push(self.session.inserted_plane());
                *picked = Some(PlaneCorner { plane: grown.len() - 1, corner: 0 });
                gesture.set(grown);
            }
            if take_exact(ui, Modifiers::NONE, Key::Delete) + take_exact(ui, Modifiers::NONE, Key::Backspace) > 0
                && let Some(corner) = on
            {
                let mut rest = shown.to_vec();
                rest.remove(corner.plane);
                *picked = (!rest.is_empty()).then(|| PlaneCorner { plane: corner.plane.min(rest.len() - 1), corner: 0 });
                gesture.set(rest);
            }
        }
    }

    /// The corners as keyboard parts and AccessKit objects, plane by plane.
    fn parts(&self, ui: &Ui, view: &CanvasView, labels: &PerspectiveWarpLabels, planes: &[Plane]) -> Vec<Part> {
        let pixels;
        let (ux, uy) = match self.units {
            Some(units) => units,
            None => {
                pixels = RulerUnits::pixels((*LocaleNumbers::of(ui)).clone());
                (&pixels, &pixels)
            }
        };
        let hit = PerspectiveWarp::style(&Tokens::of(ui)).hit;
        let warp = self.session.mode() == PlaneMode::Warp;
        let groups = linked_corners(planes);
        let mut parts = Vec::with_capacity(planes.len() * 4);
        for (p, plane) in planes.iter().enumerate() {
            for (c, &at) in corners(plane, warp).iter().enumerate() {
                let name = fill(&labels.corner, &[("plane", &(p + 1).to_string()), ("corner", &(c + 1).to_string())]);
                let mut value = fill(&labels.position, &[("x", &ux.format_value(at.x, 0.0)), ("y", &uy.format_value(at.y, 0.0))]);
                let here = PlaneCorner { plane: p, corner: c };
                if groups.iter().any(|g| g.len() > 1 && g.contains(&here)) {
                    value = format!("{value}, {}", labels.linked);
                }
                parts.push(Part {
                    key: key_of(here) as u64,
                    name,
                    value: Some(value),
                    rect: Rect::from_center_size(view.to_screen(at), vec2(2.0 * hit, 2.0 * hit)),
                    selected: false,
                });
            }
        }
        parts
    }
}

/// The keys a focused overlay keeps from the app's shortcuts.
const CLAIMED: [ClaimedKey; 3] = [
    ClaimedKey::exact(Modifiers::NONE, Key::Delete),
    ClaimedKey::exact(Modifiers::NONE, Key::Backspace),
    ClaimedKey::exact(Modifiers::NONE, Key::Insert),
];

/// What a press does.
#[derive(Clone, Debug)]
enum Kind {
    /// Moves these linked corners, keeping the pointer's offset from the corner.
    Corner { group: Vec<PlaneCorner>, offset: Vec2 },
    /// Draws a plane from this point (snapped) to the pointer.
    Plane { start: Point },
}

/// A press the overlay owns, kept between passes.
#[derive(Clone, Debug)]
struct Drag {
    press: PrimaryPress,
    kind: Kind,
    /// The press, in screen points.
    origin: Pos2,
    /// The pointer of a new plane's rubber band, once it has moved.
    cur: Option<Pos2>,
    began: bool,
    /// Esc, a replacement of the planes or a missed pass ended the press; the rest of it does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]).
    waits: bool,
    /// The session's generation at the press.
    generation: u64,
    /// The planes the movement is added to: the session's at the press.
    base: Vec<Plane>,
}

/// A new press and the corner it picks.
struct Pressed {
    drag: Drag,
    pick: Option<PlaneCorner>,
}

fn drag_key(id: Id) -> Id {
    id.with("craft_imaging::warp::perspective::drag")
}

fn picked_key(id: Id) -> Id {
    id.with("craft_imaging::warp::perspective::picked")
}

/// The part key of a corner: planes in order, four corners each.
fn key_of(corner: PlaneCorner) -> usize {
    corner.plane * 4 + corner.corner
}

/// The corners a mode shows and drags: the sources in Layout, the targets in Warp.
fn corners(plane: &Plane, warp: bool) -> &[Point; 4] {
    if warp { &plane.target } else { &plane.source }
}

/// Moves `group` to `at`: the target, and in Layout the source too.
fn place(planes: &mut [Plane], group: &[PlaneCorner], at: Point, layout: bool) {
    for c in group {
        if let Some(plane) = planes.get_mut(c.plane) {
            plane.target[c.corner] = at;
            if layout {
                plane.source[c.corner] = at;
            }
        }
    }
}

/// The corner nearest screen point `p` and strictly within `reach` of it.
fn corner_at(view: &CanvasView, planes: &[Plane], warp: bool, p: Pos2, reach: f32) -> Option<PlaneCorner> {
    planes
        .iter()
        .enumerate()
        .flat_map(|(plane, pl)| corners(pl, warp).iter().enumerate().map(move |(corner, &at)| (PlaneCorner { plane, corner }, at)))
        .map(|(c, at)| (c, view.to_screen(at).distance(p)))
        .filter(|&(_, d)| d < reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c)
}

/// `at`, or the source corner of another plane nearest it and strictly within `reach` screen
/// points; the corners in `skip` are not offered.
fn snap(view: &CanvasView, planes: &[Plane], at: Point, reach: f32, skip: &[PlaneCorner]) -> Point {
    let from = view.to_screen(at);
    planes
        .iter()
        .enumerate()
        .flat_map(|(plane, pl)| pl.source.iter().enumerate().map(move |(corner, &p)| (PlaneCorner { plane, corner }, p)))
        .filter(|(c, _)| !skip.contains(c))
        .map(|(_, p)| (p, view.to_screen(p).distance(from)))
        .filter(|&(_, d)| d < reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(at, |(p, _)| p)
}

/// Each plane's thirds and outline, then its corners; the rubber band of a new plane; the picked
/// corner's focus ring.
fn paint_planes(
    painter: &Painter,
    style: &PerspectiveWarpStyle,
    view: &CanvasView,
    planes: &[Plane],
    warp: bool,
    band: Option<Rect>,
    ring: Option<PlaneCorner>,
) {
    let ppp = painter.ctx().pixels_per_point();
    for plane in planes {
        for k in 1..3 {
            let t = f64::from(k) / 3.0;
            for (a, b) in [((t, 0.0), (t, 1.0)), ((0.0, t), (1.0, t))] {
                painter.line_segment([view.to_screen(plane.point(a.0, a.1, warp)), view.to_screen(plane.point(b.0, b.1, warp))], style.thirds);
            }
        }
        let points: Vec<Pos2> = corners(plane, warp).iter().map(|&c| view.to_screen(c)).collect();
        painter.add(Shape::closed_line(points.clone(), style.line));
        for at in points {
            painter.extend(Handle::shapes_with(&style.corner, craft_ui::primitives::HandleShape::Round, at, ppp));
        }
    }
    if let Some(rect) = band {
        painter.rect_stroke(rect, 0.0, style.band, StrokeKind::Inside);
    }
    if let Some(c) = ring
        && let Some(plane) = planes.get(c.plane)
    {
        let side = style.corner.size;
        PartFocus::ring(painter, Rect::from_center_size(view.to_screen(corners(plane, warp)[c.corner]), vec2(side, side)));
    }
}
