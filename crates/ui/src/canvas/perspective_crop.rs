//! [`PerspectiveCropQuad`]: the Perspective Crop tool's quad, its perspective grid, corner
//! handles and gestures.

use egui::{Color32, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Pos2, Response, Shape, Stroke, StrokeKind, Ui};
use kurbo::{Point, Vec2};

use super::claim::PrimaryPress;
use super::quad::Homography;
use super::story::ViewArgs;
use super::transform_box::fixed;
use super::{CanvasView, Claim, Part, PartFocus, RulerUnits};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::popup_list;
use crate::primitives::{Handle, HandleShape};
use crate::story::{Level, Story, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// The quad of a perspective crop in document units, corners ordered top left, top right, bottom
/// right, bottom left; `None` before one is drawn.
pub type PerspectiveQuad = Option<[Point; 4]>;

/// A grid line's alpha, of 255.
const GRID_ALPHA: u8 = 150;
/// Fewest and most grid steps along a side.
const GRID_STEPS: (usize, usize) = (2, 12);

/// The AccessKit names of the quad and its corners, the host's translated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerspectiveCropNames {
    /// The group, e.g. "Perspective crop".
    pub group: String,
    /// The corners clockwise from the top left: "Top left corner" …
    pub corners: [String; 4],
}

/// A quad's look and reach, from [`PerspectiveCropQuad::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PerspectiveCropStyle {
    /// The outline of a valid quad: 1.5 pt of `overlay.line`.
    pub outline: Stroke,
    /// The outline of a quad that cannot be committed: 1.5 pt of `overlay.crop_invalid`.
    pub invalid: Stroke,
    /// Side of a corner square in points.
    pub corner: f32,
    /// The corner square's fill: `overlay.handle_fill`.
    pub corner_fill: Color32,
    /// The corner square's edge width; its colour is the outline's.
    pub corner_edge: f32,
    /// Under the corner square, outside it: `handle.halo` in high contrast, else no width.
    pub halo: Stroke,
    /// A perspective grid line: `overlay.crop` at a part of its alpha.
    pub grid: Stroke,
    /// Reach of a corner in screen points (the default of [`PerspectiveCropQuad::tolerance`]):
    /// `crop.perspective_reach`.
    pub reach: f32,
    /// Screen points of the longest side per grid step: `crop.perspective_grid_step`.
    pub grid_step: f32,
    /// Radius of a placed corner click: `crop.perspective_click_dot`.
    pub click_dot: f32,
    /// A press that moves less than this (screen points) is a click: `crop.perspective_click_move`
    /// scaled by `reach` over `crop.perspective_reach`.
    pub click_move: f32,
}

/// What a press at a screen point would take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerspectiveHit {
    /// Corner `n` (0 top left, clockwise).
    Corner(usize),
    /// Inside the quad: moves it.
    Inside,
    /// Nothing: a press does nothing while a quad exists.
    Outside,
}

/// What [`PerspectiveCropQuad::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PerspectiveCropResponse {
    /// The quad's id, for [`PerspectiveCropQuad::focus`].
    pub id: Id,
    /// A drag (Begin on the first move, Update, Commit on release; Esc cancels it), the click
    /// that places the fourth corner or an arrow key's Set.
    pub edit: Option<EditEvent<PerspectiveQuad>>,
    /// Whether Enter or the commit button may commit: the quad is convex and wound like its box.
    pub committable: bool,
    /// Enter was pressed while the quad is [committable](Self::committable).
    pub apply: bool,
    /// Esc while no drag is on: close the tool.
    pub discard: bool,
    /// The corner the keys act on, picked by the pointer or the Page keys.
    pub picked: Option<usize>,
    /// What a press at the pointer would take; `None` while the pointer is off the canvas or
    /// another overlay owns the press.
    pub hovered: Option<PerspectiveHit>,
    /// Whether the quad owns the current press: the host skips its tool input.
    pub claimed: bool,
    /// Whether a drag is running.
    pub dragging: bool,
    /// Corners clicked so far, before the fourth makes the quad.
    pub placing: usize,
}

/// What a drag does to the quad.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// Draws an upright box from `anchor`.
    Draw { anchor: Point },
    /// Moves corner `index`; `offset` is the press's distance from it.
    Corner { index: usize, offset: Vec2 },
    /// Moves the whole quad.
    Move,
}

/// A drag in progress, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    kind: Kind,
    /// The host's quad at the press.
    at_press: PerspectiveQuad,
    /// The press in document space.
    from: Point,
    press: PrimaryPress,
    began: bool,
    /// Esc, an outside change or a missed pass ended the gesture; the rest of the press does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]).
    waits: bool,
}

/// The Perspective Crop tool's quad over the host's document, as Photoshop's Perspective Crop:
/// the user marks four corners on a skewed object, and on commit the host maps the quad onto an
/// upright rectangle. The library draws the quad and its perspective grid and takes the gestures;
/// the mapping, the W × H fields and the undo step stay in the app.
///
/// ```
/// use craft_ui::canvas::{CanvasView, PerspectiveCropNames, PerspectiveCropQuad, PerspectiveQuad, kurbo::Point};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let quad: PerspectiveQuad = None;
/// let names = PerspectiveCropNames {
///     group: "Perspective crop".into(),
///     corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
/// };
/// let r = PerspectiveCropQuad::new("perspective", &quad, names).show_grid(true).show(ui, &canvas, &view);
/// if r.apply {
///     // map the quad onto an upright rectangle
/// }
/// # });
/// ```
///
/// - The host owns the quad ([`PerspectiveQuad`]): it applies the events to it, passes it back
///   each frame, crops on [`apply`](PerspectiveCropResponse::apply) or its own commit button when
///   the quad is [committable](PerspectiveCropResponse::committable), and drops the quad (passes
///   `None` and calls [`clear`](Self::clear)) when the active document changes.
/// - With no quad, a drag draws an upright box, or four clicks (a press that moves under 3 screen
///   points) place corners one by one; the fourth makes the quad, corners [`ordered`]. A drag
///   clears the clicks.
/// - With a quad, the nearest corner within [`tolerance`](Self::tolerance) moves alone, keeping
///   the press offset; inside moves the quad; a press elsewhere does nothing.
/// - A quad that is not convex or is wound differently from its box draws in
///   `overlay.crop_invalid` without the grid and is not committable.
/// - Keys, whenever no text field has focus and no popup is open: Enter applies; Esc cancels a
///   drag, ends a held press before its first move, drops the clicks, or with none discards; the
///   arrows move the picked corner (1 point, ⇧ 10), or the whole quad while the quad has no
///   keyboard focus. With the focus (one Tab stop, [`PartFocus`]) Page Down and Page Up pick the
///   next and previous corner.
/// - Nothing mirrors in a right-to-left interface: the quad is the document's.
pub struct PerspectiveCropQuad<'a> {
    id_salt: IdSalt,
    state: &'a PerspectiveQuad,
    names: PerspectiveCropNames,
    show_grid: bool,
    tolerance: Option<f32>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    value: Option<String>,
    policy: EditPolicy,
}

impl<'a> PerspectiveCropQuad<'a> {
    /// The host's `state`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a PerspectiveQuad, names: PerspectiveCropNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            names,
            show_grid: true,
            tolerance: None,
            units: None,
            value: None,
            policy: EditPolicy::default(),
        }
    }

    /// Whether a valid quad shows its perspective grid (Show Grid); true by default.
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// The reach of a corner in screen points; 8 by default.
    pub fn tolerance(mut self, points: f32) -> Self {
        self.tolerance = Some(points).filter(|t| t.is_finite() && *t > 0.0);
        self
    }

    /// The ruler units of the document's x and y axes: the corners' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The group's value for a screen reader; none by default.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> PerspectiveCropStyle {
        let o = &tokens.color.overlay;
        let handle = Handle::style(tokens, HandleShape::Square);
        let c = &tokens.component.crop;
        PerspectiveCropStyle {
            outline: Stroke::new(c.perspective_outline, o.line.to_egui()),
            invalid: Stroke::new(c.perspective_outline, o.crop_invalid.to_egui()),
            corner: c.perspective_corner,
            corner_fill: handle.fill,
            corner_edge: c.perspective_corner_edge,
            halo: handle.halo,
            grid: Stroke::new(c.perspective_grid_width, o.crop.to_egui().gamma_multiply(f32::from(GRID_ALPHA) / 255.0)),
            reach: c.perspective_reach,
            grid_step: c.perspective_grid_step,
            click_dot: c.perspective_click_dot,
            click_move: c.perspective_click_move,
        }
    }

    /// Gives the quad `id` ([`PerspectiveCropResponse::id`]) the keyboard.
    pub fn focus(ctx: &Context, id: Id) {
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Drops the corners clicked so far and the picked corner, for the quad `id`: the pending
    /// quad belongs to one document, so the host calls it when the active document changes.
    pub fn clear(ctx: &Context, id: Id) {
        ctx.data_mut(|d| {
            d.remove::<Vec<Point>>(clicks_key(id));
            d.remove::<usize>(pick_key(id));
            d.remove::<Drag>(drag_key(id));
        });
    }

    /// Draws the quad over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PerspectiveCropResponse {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let mut style = PerspectiveCropQuad::style(&tokens);
        if let Some(t) = self.tolerance {
            style.click_move *= t / style.reach;
            style.reach = t;
        }
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = PerspectiveCropResponse {
            id,
            edit: None,
            committable: false,
            apply: false,
            discard: false,
            picked: None,
            hovered: None,
            claimed: false,
            dragging: false,
            placing: 0,
        };
        if self.state.is_some_and(|q| !finite(&q)) {
            return out;
        }
        let mut picked: Option<usize> = ctx.data(|d| d.get_temp(pick_key(id)));
        let mut clicks: Vec<Point> = ctx.data(|d| d.get_temp(clicks_key(id))).unwrap_or_default();

        // A drag whose press ended while the quad was not shown is dropped, and its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.is_some_and(|d| d.waits);
        if let Some(d) = &mut drag
            && d.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                // A typing run that ended first may have changed the quad: the press works from it.
                Turn::Runs => {
                    d.waits = false;
                    d.at_press = *self.state;
                }
                Turn::Now => drag = None,
            }
        }
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            drag = self.press(ui, canvas, view, &style, id);
            if let Some(Drag { kind: Kind::Corner { index, .. }, .. }) = drag {
                picked = Some(index);
            } else if matches!(drag, Some(Drag { kind: Kind::Move, .. })) {
                picked = None;
            }
        }
        let reported = Gesture::<PerspectiveQuad>::reported(ui, id).map(|(_, value)| value);
        let host = match drag {
            Some(d) if d.began && !d.cancelled && (*self.state == d.at_press || Some(*self.state) == reported) => reported.unwrap_or(d.at_press),
            _ => *self.state,
        };
        let mut gesture = Gesture::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if let Some(d) = &mut drag
            && d.began
            && !d.cancelled
            && !gesture.is_active()
        {
            d.cancelled = true;
        }
        // Esc before the first move ends the press as it would end its drag.
        if let Some(d) = &mut drag
            && !d.began
            && !d.cancelled
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            d.cancelled = true;
        }
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        if let Some(d) = &mut drag
            && !d.waits
        {
            self.track(&ctx, view, &style, d, &mut gesture, &mut clicks);
        }

        let shown = *gesture.shown();
        let dragged_kind = drag.filter(|d| d.began && !d.cancelled).map(|d| d.kind);
        let screen = shown.map(|q| q.map(|p| view.to_screen(p)));
        let parts = self.parts(screen, &style, shown, picked);
        let mut focus = PartFocus::new(id, self.names.group.clone()).parts(&parts).picked(picked.map(|i| i as u64));
        if let Some(value) = &self.value {
            focus = focus.value(value.clone());
        }
        let focus = focus.show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.map(|k| (k as usize).min(3));
        }
        if drag.is_none_or(|d| d.cancelled) {
            let enter_ok = focus.focused || ctx.memory(|m| m.focused()).is_none_or(|f| f == canvas.id);
            self.keys(ui, drag.is_none(), enter_ok, picked.filter(|_| focus.focused), &mut gesture, &mut clicks, &mut out, shown);
        }

        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if let (Some(kind), Some(_)) = (dragged_kind, pointer) {
            ctx.set_cursor_icon(match kind {
                Kind::Draw { .. } => CursorIcon::Crosshair,
                Kind::Corner { .. } | Kind::Move => CursorIcon::Move,
            });
            out.dragging = true;
        } else if drag.is_none()
            && !other
            && let Some(p) = canvas.hover_pos()
            && ui.is_enabled()
        {
            let over = self.over(view, p, &style, shown.as_ref());
            out.hovered = Some(over);
            ctx.set_cursor_icon(match (shown.is_some(), over) {
                (false, _) => CursorIcon::Crosshair,
                (true, PerspectiveHit::Outside) => CursorIcon::Default,
                (true, _) => CursorIcon::Move,
            });
        }

        out.edit = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = picked;
        out.placing = clicks.len();
        out.committable = shown.is_some_and(|q| is_valid(&q));
        out.apply &= out.committable;
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
                Some(p) => {
                    data.insert_temp(pick_key(id), p);
                }
                None => {
                    data.remove::<usize>(pick_key(id));
                }
            }
            if clicks.is_empty() {
                data.remove::<Vec<Point>>(clicks_key(id));
            } else {
                data.insert_temp(clicks_key(id), clicks.clone());
            }
        });
        self.paint(ui, view, &style, shown, &clicks, dragged_kind, picked, focus.keyboard);
        out
    }

    /// A press on the canvas: what it does to the quad, claimed for it.
    fn press(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &PerspectiveCropStyle, id: Id) -> Option<Drag> {
        let ctx = ui.ctx();
        if !ui.is_enabled() || !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let from = view.to_doc(press.origin);
        if !(from.x.is_finite() && from.y.is_finite()) {
            return None;
        }
        let kind = match self.state {
            None => Kind::Draw { anchor: from },
            Some(q) => match hit(view, q, press.origin, style.reach) {
                PerspectiveHit::Corner(index) => Kind::Corner { index, offset: from - q[index] },
                PerspectiveHit::Inside => Kind::Move,
                PerspectiveHit::Outside => return None,
            },
        };
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { kind, at_press: *self.state, from, press, began: false, cancelled: false, waits })
    }

    /// What screen point `p` is over.
    fn over(&self, view: &CanvasView, p: Pos2, style: &PerspectiveCropStyle, quad: Option<&[Point; 4]>) -> PerspectiveHit {
        quad.map_or(PerspectiveHit::Outside, |q| hit(view, q, p, style.reach))
    }

    /// One pass of a drag: begins it on the first move (a box draw, past the click distance),
    /// updates it, ends it on the release. A draw released within the click distance places a
    /// corner.
    fn track(
        &self,
        ctx: &Context,
        view: &CanvasView,
        style: &PerspectiveCropStyle,
        d: &mut Drag,
        gesture: &mut Gesture<PerspectiveQuad>,
        clicks: &mut Vec<Point>,
    ) {
        if d.cancelled {
            return;
        }
        let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
        let click = style.click_move;
        if let Some(pointer) = pointer {
            let far = match d.kind {
                Kind::Draw { .. } => (pointer - d.press.origin).abs().max_elem() > click,
                _ => pointer != d.press.origin,
            };
            if !d.began && far {
                gesture.begin();
                d.began = gesture.is_active();
                if d.began && matches!(d.kind, Kind::Draw { .. }) {
                    clicks.clear();
                }
            }
            if d.began {
                let to = view.to_doc(pointer);
                if to.x.is_finite() && to.y.is_finite() {
                    gesture.update(self.dragged(d, to), Modifiers::NONE);
                }
            }
        }
        if down {
            return;
        }
        if d.began {
            gesture.commit();
        } else if let Kind::Draw { anchor } = d.kind {
            clicks.push(anchor);
            if let [a, b, c, e] = clicks[..] {
                gesture.set(Some(ordered([a, b, c, e])));
                clicks.clear();
            }
        }
    }

    /// The quad a drag gives with the pointer at document point `to`; a box with no area is no
    /// quad.
    fn dragged(&self, d: &Drag, to: Point) -> PerspectiveQuad {
        match (d.kind, d.at_press) {
            (Kind::Draw { anchor }, _) => {
                let (x0, y0, x1, y1) = (anchor.x.min(to.x), anchor.y.min(to.y), anchor.x.max(to.x), anchor.y.max(to.y));
                (x1 > x0 && y1 > y0).then_some([Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)])
            }
            (Kind::Corner { index, offset }, Some(mut q)) => {
                q[index] = to - offset;
                Some(q)
            }
            (Kind::Move, Some(q)) => {
                let by = to - d.from;
                Some(q.map(|p| p + by))
            }
            _ => d.at_press,
        }
    }

    /// Enter, Esc and the arrows; `idle`: no press is held; `enter_ok`: no other widget (a
    /// Tab-focused button) holds the keyboard, so Enter is the quad's.
    #[allow(clippy::too_many_arguments)]
    fn keys(
        &self,
        ui: &Ui,
        idle: bool,
        enter_ok: bool,
        picked: Option<usize>,
        gesture: &mut Gesture<PerspectiveQuad>,
        clicks: &mut Vec<Point>,
        out: &mut PerspectiveCropResponse,
        shown: PerspectiveQuad,
    ) {
        let ctx = ui.ctx();
        if !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (enter_ok && i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        out.apply |= enter;
        out.discard |= escape && !enter && clicks.is_empty();
        if escape {
            clicks.clear();
        }
        if !idle {
            return;
        }
        if let (Some(q), Some(by)) = (shown, nudge::read(ui, nudge::STEP, nudge::BIG_STEP)) {
            let next = match picked {
                Some(i) => {
                    let mut q = q;
                    q[i] += by;
                    q
                }
                None => q.map(|p| p + by),
            };
            if finite(&next) {
                gesture.set(Some(next));
            }
        }
    }

    /// The keyboard parts of the quad `shown` (on screen as `screen`): its four corners.
    fn parts(&self, screen: Option<[Pos2; 4]>, style: &PerspectiveCropStyle, shown: PerspectiveQuad, picked: Option<usize>) -> Vec<Part> {
        let (Some(screen), Some(doc)) = (screen, shown) else { return Vec::new() };
        let reach = egui::Vec2::splat(2.0 * style.reach);
        (0..4)
            .map(|i| Part {
                key: i as u64,
                name: self.names.corners[i].clone(),
                value: self.units.map(|(x, y)| format!("{}, {}", x.format_value(fixed(doc[i].x, 1), 0.0), y.format_value(fixed(doc[i].y, 1), 0.0))),
                rect: egui::Rect::from_center_size(screen[i], reach),
                selected: picked == Some(i),
            })
            .collect()
    }

    /// Draws the grid, the outline, the corner squares, and the corners clicked so far.
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &self,
        ui: &Ui,
        view: &CanvasView,
        style: &PerspectiveCropStyle,
        shown: PerspectiveQuad,
        clicks: &[Point],
        dragging: Option<Kind>,
        picked: Option<usize>,
        keyboard: bool,
    ) {
        let painter = ui.painter_at(view.rect);
        let tokens = Tokens::of(ui);
        let accent = tokens.color.overlay.line.to_egui();
        let at = |p: Point| view.to_screen(p);
        if let Some(q) = shown {
            let valid = is_valid(&q);
            let stroke = if valid { style.outline } else { style.invalid };
            let pts: Vec<Pos2> = q.iter().map(|&c| at(c)).collect();
            if valid && self.show_grid {
                for [a, b] in grid_lines(view, &q, style.grid_step) {
                    painter.line_segment([a, b], style.grid);
                }
            }
            painter.add(Shape::closed_line(pts.clone(), stroke));
            if !matches!(dragging, Some(Kind::Draw { .. })) {
                for (i, c) in pts.iter().enumerate() {
                    let r = egui::Rect::from_center_size(*c, egui::vec2(style.corner, style.corner));
                    if style.halo.width > 0.0 {
                        painter.rect_filled(r.expand(style.halo.width), 0.0, style.halo.color);
                    }
                    painter.rect_filled(r, 0.0, style.corner_fill);
                    painter.rect_stroke(r, 0.0, Stroke::new(style.corner_edge, stroke.color), StrokeKind::Inside);
                    if keyboard && picked == Some(i) {
                        PartFocus::ring(&painter, r);
                    }
                }
            }
        }
        let dots: Vec<Pos2> = clicks.iter().map(|&c| at(c)).collect();
        if dots.len() > 1 {
            painter.add(Shape::line(dots.clone(), Stroke::new(style.outline.width, accent)));
        }
        for c in dots {
            painter.circle_filled(c, style.click_dot, style.corner_fill);
            painter.circle_stroke(c, style.click_dot, Stroke::new(style.corner_edge, accent));
        }
    }
}

/// Four points in quad order: clockwise on screen about their centre, starting at the smallest
/// x + y (the top left of an upright box).
pub fn ordered(points: [Point; 4]) -> [Point; 4] {
    let c = Point::new(points.iter().map(|p| p.x).sum::<f64>() / 4.0, points.iter().map(|p| p.y).sum::<f64>() / 4.0);
    let mut v = points;
    // y grows downwards, so an increasing angle runs clockwise on screen.
    v.sort_by(|a, b| (a.y - c.y).atan2(a.x - c.x).total_cmp(&(b.y - c.y).atan2(b.x - c.x)));
    let first = (0..4).min_by(|&i, &j| (v[i].x + v[i].y).total_cmp(&(v[j].x + v[j].y))).unwrap_or(0);
    v.rotate_left(first);
    v
}

/// Whether `q` (top left, top right, bottom right, bottom left) is a usable perspective crop
/// quad: finite, strictly convex with every corner angle above about 0.06°, wound clockwise on
/// screen (y down) and at least one unit² in area.
pub fn is_valid(q: &[Point; 4]) -> bool {
    if !finite(q) {
        return false;
    }
    let mut area2 = 0.0;
    for i in 0..4 {
        let (a, b, c) = (q[i], q[(i + 1) % 4], q[(i + 2) % 4]);
        let (e1, e2) = (b - a, c - b);
        // sin(turn) = cross / (|e1| |e2|): positive (clockwise on screen) and not a straight angle.
        let cross = e1.cross(e2);
        if cross.is_nan() || cross <= 1e-3 * e1.hypot() * e2.hypot() {
            return false;
        }
        area2 += a.x * b.y - b.x * a.y;
    }
    area2.is_finite() && area2 >= 2.0
}

fn finite(q: &[Point; 4]) -> bool {
    q.iter().all(|p| p.x.is_finite() && p.y.is_finite())
}

/// Grid steps along a side that is about `side` screen points: one per `step`, from 2 to 12.
pub fn grid_steps(side: f32, step: f32) -> usize {
    ((side / step).round() as usize).clamp(GRID_STEPS.0, GRID_STEPS.1)
}

/// The grid of the valid quad `q` on screen: its inner lines at equal steps of the unit square
/// mapped through the quad's homography, so they converge like the object's own lines. The step
/// count follows the longest of the first three sides, as PhotoCraft's.
pub fn grid_lines(view: &CanvasView, q: &[Point; 4], step: f32) -> Vec<[Pos2; 2]> {
    let Some(h) = Homography::unit_to_quad(*q) else { return Vec::new() };
    let pts = q.map(|c| view.to_screen(c));
    let side = pts.iter().zip(pts.iter().skip(1)).map(|(a, b)| a.distance(*b)).fold(0.0, f32::max);
    let n = grid_steps(side, step);
    let at = |u: f64, v: f64| view.to_screen(h.apply(Point::new(u, v)));
    let mut out = Vec::with_capacity(2 * (n - 1));
    for k in 1..n {
        let t = k as f64 / n as f64;
        out.push([at(t, 0.0), at(t, 1.0)]);
        out.push([at(0.0, t), at(1.0, t)]);
    }
    out
}

/// What a press at screen point `p` takes: the nearest corner within `reach` screen points, else
/// the inside of the quad, else nothing.
fn hit(view: &CanvasView, q: &[Point; 4], p: Pos2, reach: f32) -> PerspectiveHit {
    let near = (0..4).map(|i| (i, view.to_screen(q[i]).distance(p))).filter(|(_, d)| *d <= reach).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i);
    if let Some(i) = near {
        return PerspectiveHit::Corner(i);
    }
    let d = view.to_doc(p);
    // Inside a quad wound clockwise on screen: every side has the point on its right.
    if (0..4).all(|i| (q[(i + 1) % 4] - q[i]).cross(d - q[i]) >= 0.0) { PerspectiveHit::Inside } else { PerspectiveHit::Outside }
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::PerspectiveCropQuad::drag")
}

fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::PerspectiveCropQuad::picked")
}

fn clicks_key(id: Id) -> Id {
    id.with("craft_ui::canvas::PerspectiveCropQuad::clicks")
}

/// The English names the story and tests use.
pub(crate) fn english_names() -> PerspectiveCropNames {
    PerspectiveCropNames {
        group: "Perspective crop".into(),
        corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Perspective crop quad"];

#[derive(crate::story::StoryArgs, Debug, PartialEq)]
struct PerspectiveArgs {
    #[arg(flatten)]
    view: ViewArgs,
    grid: bool,
}

impl Default for PerspectiveArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), grid: true }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/canvas/perspective-crop",
            Level::Primitives,
            "Perspective crop quad",
            "Perspective crop",
            PerspectiveArgs::default(),
            show_story
        )
        .on(Surface::Canvas),
    ]
}

/// The skewed page the story's quad is marked on.
const PAGE: [Point; 4] = [Point::new(250.0, 110.0), Point::new(430.0, 130.0), Point::new(400.0, 290.0), Point::new(220.0, 250.0)];

/// A skewed page: drag a box or click its four corners, drag a corner onto each page corner,
/// Enter commits (the quad goes), Esc cancels.
fn show_story(ui: &mut Ui, args: &mut PerspectiveArgs) {
    let key = ui.scope_id().with("perspective-crop-story");
    let mut quad: PerspectiveQuad = ui.data(|d| d.get_temp(key)).unwrap_or(None);
    let before_key = key.with("before");
    let mut before: PerspectiveQuad = ui.data(|d| d.get_temp(before_key)).unwrap_or(None);
    let canvas = super::story::document(ui, args.view, "Document");
    let view = &canvas.view;
    ui.painter_at(view.rect).add(Shape::convex_polygon(
        PAGE.iter().map(|&p| view.to_screen(p)).collect(),
        Color32::from_rgb(70, 130, 180),
        Stroke::new(1.0, Color32::from_gray(40)),
    ));
    let px = RulerUnits::pixels(crate::localized::LocaleNumbers::from_tag("en").ungrouped());
    let r = PerspectiveCropQuad::new("story-perspective-crop", &quad, english_names()).show_grid(args.grid).units(&px, &px).show(
        ui,
        &canvas.response,
        view,
    );
    match &r.edit {
        Some(EditEvent::Begin { .. }) => before = quad,
        Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => quad = *value,
        Some(EditEvent::Cancel { .. }) => quad = before,
        None => {}
    }
    ui.data_mut(|d| d.insert_temp(before_key, before));
    if r.apply || r.discard {
        quad = None;
        PerspectiveCropQuad::clear(ui.ctx(), r.id);
    }
    ui.data_mut(|d| d.insert_temp(key, quad));
}

#[cfg(test)]
#[path = "perspective_crop_tests.rs"]
mod tests;
