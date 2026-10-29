//! [`WarpGrid`]: the mesh of a warp (PhotoCraft's Edit › Transform › Warp) on a canvas: patch
//! boundaries, anchors and their handles, dragged by the pointer and moved from the keyboard.

pub(crate) mod story;

use egui::{AsIdSalt, Context, CursorIcon, Id, IdSalt, Painter, Pos2, Response, Shape, Stroke, Ui, vec2};
use kurbo::{BezPath, PathEl, Point, Rect, Vec2};

use super::claim::PrimaryPress;
use super::{CanvasView, Claim, Part, PartFocus, RulerUnits, hit};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A warp's control mesh: `columns` × `rows` bicubic Bézier patches sharing their edges, as
/// `(3 columns + 1) × (3 rows + 1)` control points row by row, in document units.
///
/// A point on a row and a column that are both multiples of 3 is an anchor, where patches meet.
/// The other points on those rows and columns are the anchors' handles; the points inside a patch
/// shape its inside and are neither drawn nor dragged.
#[derive(Clone, Debug, PartialEq)]
pub struct WarpMesh {
    pub columns: usize,
    pub rows: usize,
    pub points: Vec<Point>,
}

/// The side of its anchor a handle leaves on, in the mesh's own order: up and down along a
/// column, left and right along a row (in the document, unmirrored).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarpSide {
    Up,
    Left,
    Right,
    Down,
}

impl WarpMesh {
    /// An unwarped mesh over `rect`: `columns` × `rows` even patches.
    pub fn identity(rect: Rect, columns: usize, rows: usize) -> Self {
        let (columns, rows) = (columns.max(1), rows.max(1));
        let (w, h) = (3 * columns + 1, 3 * rows + 1);
        let points = (0..h)
            .flat_map(|j| {
                (0..w).map(move |i| {
                    let (u, v) = (i as f64 / (w - 1) as f64, j as f64 / (h - 1) as f64);
                    Point::new(rect.x0 + u * rect.width(), rect.y0 + v * rect.height())
                })
            })
            .collect();
        Self { columns, rows, points }
    }

    /// Control points per row: `3 columns + 1`.
    pub fn width(&self) -> usize {
        3 * self.columns + 1
    }

    /// Control points per column: `3 rows + 1`.
    pub fn height(&self) -> usize {
        3 * self.rows + 1
    }

    /// Whether the mesh has at least one patch and as many points as its patches need.
    pub fn is_valid(&self) -> bool {
        self.columns > 0 && self.rows > 0 && self.points.len() == self.width() * self.height()
    }

    /// The control point at column `i`, row `j`.
    pub fn point(&self, i: usize, j: usize) -> Point {
        self.points[j * self.width() + i]
    }

    /// The anchor the point `index` belongs to: its own index for an anchor, its anchor's for a
    /// handle, `None` for a point inside a patch.
    pub fn anchor_of(&self, index: usize) -> Option<usize> {
        let w = self.width();
        let (i, j) = (index % w, index / w);
        let near = |k: usize| match k % 3 {
            0 => k,
            1 => k - 1,
            _ => k + 1,
        };
        match (i % 3 == 0, j % 3 == 0) {
            (true, true) => Some(index),
            (true, false) => Some(near(j) * w + i),
            (false, true) => Some(j * w + near(i)),
            (false, false) => None,
        }
    }

    /// The handles of the anchor `index`, each with its side, in the mesh's order.
    pub fn handles(&self, index: usize) -> Vec<(usize, WarpSide)> {
        let w = self.width();
        let (i, j) = (index % w, index / w);
        let mut out = Vec::with_capacity(4);
        if j > 0 {
            out.push((index - w, WarpSide::Up));
        }
        if i > 0 {
            out.push((index - 1, WarpSide::Left));
        }
        if i + 1 < w {
            out.push((index + 1, WarpSide::Right));
        }
        if j + 1 < self.height() {
            out.push((index + w, WarpSide::Down));
        }
        out
    }

    /// The anchors row by row.
    fn anchors(&self) -> impl Iterator<Item = usize> + '_ {
        let w = self.width();
        (0..self.height()).step_by(3).flat_map(move |j| (0..w).step_by(3).map(move |i| j * w + i))
    }

    /// The patch boundaries: each a chain of cubic segments along a row or a column of anchors.
    fn boundaries(&self) -> Vec<BezPath> {
        let (w, h) = (self.width(), self.height());
        let chain = |points: Vec<Point>| {
            let mut path = BezPath::new();
            path.move_to(points[0]);
            for &[a, b, c] in points[1..].as_chunks::<3>().0 {
                path.curve_to(a, b, c);
            }
            path
        };
        let rows = (0..h).step_by(3).map(|j| chain((0..w).map(|i| self.point(i, j)).collect()));
        let columns = (0..w).step_by(3).map(|i| chain((0..h).map(|j| self.point(i, j)).collect()));
        rows.chain(columns).collect()
    }

    /// A one-patch mesh's rule-of-thirds curves: the patch at a third and two thirds across, each
    /// way.
    fn thirds(&self) -> Vec<BezPath> {
        let curve = |q: [Point; 4]| {
            let mut path = BezPath::new();
            path.move_to(q[0]);
            path.curve_to(q[1], q[2], q[3]);
            path
        };
        let mut out = Vec::with_capacity(4);
        for t in [1.0 / 3.0, 2.0 / 3.0] {
            out.push(curve(std::array::from_fn(|j| cubic(std::array::from_fn(|i| self.point(i, j)), t))));
            out.push(curve(std::array::from_fn(|i| cubic(std::array::from_fn(|j| self.point(i, j)), t))));
        }
        out
    }
}

/// The cubic Bézier through control points `p` at `t`.
fn cubic(p: [Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    let b = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
    Point::new(p.iter().zip(b).map(|(p, b)| p.x * b).sum(), p.iter().zip(b).map(|(p, b)| p.y * b).sum())
}

/// A warp drag or nudge, in document units. A drag's `by` is the whole move from its press, which
/// the host applies to the mesh it had at the press; a nudge's (a [`Set`](EditEvent::Set)) is
/// one step from the mesh it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarpEdit {
    /// Move the control point `index` (an anchor carries its handles along: the host's rule).
    Point { index: usize, by: Vec2 },
    /// Pull the surface at the host's parameters `(s, t)` ([`WarpGrid::surface`]).
    Surface { s: f64, t: f64, by: Vec2 },
}

/// The AccessKit names of a warp mesh's parts, the host's translated text.
pub struct WarpNames<'a> {
    /// The group of parts ("Warp").
    pub group: String,
    /// The name of the anchor at a row and column (from 1): "Row 2, column 3".
    pub anchor: Box<dyn Fn(usize, usize) -> String + 'a>,
    /// The name of a handle of the anchor at a row and column (from 1): "Row 2, column 3, right
    /// handle".
    pub handle: Box<dyn Fn(usize, usize, WarpSide) -> String + 'a>,
}

impl<'a> WarpNames<'a> {
    pub fn new(
        group: impl Into<String>,
        anchor: impl Fn(usize, usize) -> String + 'a,
        handle: impl Fn(usize, usize, WarpSide) -> String + 'a,
    ) -> Self {
        Self { group: group.into(), anchor: Box::new(anchor), handle: Box::new(handle) }
    }
}

/// A warp mesh's look and reach, from [`WarpGrid::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WarpStyle {
    /// Patch boundaries: `transform.line` of `overlay.line`, bare.
    pub line: Stroke,
    /// A one-patch mesh's rule-of-thirds lines: `transform.warp_minor` of `overlay.line` at 0.7
    /// alpha.
    pub minor: Stroke,
    /// Anchors: [`Handle::style`] `Square` at `transform.warp_anchor`.
    pub anchor: HandleStyle,
    /// Handle ends: [`Handle::style`] `Round` at `transform.warp_control`.
    pub control: HandleStyle,
    /// The arm from an anchor to a shown handle: `transform.warp_minor` of `overlay.handle_stroke`.
    pub arm: Stroke,
    /// Distance from an anchor or a handle within which a press takes it: `transform.hit`.
    pub hit: f32,
    /// Distance from the pointer within which a handle and its arm show, measured to the handle
    /// or its anchor: `transform.warp_reveal`.
    pub reveal: f32,
    /// Largest distance between a curve and the lines it is drawn with: `bezier.flatten`.
    pub flatten: f32,
}

/// What [`WarpGrid::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the warp edits to the host's mesh"]
pub struct WarpResponse {
    /// The grid's id, also its Tab stop's.
    pub id: Id,
    /// A drag (Begin on the first move, Update, Commit on release; Esc cancels it) or an arrow
    /// key's Set.
    pub event: Option<EditEvent<WarpEdit>>,
    /// The control point the keys act on, picked by the pointer or the Page keys.
    pub picked: Option<usize>,
    /// Whether the grid owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The host's surface parameters under a document point ([`WarpGrid::surface`]).
type Surface<'a> = Box<dyn Fn(Point) -> Option<(f64, f64)> + 'a>;

/// What a press drags.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Point(usize),
    Surface(f64, f64),
}

/// A press the grid owns, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    press: PrimaryPress,
    target: Target,
    began: bool,
    /// Esc ended it: the rest of the press does nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); its target is
    /// the one at the press and is made again when its turn comes.
    waits: bool,
}

/// The mesh of a warp over a canvas: patch boundaries, anchors and their handles, dragged by the
/// pointer and moved from the keyboard as PhotoCraft's Warp has them. The grid draws, senses and
/// reports [`WarpEdit`]s; the host moves its mesh (an anchor carrying its handles, a pull of the
/// surface) and passes it back each frame.
///
/// ```
/// use craft_ui::canvas::{CanvasView, WarpGrid, WarpMesh, WarpNames, kurbo::{Point, Rect}};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let mesh = WarpMesh::identity(Rect::new(100.0, 80.0, 300.0, 220.0), 1, 1);
/// let names = WarpNames::new("Warp", |row, column| format!("Row {row}, column {column}"), |row, column, side| {
///     format!("Row {row}, column {column}, {side:?} handle")
/// });
/// let r = WarpGrid::new("warp", &mesh, names).show(ui, &canvas, &view);
/// if !r.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - Look: patch boundaries a bare `transform.line`; a one-patch mesh also its rule-of-thirds
///   lines. Anchors always show; a handle and its arm only within `transform.warp_reveal` of the
///   pointer (from the handle or its anchor), while its anchor or one of its handles is dragged,
///   or while one of its anchor's parts is picked and the grid has the keyboard.
/// - Pointer (screen points): the nearest anchor or handle within `transform.hit` drags that
///   point ([`WarpEdit::Point`]); elsewhere on the surface ([`surface`](Self::surface)) the press
///   pulls it ([`WarpEdit::Surface`]). A drag begins on the first move; Esc cancels it and the
///   rest of the press does nothing. The host shows the grid before its guides and tool input and
///   skips the tool while [`claimed`](WarpResponse::claimed).
/// - Keyboard: one Tab stop ([`PartFocus`]). Page Down and Page Up pick the next and previous
///   part (anchors row by row, each followed by its handles), Home and End the first and last; the
///   arrows move the picked point (⇧ ten steps).
/// - Nothing mirrors in a right-to-left interface: the mesh is the document's.
pub struct WarpGrid<'a> {
    id_salt: IdSalt,
    mesh: &'a WarpMesh,
    names: WarpNames<'a>,
    editable: bool,
    surface: Option<Surface<'a>>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    zero: Point,
    steps: (f64, f64),
    policy: EditPolicy,
}

impl<'a> WarpGrid<'a> {
    /// The host's `mesh`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, mesh: &'a WarpMesh, names: WarpNames<'a>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            mesh,
            names,
            editable: true,
            surface: None,
            units: None,
            zero: Point::ZERO,
            steps: (nudge::STEP, nudge::BIG_STEP),
            policy: EditPolicy::default(),
        }
    }

    /// Whether the mesh's points show and can be dragged; true by default. False draws the patch
    /// boundaries only (PhotoCraft's Warp on type, which takes only the warp styles).
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// The host's surface parameters `(s, t)` under a document point, `None` off the surface: a
    /// press there pulls the surface. Without it only the points are dragged.
    pub fn surface(mut self, surface: impl Fn(Point) -> Option<(f64, f64)> + 'a) -> Self {
        self.surface = Some(Box::new(surface));
        self
    }

    /// The ruler units of the document's x and y axes, for the parts' AccessKit values. Without
    /// them the parts have no value.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The document point the rulers count from, for the AccessKit values.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// The arrow keys' move, in document units, plain and with Shift (1 and 10).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> WarpStyle {
        let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
        let stroke = o.line.to_egui();
        WarpStyle {
            line: Stroke::new(t.line, stroke),
            minor: Stroke::new(t.warp_minor, stroke.gamma_multiply(0.7)),
            anchor: HandleStyle { size: t.warp_anchor, ..Handle::style(tokens, HandleShape::Square) },
            control: HandleStyle { size: t.warp_control, ..Handle::style(tokens, HandleShape::Round) },
            arm: Stroke::new(t.warp_minor, o.handle_stroke.to_egui()),
            hit: t.hit,
            reveal: t.warp_reveal,
            flatten: tokens.component.bezier.flatten,
        }
    }

    /// Draws the mesh over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> WarpResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let style = WarpGrid::style(&Tokens::of(ui));
        let mut out = WarpResponse { id, event: None, picked: None, claimed: false };
        if !self.mesh.is_valid() {
            return out;
        }
        let editable = self.editable && ui.is_enabled();
        let screen: Vec<Pos2> = self.mesh.points.iter().map(|&p| view.to_screen(p)).collect();
        let picked_key = id.with("craft_ui::canvas::WarpGrid::picked");
        let mut picked: Option<usize> =
            ctx.data(|d| d.get_temp(picked_key)).filter(|&i: &usize| editable && i < screen.len() && self.mesh.anchor_of(i).is_some());

        // A press whose end the grid missed is dropped, its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.is_some_and(|d| d.waits);
        // The pass in which the press starts: its pass, or the one its turn comes in.
        let mut starts = false;
        if let Some(d) = drag
            && d.waits
        {
            drag = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => drag,
                Turn::Runs => {
                    starts = true;
                    let target = self.target_at(view, &screen, &style, d.press.origin);
                    Some(Drag { target: target.unwrap_or(d.target), cancelled: target.is_none(), waits: false, ..d })
                }
                Turn::Now => None,
            };
        }
        if drag.is_none() && !waited && editable {
            drag = self.press(&ctx, canvas, view, &screen, &style, id);
            starts = drag.is_some_and(|d| !d.waits);
        }
        if starts && let Some(Drag { target: Target::Point(index), cancelled: false, .. }) = drag {
            picked = Some(index);
        }
        let reported = Gesture::<WarpEdit>::reported(ui, id).map(|(_, value)| value);
        let mut gesture = Gesture::load(ui, id, &reported.unwrap_or(IDLE), self.policy).focus_owners(&[]).enabled(editable);
        if let Some(d) = &mut drag {
            if d.began && !gesture.is_active() {
                // Esc, `cancel_all` or a missed pass ended it; the press stays claimed.
                d.cancelled = true;
            }
            if !d.began && !d.cancelled && !d.waits && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                d.cancelled = true;
            }
        }
        if let Some(d) = drag.take() {
            drag = if d.waits { Some(d) } else { self.track(&ctx, view, d, &mut gesture) };
        }

        let parts = if editable { self.parts(&screen, &style, picked) } else { Vec::new() };
        let focus = PartFocus::new(id, &self.names.group).parts(&parts).picked(picked.map(|i| i as u64)).focusable(editable).show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.map(|k| k as usize);
        }
        if focus.focused
            && editable
            && drag.is_none()
            && let Some(index) = picked
            && let Some(by) = nudge::read(ui, self.steps.0, self.steps.1)
        {
            gesture.set(WarpEdit::Point { index, by });
        }
        if editable {
            self.cursor(&ctx, canvas, view, &screen, &style, drag);
        }

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = picked;
        let held = drag.filter(|d| d.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|d| {
            match held {
                Some(drag) => {
                    d.insert_temp(drag_key(id), drag);
                }
                None => d.remove::<Drag>(drag_key(id)),
            }
            match picked {
                Some(index) => {
                    d.insert_temp(picked_key, index);
                }
                None => d.remove::<usize>(picked_key),
            }
        });
        let dragged = held.and_then(|d| match d.target {
            Target::Point(index) if !d.cancelled => self.mesh.anchor_of(index),
            _ => None,
        });
        let kept = picked.filter(|_| focus.focused).and_then(|i| self.mesh.anchor_of(i));
        self.paint(ui, view, &screen, &style, editable, [dragged, kept]);
        if focus.keyboard
            && let Some(index) = picked
        {
            let size = if self.mesh.anchor_of(index) == Some(index) { style.anchor.size } else { style.control.size };
            PartFocus::ring(&ui.painter_at(view.rect), egui::Rect::from_center_size(screen[index], vec2(size, size)));
        }
        out
    }

    /// A press on the canvas: the nearest point within reach, else the surface under it, claimed
    /// for the grid.
    fn press(&self, ctx: &Context, canvas: &Response, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, id: Id) -> Option<Drag> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let target = self.target_at(view, screen, style, press.origin)?;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { press, target, began: false, cancelled: false, waits })
    }

    /// What a press at screen point `p` would drag.
    fn target_at(&self, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, p: Pos2) -> Option<Target> {
        let nearest = (0..screen.len())
            .filter(|&i| self.mesh.anchor_of(i).is_some())
            .map(|i| (i, hit::point(p, screen[i])))
            .filter(|&(_, d)| d < style.hit)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = nearest {
            return Some(Target::Point(index));
        }
        let (s, t) = self.surface.as_ref()?(view.to_doc(p))?;
        Some(Target::Surface(s, t))
    }

    /// One pass of a press the grid owns: the press that goes on, if it does.
    fn track(&self, ctx: &Context, view: &CanvasView, mut d: Drag, gesture: &mut Gesture<WarpEdit>) -> Option<Drag> {
        let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        if d.cancelled {
            return down.then_some(d);
        }
        if let Some(pointer) = pointer {
            if !d.began && pointer != d.press.origin {
                gesture.begin();
                d.began = gesture.is_active();
            }
            if d.began {
                let by = view.to_doc(pointer) - view.to_doc(d.press.origin);
                let edit = match d.target {
                    Target::Point(index) => WarpEdit::Point { index, by },
                    Target::Surface(s, t) => WarpEdit::Surface { s, t, by },
                };
                gesture.update(edit, mods);
            }
        }
        if down {
            return Some(d);
        }
        if d.began {
            gesture.commit();
        }
        None
    }

    /// The pointer (PhotoCraft's): a cross hair over a point, a hand over the surface.
    fn cursor(&self, ctx: &Context, canvas: &Response, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, drag: Option<Drag>) {
        let target = match drag {
            Some(d) => (!d.cancelled).then_some(d.target),
            None if Claim::owner(ctx, canvas.id).is_none() => canvas.hover_pos().and_then(|p| self.target_at(view, screen, style, p)),
            None => None,
        };
        match target {
            Some(Target::Point(_)) => ctx.set_cursor_icon(CursorIcon::Crosshair),
            Some(Target::Surface(..)) => ctx.set_cursor_icon(if drag.is_some() { CursorIcon::Grabbing } else { CursorIcon::Grab }),
            None => {}
        }
    }

    /// The keyboard parts and their AccessKit nodes: anchors row by row, each followed by its
    /// handles.
    fn parts(&self, screen: &[Pos2], style: &WarpStyle, picked: Option<usize>) -> Vec<Part> {
        let w = self.mesh.width();
        let reach = vec2(2.0 * style.hit, 2.0 * style.hit);
        let value = |index: usize| {
            let p = self.mesh.points[index];
            self.units.map(|(x, y)| format!("{}, {}", x.format_value(p.x, self.zero.x), y.format_value(p.y, self.zero.y)))
        };
        let part = |index: usize, name: String| Part {
            key: index as u64,
            name,
            value: value(index),
            rect: egui::Rect::from_center_size(screen[index], reach),
            selected: picked == Some(index),
        };
        let mut parts = Vec::new();
        for anchor in self.mesh.anchors() {
            let (row, column) = (anchor / w / 3 + 1, anchor % w / 3 + 1);
            parts.push(part(anchor, (self.names.anchor)(row, column)));
            for (index, side) in self.mesh.handles(anchor) {
                parts.push(part(index, (self.names.handle)(row, column, side)));
            }
        }
        parts
    }

    /// Draws the patch boundaries, a one-patch mesh's thirds and, when `editable`, the anchors and
    /// the handles the pointer is near or whose anchor is in `kept`.
    fn paint(&self, ui: &Ui, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, editable: bool, kept: [Option<usize>; 2]) {
        let painter = ui.painter_at(view.rect);
        let to_screen = view.affine();
        let tolerance = f64::from(style.flatten);
        if self.mesh.columns == 1 && self.mesh.rows == 1 {
            for path in self.mesh.thirds() {
                stroke_path(&painter, to_screen * path, tolerance, style.minor);
            }
        }
        for path in self.mesh.boundaries() {
            stroke_path(&painter, to_screen * path, tolerance, style.line);
        }
        if !editable {
            return;
        }
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.rect.contains(*p));
        let near = |p: Pos2| pointer.is_some_and(|h| h.distance(p) <= style.reveal);
        let ppp = ui.pixels_per_point();
        let mut ends = Vec::new();
        for anchor in self.mesh.anchors() {
            let a = screen[anchor];
            for (index, _) in self.mesh.handles(anchor) {
                let p = screen[index];
                if near(p) || near(a) || kept.contains(&Some(anchor)) {
                    painter.line_segment([a, p], style.arm);
                    ends.push(p);
                }
            }
        }
        for anchor in self.mesh.anchors() {
            painter.extend(Handle::shapes_with(&style.anchor, HandleShape::Square, screen[anchor], ppp));
        }
        for p in ends {
            painter.extend(Handle::shapes_with(&style.control, HandleShape::Round, p, ppp));
        }
    }
}

/// The gesture's host value while idle: an edit that moves nothing, so a key's Set always differs.
const IDLE: WarpEdit = WarpEdit::Point { index: usize::MAX, by: Vec2::ZERO };

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::WarpGrid::drag")
}

/// Strokes a screen-space path of open curves as polylines within `tolerance` of them.
fn stroke_path(painter: &Painter, path: BezPath, tolerance: f64, stroke: Stroke) {
    let mut lines: Vec<Vec<Pos2>> = Vec::new();
    let pos = |p: Point| Pos2::new(p.x as f32, p.y as f32);
    kurbo::flatten(&path, tolerance, |el| match el {
        PathEl::MoveTo(p) => lines.push(vec![pos(p)]),
        PathEl::LineTo(p) => {
            if let Some(points) = lines.last_mut() {
                points.push(pos(p));
            }
        }
        _ => {}
    });
    painter.extend(lines.into_iter().filter(|l| l.len() > 1).map(|l| Shape::line(l, stroke)));
}

#[cfg(test)]
mod tests;
