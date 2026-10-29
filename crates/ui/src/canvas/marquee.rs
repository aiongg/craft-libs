//! [`Marquee`]: selection outlines (marching ants, a selection band, an outline being drawn) and
//! [`MarqueeDrag`], the press-drag-release gesture that sizes one.

mod ants;
#[cfg(test)]
mod tests;

use std::time::Duration;

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, IdSalt, Key, Modifiers, Pos2, Rect as ScreenRect, Response, Sense, Shape, Stroke, Ui, UiBuilder, WidgetInfo, pos2};
use kurbo::{Point, Rect};

use super::{CanvasView, Claim, Readout};
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::{LocaleNumbers, Unit};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

use ants::Ants;

/// Opacity of the light line under an outline being drawn (PhotoCraft's alpha 140 of 255).
const DRAWING_HALO_ALPHA: f32 = 0.55;

/// How a marquee is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarqueeLook {
    /// Marching ants laid out in device pixels, for selections, marquee drags and zoom boxes.
    #[default]
    Ants,
    /// A static 1 point line with dashes over it, for selecting objects by dragging in a layout
    /// or vector app. It turns with the view.
    Band,
    /// An outline still being drawn (a lasso): a dark 1 point line over a wider translucent light
    /// line. A polygon is an open path in this look.
    Drawing,
}

impl MarqueeLook {
    /// Every look, in declaration order.
    pub const ALL: &'static [MarqueeLook] = &[MarqueeLook::Ants, MarqueeLook::Band, MarqueeLook::Drawing];

    /// The name in story args.
    pub const fn name(self) -> &'static str {
        match self {
            MarqueeLook::Ants => "ants",
            MarqueeLook::Band => "band",
            MarqueeLook::Drawing => "drawing",
        }
    }
}

/// A marquee's look, from [`Marquee::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MarqueeStyle {
    /// The ants' band and the band look's line: `overlay.marquee`.
    pub light: Color32,
    /// The ants' dashes, the band's dashes and the line of an outline being drawn:
    /// `overlay.marquee_dash`.
    pub dark: Color32,
    /// Length of the ants' dashes and gaps, in line widths: `marquee.dash`.
    pub dash: f32,
    /// How fast the ants march, in line widths per second: `marquee.speed`.
    pub speed: f32,
    /// Milliseconds between the ants' repaints: `marquee.tick_ms`.
    pub tick_ms: u32,
    /// Whether the ants march: off with reduced motion.
    pub march: bool,
    /// The band look's line: 1 point of `overlay.marquee`.
    pub band_line: Stroke,
    /// Dash length of the band look: `marquee.band_dash`.
    pub band_dash: f32,
    /// Gap between the band look's dashes: `marquee.band_gap`.
    pub band_gap: f32,
    /// The drawing look's light line: `marquee.drawing_halo` points of `overlay.marquee` at 0.55
    /// opacity.
    pub drawing_halo: Stroke,
    /// The drawing look's dark line: 1 point of `overlay.marquee_dash`.
    pub drawing_line: Stroke,
    /// Farthest a press may move and still be a click, in points: `marquee.click_slop`.
    pub click_slop: f32,
}

/// What a marquee outlines, in document space.
#[derive(Clone, Debug, PartialEq)]
enum Outline {
    Rect(Rect),
    Ellipse(Rect),
    Polygon(Vec<Point>),
    Segments(Vec<(Point, Point)>),
}

/// One run of an outline on the screen.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub points: Vec<Pos2>,
    pub closed: bool,
}

/// An outline over the document: `Marquee::rect(r, "Selection").look(MarqueeLook::Ants)
/// .bounds_value(&numbers, &unit).show(ui, &canvas, &view)`.
///
/// Shapes are given in document space and mapped through the [`CanvasView`] point by point, so a
/// rotated view draws the hinted rectangle where the release selects (PhotoCraft #2512). It is in
/// the AccessKit tree as one graphics object inside the host's canvas node, named by the host's
/// label and valued by [`value`](Marquee::value) or [`bounds_value`](Marquee::bounds_value), one
/// of which the host gives. It takes no focus or input. With reduced motion the ants stand still
/// and ask for no repaints.
#[derive(Clone, Debug)]
pub struct Marquee {
    outline: Outline,
    look: MarqueeLook,
    label: String,
    value: Option<String>,
    id_salt: Option<IdSalt>,
}

impl Marquee {
    fn new(outline: Outline, label: impl Into<String>) -> Self {
        Self { outline, look: MarqueeLook::Ants, label: label.into(), value: None, id_salt: None }
    }

    /// A rectangle, named `label` (the host's translated text, e.g. "Selection").
    pub fn rect(rect: Rect, label: impl Into<String>) -> Self {
        Marquee::new(Outline::Rect(rect), label)
    }

    /// The ellipse inscribed in `rect`, named `label`.
    pub fn ellipse(rect: Rect, label: impl Into<String>) -> Self {
        Marquee::new(Outline::Ellipse(rect), label)
    }

    /// A closed polygon (an open path in the [`Drawing`](MarqueeLook::Drawing) look), named
    /// `label`.
    pub fn polygon(points: impl IntoIterator<Item = Point>, label: impl Into<String>) -> Self {
        Marquee::new(Outline::Polygon(points.into_iter().collect()), label)
    }

    /// Separate segments, such as a traced outline's edges, named `label`. Slanted ends snap to
    /// pixel centres in the ants look, so the steps of a staircase meet.
    pub fn segments(segments: impl IntoIterator<Item = (Point, Point)>, label: impl Into<String>) -> Self {
        Marquee::new(Outline::Segments(segments.into_iter().collect()), label)
    }

    /// How it is drawn; marching ants by default.
    pub fn look(mut self, look: MarqueeLook) -> Self {
        self.look = look;
        self
    }

    /// The accessible value, the host's translated text.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The value as the bounds "W × H (X, Y)" with `numbers` and `unit` around each number.
    pub fn bounds_value(mut self, numbers: &LocaleNumbers, unit: &Unit) -> Self {
        self.value = Some(bounds_text(self.bounds(), numbers, unit));
        self
    }

    /// Tells marquees shown on one canvas apart.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens) -> MarqueeStyle {
        let (m, o) = (&tokens.component.marquee, &tokens.color.overlay);
        let (light, dark) = (o.marquee.to_egui(), o.marquee_dash.to_egui());
        MarqueeStyle {
            light,
            dark,
            dash: m.dash,
            speed: m.speed,
            tick_ms: m.tick_ms,
            march: !tokens.appearance.reduced_motion,
            band_line: Stroke::new(1.0, light),
            band_dash: m.band_dash,
            band_gap: m.band_gap,
            drawing_halo: Stroke::new(m.drawing_halo, light.gamma_multiply(DRAWING_HALO_ALPHA)),
            drawing_line: Stroke::new(1.0, dark),
            click_slop: m.click_slop,
        }
    }

    /// The document bounds of the outline.
    fn bounds(&self) -> Rect {
        match &self.outline {
            Outline::Rect(r) | Outline::Ellipse(r) => r.abs(),
            Outline::Polygon(points) => bounds_of(points.iter().copied()),
            Outline::Segments(segments) => bounds_of(segments.iter().flat_map(|&(a, b)| [a, b])),
        }
    }

    /// The outline's runs on the screen under `view`.
    pub(crate) fn paths(&self, view: &CanvasView, look: MarqueeLook) -> Vec<Path> {
        let map = |points: &[Point]| points.iter().map(|&p| view.to_screen(p)).collect::<Vec<_>>();
        match &self.outline {
            Outline::Rect(r) => vec![Path { points: view.quad(*r).to_vec(), closed: true }],
            Outline::Ellipse(r) => vec![Path { points: ellipse_points(view, r.abs()), closed: true }],
            Outline::Polygon(points) => vec![Path { points: map(points), closed: look != MarqueeLook::Drawing }],
            Outline::Segments(segments) => segments.iter().map(|&(a, b)| Path { points: map(&[a, b]), closed: false }).collect(),
        }
    }

    /// Draws the outline over `view` and describes it to assistive technology inside the host's
    /// `canvas` node.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) {
        let tokens = Tokens::of(ui);
        let style = Marquee::style(&tokens);
        let paths = self.paths(view, self.look);
        let painter = ui.painter_at(view.rect);
        match self.look {
            MarqueeLook::Ants => {
                let time = if style.march { ui.input(|i| i.time) } else { 0.0 };
                let ants = Ants::new(ui.pixels_per_point(), time, style.dash, style.speed, style.light, style.dark);
                let lines = paths.iter().flat_map(|p| ants::polyline(&p.points, p.closed));
                ants::draw(&painter, lines, ants, matches!(self.outline, Outline::Segments(_)));
                if style.march {
                    ui.ctx().request_repaint_after(Duration::from_millis(u64::from(style.tick_ms)));
                }
            }
            MarqueeLook::Band => {
                for path in &paths {
                    painter.add(line(&path.points, path.closed, style.band_line));
                    let mut dashed = path.points.clone();
                    if path.closed
                        && let Some(&first) = path.points.first()
                    {
                        dashed.push(first);
                    }
                    painter.extend(Shape::dashed_line(&dashed, Stroke::new(style.band_line.width, style.dark), style.band_dash, style.band_gap));
                }
            }
            MarqueeLook::Drawing => {
                for path in &paths {
                    painter.add(line(&path.points, path.closed, style.drawing_halo));
                    painter.add(line(&path.points, path.closed, style.drawing_line));
                }
            }
        }
        self.describe(ui, canvas, view, &paths);
    }

    /// The AccessKit node: a hover-only widget over the outline's screen bounds, a child of the
    /// canvas node.
    fn describe(&self, ui: &mut Ui, canvas: &Response, view: &CanvasView, paths: &[Path]) {
        debug_assert!(!self.label.trim().is_empty(), "a marquee needs the host's name");
        debug_assert!(self.value.is_some(), "a marquee needs the host's value: Marquee::value or Marquee::bounds_value");
        let id = canvas.id.with("craft_ui::canvas::Marquee").with_salt(self.id_salt.unwrap_or_else(|| IdSalt::new("marquee")));
        let screen = paths.iter().flat_map(|p| p.points.iter().copied()).collect::<Vec<_>>();
        let bounds = ScreenRect::from_points(&screen).intersect(view.rect);
        let rect = if bounds.is_positive() { bounds } else { ScreenRect::from_center_size(view.rect.center(), egui::Vec2::ZERO) };
        let nodes = ui.new_child(UiBuilder::new().max_rect(view.rect).accessibility_parent(canvas.id));
        let response = nodes.interact(rect, id, Sense::hover());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::GraphicsObject, nodes.is_enabled(), &self.label);
            info.current_text_value.clone_from(&self.value);
            info
        });
    }
}

fn line(points: &[Pos2], closed: bool, stroke: Stroke) -> Shape {
    if closed { Shape::closed_line(points.to_vec(), stroke) } else { Shape::line(points.to_vec(), stroke) }
}

fn bounds_of(points: impl Iterator<Item = Point>) -> Rect {
    points.fold(None, |r: Option<Rect>, p| Some(r.map_or(Rect::from_points(p, p), |r| r.union_pt(p)))).unwrap_or_default()
}

/// "W × H (X, Y)" with the unit around each number.
fn bounds_text(bounds: Rect, numbers: &LocaleNumbers, unit: &Unit) -> String {
    let n = |v: f64| format!("{}{}{}", unit.prefix(), numbers.format(v, 0..=2), unit.suffix());
    format!("{} × {} ({}, {})", n(bounds.width()), n(bounds.height()), n(bounds.x0), n(bounds.y0))
}

/// The vertices of the ellipse inscribed in the document rectangle `r`, on the screen. An
/// axis-aligned view maps it onto the ellipse inscribed in the screen box of two opposite
/// corners, drawn with its vertices where they are: snapping each on its own turns the nearly
/// flat arcs at the edge midpoints into 1 px stairs whose antialiased steps wash the ants out to
/// grey. Any other rotation samples the document ellipse and maps each point, with as many
/// vertices as the same ellipse would get on the screen (PhotoCraft #2512).
fn ellipse_points(view: &CanvasView, r: Rect) -> Vec<Pos2> {
    if view.axis_aligned() {
        let screen = ScreenRect::from_two_pos(view.to_screen(Point::new(r.x0, r.y0)), view.to_screen(Point::new(r.x1, r.y1)));
        let (c, a, b) = (screen.center(), screen.width() / 2.0, screen.height() / 2.0);
        let n = (((a.abs() + b.abs()) * 0.5).clamp(16.0, 256.0)) as usize;
        return (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * std::f32::consts::TAU;
                pos2(c.x + a * t.cos(), c.y + b * t.sin())
            })
            .collect();
    }
    let screen = ScreenRect::from_points(&view.quad(r));
    let n = (((screen.width() + screen.height()) * 0.25) as usize).clamp(16, 256);
    let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            view.to_screen(Point::new(c.x + rx * t.cos(), c.y + ry * t.sin()))
        })
        .collect()
}

/// The two corners of a marquee in document space, as dragged: not ordered, not rounded.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MarqueeRect {
    pub a: Point,
    pub b: Point,
}

impl MarqueeRect {
    /// The rectangle between the corners, with `x0 <= x1` and `y0 <= y1`.
    pub fn rect(&self) -> Rect {
        Rect::from_points(self.a, self.b)
    }
}

/// What a [`MarqueeDrag`] sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarqueeShape {
    /// A rectangle.
    #[default]
    Rect,
    /// An ellipse inscribed in the rectangle: the corners are the same.
    Ellipse,
}

impl MarqueeShape {
    /// The marquee named `label` that draws `rect` as this shape.
    pub fn marquee(self, rect: Rect, label: impl Into<String>) -> Marquee {
        match self {
            MarqueeShape::Rect => Marquee::rect(rect, label),
            MarqueeShape::Ellipse => Marquee::ellipse(rect, label),
        }
    }
}

/// The options bar's marquee style.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Constraint {
    /// Sized by the drag; ⇧ makes it square.
    #[default]
    Free,
    /// Width and height keep the ratio `(w, h)`; the larger drag extent wins.
    Ratio(f64, f64),
    /// A fixed size `(w, h)` in document units: the drag positions it. The marquee begins at
    /// the press and follows the pointer, and a click places it there.
    Size(f64, f64),
}

impl Constraint {
    /// The corners of the marquee dragged from `start` to `end`: the style applied, ⇧ (`square`)
    /// making it square when free, ⌥ (`centre`) drawing it from the centre. For a sized marquee
    /// drawn from the centre the drag is the half extent; a fixed size is positioned at `end`,
    /// centred on it with ⌥.
    pub fn corners(self, start: Point, end: Point, square: bool, centre: bool) -> (Point, Point) {
        if let Constraint::Size(w, h) = self {
            let (w, h) = (w.max(1.0), h.max(1.0));
            let a = if centre { Point::new(end.x - w / 2.0, end.y - h / 2.0) } else { end };
            return (a, Point::new(a.x + w, a.y + h));
        }
        let (dx, dy) = (end.x - start.x, end.y - start.y);
        let (sx, sy) = (if dx < 0.0 { -1.0 } else { 1.0 }, if dy < 0.0 { -1.0 } else { 1.0 });
        let e = match self {
            Constraint::Ratio(w, h) if w > 0.0 && h > 0.0 => {
                let k = (dx.abs() / w).max(dy.abs() / h);
                Point::new(start.x + sx * k * w, start.y + sy * k * h)
            }
            _ if square => {
                let m = dx.abs().max(dy.abs());
                Point::new(start.x + sx * m, start.y + sy * m)
            }
            _ => end,
        };
        if !centre {
            return (start, e);
        }
        let (dx, dy) = (e.x - start.x, e.y - start.y);
        (Point::new(start.x - dx, start.y - dy), Point::new(start.x + dx, start.y + dy))
    }
}

/// The pixel rectangle of a fixed-size marquee between the corners `a` and `b`: exactly `size`
/// `(w, h)` in whole pixels (at least 1 × 1). Snapping both edges outwards added a pixel to each
/// side whenever the press sat between pixels, so 10 × 10 read 11 × 11. The near edge goes to the
/// closest pixel edge and the far one is the size away.
pub fn fixed_size_pixels(a: Point, b: Point, size: (f64, f64)) -> Rect {
    let (w, h) = (size.0.round().max(1.0), size.1.round().max(1.0));
    let (x0, y0) = (a.x.min(b.x).round(), a.y.min(b.y).round());
    Rect::new(x0, y0, x0 + w, y0 + h)
}

/// The pixel rectangle a marquee between two corners selects: both edges snapped outwards to
/// whole pixels, so every pixel the marquee touches is in.
pub fn marquee_pixels(a: Point, b: Point) -> Rect {
    Rect::new(a.x.min(b.x).floor(), a.y.min(b.y).floor(), a.x.max(b.x).ceil(), a.y.max(b.y).ceil())
}

/// A square marquee's pixel rectangle `r`, as [`marquee_pixels`] returns it, with equal sides. Snapping each axis on its own made one side a pixel longer than the other when
/// the press sat between pixels (a ⇧ drag read 101 × 102). The shorter side grows to the longer
/// one, away from the press point `start` towards `end`, so the square still covers every pixel
/// the drag touched.
pub fn square_pixels(r: Rect, start: Point, end: Point) -> Rect {
    let n = r.width().max(r.height());
    let grow = |lo: f64, hi: f64, from: f64, to: f64| {
        let d = (n - (hi - lo)).max(0.0);
        if to < from { (lo - d, hi) } else { (lo, hi + d) }
    };
    let (x0, x1) = grow(r.x0, r.x1, start.x, end.x);
    let (y0, y1) = grow(r.y0, r.y1, start.y, end.y);
    Rect::new(x0, y0, x1, y1)
}

/// What [`MarqueeDrag::show`] reports for a frame.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct MarqueeResponse {
    /// The gesture's event this frame: `Begin` once the pointer is past the click slop (at the
    /// press with [`Constraint::Size`]), `Update`
    /// every frame, `Commit` on release, `Cancel` on Esc.
    pub event: Option<EditEvent<MarqueeRect>>,
    /// A press released within the click slop of where it went down (PhotoCraft deselects). Never
    /// set with [`Constraint::Size`]: that marquee begins at the press and a click commits it.
    pub clicked: bool,
    /// The modifiers held at the press, for the host's selection mode (add, subtract). ⇧ and ⌥
    /// held at the press mean that mode, not square or centre, until released and pressed again.
    pub press_modifiers: Modifiers,
    /// The marquee as it is now, while the gesture runs: the host rounds it to pixels or applies
    /// its own rules and draws a [`Marquee`] of the rectangle it will commit.
    pub rect: Option<Rect>,
}

/// The press-drag-release gesture of a marquee tool.
///
/// The press on the canvas takes the canvas's [`Claim`], so run it before the host's tool input
/// and skip that input while [`Claim::owner`] is set. Past `marquee.click_slop` it begins; ⇧ makes
/// the marquee square (a circle) and ⌥ draws it from the centre, each only when pressed after the
/// press or released and pressed again; Space held moves the whole marquee; Esc cancels. A press
/// released inside the slop is a click, with no event; with [`Constraint::Size`] the gesture begins at the
/// press instead, previewing the fixed box at the pointer, and a click commits it there. Keyboard nudging of the finished
/// selection is the host's: [`nudge::read`](crate::behavior::nudge::read).
///
/// ```
/// use craft_ui::canvas::{CanvasView, Marquee, MarqueeDrag, kurbo::{Point, Rect}};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let drag = MarqueeDrag::new("rect-marquee").show(ui, &canvas, &view);
/// if let Some(r) = drag.rect {
///     Marquee::rect(r, "Selection").value(format!("{:.0} × {:.0}", r.width(), r.height())).show(ui, &canvas, &view);
/// }
/// # });
/// ```
#[must_use = "show the gesture"]
pub struct MarqueeDrag<'a> {
    salt: IdSalt,
    shape: MarqueeShape,
    constraint: Constraint,
    snap: Option<Box<dyn Fn(Point, Modifiers) -> Point + 'a>>,
    policy: EditPolicy,
    modifiers: Option<Modifiers>,
}

/// The press being tracked, in egui memory between frames.
#[derive(Clone, Debug)]
struct Press {
    /// Where the press went down on the screen.
    origin: Pos2,
    modifiers: Modifiers,
    live: Modifiers,
    released_shift: bool,
    released_alt: bool,
    pointer: Pos2,
    /// The corner the drag started from, and where it is now, in document space.
    start: Point,
    end: Point,
    begun: bool,
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); the gesture
    /// has not begun and nothing is reported.
    waits: bool,
}

impl Press {
    /// ⇧ and ⌥ for the marquee: held now, and either not held at the press or released since.
    fn fresh(&self) -> (bool, bool) {
        (self.live.shift && (!self.modifiers.shift || self.released_shift), self.live.alt && (!self.modifiers.alt || self.released_alt))
    }
}

impl<'a> MarqueeDrag<'a> {
    /// A marquee gesture; `id_source` tells it from other gestures on the canvas (the rectangle
    /// and the ellipse tool).
    pub fn new(id_source: impl AsIdSalt) -> Self {
        Self {
            salt: IdSalt::new(id_source),
            shape: MarqueeShape::Rect,
            constraint: Constraint::Free,
            snap: None,
            policy: EditPolicy::default(),
            modifiers: None,
        }
    }

    /// What the marquee outlines; see [`MarqueeShape::marquee`].
    pub fn shape(mut self, shape: MarqueeShape) -> Self {
        self.shape = shape;
        self
    }

    /// The options bar's style: free, a fixed ratio or a fixed size.
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraint = constraint;
        self
    }

    /// The host's snapping of a corner, given the document point and the modifiers held.
    pub fn snap(mut self, snap: impl Fn(Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The modifiers every test of the marquee reads (the press, the Shift and Alt rules, the snap) in place of the input state: a touch
    /// host's sticky modifiers, or Ctrl turning snapping off.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// The edit policy: realtime and cancellable by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The marquee named `label` that draws `rect` as this gesture's shape.
    pub fn marquee(&self, rect: Rect, label: impl Into<String>) -> Marquee {
        self.shape.marquee(rect, label)
    }

    fn snapped(&self, p: Point, modifiers: Modifiers) -> Point {
        self.snap.as_ref().map_or(p, |snap| snap(p, modifiers))
    }

    /// Runs the gesture for this pass on the host's canvas `response`.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> MarqueeResponse {
        let ctx = ui.ctx().clone();
        let id = canvas.id.with_salt(self.salt);
        let key = id.with("press");
        let slop = Marquee::style(&Tokens::of(ui)).click_slop;
        let (pressed, down, released, input_modifiers, pointer, origin) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.modifiers,
                i.pointer.latest_pos(),
                i.pointer.press_origin(),
            )
        });
        let modifiers = self.modifiers.unwrap_or(input_modifiers);
        let mut press: Option<Press> = ctx.data(|d| d.get_temp(key));
        if let Some(p) = &mut press
            && p.waits
        {
            let turn = if Claim::is_owner(&ctx, canvas.id, id) { Claim::turn(&ctx, canvas.id, id) } else { Turn::Now };
            match turn {
                Turn::Waits => return MarqueeResponse::default(),
                Turn::Runs => p.waits = false,
                // The press that waited was superseded: a press of this pass is handled below.
                Turn::Now => {
                    ctx.data_mut(|d| d.remove::<Press>(key));
                    press = None;
                }
            }
        }
        if press.is_none()
            && pressed
            && canvas.enabled()
            && canvas.is_pointer_button_down_on()
            && let Some(origin) = origin.or(pointer)
            && Claim::take(&ctx, canvas.id, id)
        {
            let start = self.snapped(view.to_doc(origin), modifiers);
            press = Some(Press {
                origin,
                modifiers,
                live: modifiers,
                released_shift: false,
                released_alt: false,
                pointer: origin,
                start,
                end: start,
                begun: false,
                cancelled: false,
                waits: Claim::turn(&ctx, canvas.id, id) == Turn::Waits,
            });
        }
        let Some(mut press) = press else { return MarqueeResponse::default() };
        if press.waits {
            ctx.data_mut(|d| d.insert_temp(key, press));
            return MarqueeResponse::default();
        }

        // The marquee is the host's to draw from `rect`, not a value it stores, so the gesture's
        // own last report stands for the host's value (otherwise it would see an outside change).
        let shown = Gesture::<MarqueeRect>::reported(ui, id).map(|(_, value)| value).unwrap_or_default();
        let mut gesture = Gesture::load(ui, id, &shown, self.policy).focus_owners(&[]).enabled(canvas.enabled());
        if press.begun && !gesture.is_active() {
            press.cancelled = true;
        }
        press.live = modifiers;
        press.released_shift |= !modifiers.shift;
        press.released_alt |= !modifiers.alt;
        if let Some(pointer) = pointer {
            press.pointer = pointer;
        }
        let ended = released || !down;
        let mut rect = None;
        if !press.cancelled {
            let at_press = matches!(self.constraint, Constraint::Size(..));
            if !press.begun && (at_press || press.pointer.distance(press.origin) > slop) {
                press.begun = true;
                gesture.begin();
            }
            if press.begun {
                // Space moves the marquee instead of sizing it; its key press must not click the
                // focused canvas.
                let space = ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Space);
                    i.key_down(Key::Space)
                });
                let end = self.snapped(view.to_doc(press.pointer), modifiers);
                if space {
                    press.start += end - press.end;
                }
                press.end = end;
                let (square, centre) = press.fresh();
                let (a, b) = self.constraint.corners(press.start, press.end, square, centre);
                let value = MarqueeRect { a, b };
                rect = Some(value.rect());
                gesture.update(value, modifiers);
                if ended {
                    gesture.commit();
                }
            }
        }
        let clicked = ended && !press.begun && !press.cancelled;
        let press_modifiers = press.modifiers;
        let event = gesture.finish();
        if ended {
            ctx.data_mut(|d| d.remove::<Press>(key));
        } else {
            ctx.data_mut(|d| d.insert_temp(key, press));
        }
        MarqueeResponse { event, clicked, press_modifiers, rect }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Marquee"];

const SHAPES: &[&str] = &["rectangle", "ellipse"];
const CONSTRAINTS: &[&str] = &["free", "ratio 4:3", "size 200 x 120"];
const LOOKS: &[&str] = &["ants", "band", "drawing"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/marquee/drag",
        level: Level::Primitives,
        component: "Marquee",
        name: "Drag",
        args: || super::story::args().with_choice("look", LOOKS, 0).with_choice("shape", SHAPES, 0).with_choice("style", CONSTRAINTS, 0),
        show: show_drag,
    }]
}

/// Drag a marquee on the document: ⇧ square, ⌥ from the centre, Space moves it, Esc cancels. The
/// committed rectangle stays as the selection and the arrow keys nudge it while the canvas has
/// focus; a click deselects. The width and height follow the pointer in a readout.
fn show_drag(ui: &mut Ui, args: &mut Args) {
    let canvas = super::story::document(ui, args, "Document");
    let (response, view) = (&canvas.response, &canvas.view);
    let look = MarqueeLook::ALL.iter().copied().find(|l| l.name() == args.choice("look")).unwrap_or_default();
    let shape = if args.choice("shape") == SHAPES[1] { MarqueeShape::Ellipse } else { MarqueeShape::Rect };
    let constraint = match args.choice("style") {
        "ratio 4:3" => Constraint::Ratio(4.0, 3.0),
        "size 200 x 120" => Constraint::Size(200.0, 120.0),
        _ => Constraint::Free,
    };
    let selection_key = ui.scope_id().with("marquee-story-selection");
    let mut selection: Option<Rect> = ui.data(|d| d.get_temp(selection_key));
    let result = MarqueeDrag::new("marquee-story").shape(shape).constraint(constraint).show(ui, response, view);
    if result.clicked {
        selection = None;
    }
    if let Some(EditEvent::Commit { value, changed: true, .. }) = &result.event {
        selection = Some(value.rect());
    }
    if response.has_focus()
        && result.rect.is_none()
        && let Some(step) = crate::behavior::nudge::read(ui, crate::behavior::nudge::STEP, crate::behavior::nudge::BIG_STEP)
    {
        selection = selection.map(|r| r + step);
    }
    ui.data_mut(|d| {
        if let Some(r) = selection {
            d.insert_temp(selection_key, r);
        } else {
            d.remove::<Rect>(selection_key);
        }
    });
    // The marquee being dragged stands in for the selection until it commits.
    let shown = result.rect.or(selection);
    if let Some(rect) = shown {
        let numbers = LocaleNumbers::from_tag("en");
        shape.marquee(rect, "Selection").look(look).bounds_value(&numbers, &Unit::new("{n} px")).show(ui, response, view);
    }
    if let (Some(rect), Some(pointer)) = (result.rect, response.interact_pointer_pos()) {
        Readout::new("marquee-story-size")
            .row("W:", format!("{:.0}", rect.width()))
            .row("H:", format!("{:.0}", rect.height()))
            .show(ui.ctx(), pointer);
    }
}
