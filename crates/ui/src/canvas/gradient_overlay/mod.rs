//! [`GradientOverlay`]: a gradient drawn on the canvas as PhotoCraft's live Gradient tool draws
//! it, a line between a start and an end handle with the colour stops and midpoint diamonds on it.

pub(crate) mod story;

use std::f32::consts::FRAC_PI_2;

use egui::{AsIdSalt, Color32, Context, Id, IdSalt, Painter, Pos2, Response, Shape, Stroke, Ui, vec2};

use super::gradient_line::{at_screen, midpoint_location};
use super::{CanvasView, GradientExtent, GradientLine, PartFocus};
use crate::composites::{GradientStops, StopRef, StopRow, StopRows, StopRules};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The names a screen reader reads for the overlay's group and parts, translated by the host.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientLineNames {
    /// The group: "Gradient".
    pub group: String,
    /// The line between the handles.
    pub line: String,
    /// The start handle: "Start point".
    pub start: String,
    /// The end handle: "End point".
    pub end: String,
}

/// A gradient line's look and reach, from [`GradientOverlay::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientLineStyle {
    /// The line and the circle and square extents: `gradient_line.line` of `overlay.handle_fill`.
    pub line: Stroke,
    /// The outline each side of those: `gradient_line.halo` of `overlay.halo`.
    pub halo: Stroke,
    /// Dash and gap of the mirrored extent: `gradient_line.dash`, `gradient_line.gap`.
    pub dash: (f32, f32),
    /// Radius of an end handle's ring: `gradient_line.end_ring`.
    pub end_ring: f32,
    /// An end handle's ring: `gradient_line.end_width` of `overlay.handle_fill`.
    pub end: Stroke,
    /// The outline each side of the ring: `gradient_line.end_halo` of `overlay.halo`.
    pub end_halo: Stroke,
    /// Radius of a colour stop's disc: `gradient_line.stop`.
    pub stop: f32,
    /// The ring just outside the disc: `gradient_line.stop_ring` of `overlay.handle_fill`.
    pub stop_ring: Stroke,
    /// The 1 pt ring outside that: `handle.halo` of `overlay.halo`.
    pub stop_halo: Stroke,
    /// Radius of a midpoint's diamond: `gradient_line.diamond`.
    pub diamond: f32,
    /// A diamond's fill: `overlay.handle_fill`.
    pub diamond_fill: Color32,
    /// A diamond's outline: `handle.halo` of `overlay.halo`.
    pub diamond_halo: Stroke,
    /// The selected stop's ring and the selected diamond's fill: `overlay.handle_stroke`.
    pub selected: Color32,
    /// The fill under a stop's colour where the colour has opacity: `overlay.handle_fill`.
    pub backing: Color32,
    /// Distance from a part within which the pointer takes it: `gradient_line.hit`.
    pub hit: f32,
    /// Farthest a press may move and still be a click: `gradient_line.click_slop`.
    pub click_slop: f32,
    /// Distance from the line past which a dragged stop is torn off: `gradient_line.tear_off`.
    pub tear_off: f32,
}

/// A gradient's line over a canvas: the line between its handles, the circle, square or mirrored
/// half its style adds, and the colour stops and midpoint diamonds on it, as PhotoCraft's Gradient
/// tool draws them. The host holds the gradient and passes the line and stops each frame.
///
/// ```
/// use craft_ui::canvas::{CanvasView, GradientLine, GradientLineNames, GradientOverlay, kurbo::Point};
/// use craft_ui::composites::{ColorStop, GradientStops};
/// use craft_theme::Srgba;
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let line = GradientLine { start: Point::new(120.0, 150.0), end: Point::new(280.0, 150.0) };
/// let stops = GradientStops {
///     color: vec![ColorStop::new(0.0, Srgba::BLACK), ColorStop::new(1.0, Srgba::WHITE)],
///     opacity: vec![],
/// };
/// let names = GradientLineNames { group: "Gradient".into(), line: "Gradient line".into(), start: "Start point".into(), end: "End point".into() };
/// GradientOverlay::new("gradient", &line, &stops, names).show(ui, &canvas, &view);
/// # });
/// ```
///
/// - Look: a 1.5 pt `overlay.handle_fill` line over an `overlay.halo` outline, the extent in the
///   line's style (the mirrored half dashed, no outline), rings at the ends, a disc in each stop's
///   colour with a ring round it, and a diamond between stops. Drawn in that order, so a stop at
///   an end sits inside its ring. Points go through the view, so a rotated, flipped or stretched
///   view draws the line where the document has it.
/// - Under [`StopRows::Split`] a stop's disc is opaque; under [`StopRows::Combined`] its colour
///   shows its opacity over a white backing.
/// - With [`reverse`](Self::reverse) location `t` is drawn at `1 − t` from the start.
/// - The selected stop or midpoint ([`select`](Self::select)) is drawn with its ring or fill in
///   `overlay.handle_stroke`, whether or not the overlay has focus.
/// - Nothing mirrors in a right-to-left interface: the line is the document's.
pub struct GradientOverlay<'a> {
    id_salt: IdSalt,
    line: &'a GradientLine,
    stops: &'a GradientStops,
    names: GradientLineNames,
    rules: StopRules,
    midpoints: bool,
    reverse: bool,
    extent: GradientExtent,
    editable: bool,
}

impl<'a> GradientOverlay<'a> {
    /// The host's gradient `line` and `stops`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, line: &'a GradientLine, stops: &'a GradientStops, names: GradientLineNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            line,
            stops,
            names,
            rules: StopRules::default(),
            midpoints: true,
            reverse: false,
            extent: GradientExtent::None,
            editable: true,
        }
    }

    /// The rules the host gives its gradient editor; the overlay reads the row layout from them.
    pub fn rules(mut self, rules: StopRules) -> Self {
        self.rules = rules;
        self
    }

    /// Whether midpoint diamonds show; true by default.
    pub fn midpoints(mut self, midpoints: bool) -> Self {
        self.midpoints = midpoints;
        self
    }

    /// Whether the gradient runs from the end to the start.
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// The shape the gradient's style adds round the line; decoration only.
    pub fn extent(mut self, extent: GradientExtent) -> Self {
        self.extent = extent;
        self
    }

    /// Whether the selection shows; true by default. False draws the line and its parts only.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// The overlay `id`'s selected stop or midpoint.
    pub fn selected(ctx: &Context, id: Id) -> Option<StopRef> {
        ctx.data(|d| d.get_temp(id.with(SELECTED)))
    }

    /// Selects a stop or midpoint of the overlay `id`.
    pub fn select(ctx: &Context, id: Id, selected: Option<StopRef>) {
        ctx.data_mut(|d| match selected {
            Some(s) => {
                d.insert_temp(id.with(SELECTED), s);
            }
            None => {
                d.remove::<StopRef>(id.with(SELECTED));
            }
        });
    }

    pub fn style(tokens: &UiTokens) -> GradientLineStyle {
        let (g, o, h) = (&tokens.component.gradient_line, &tokens.color.overlay, &tokens.component.handle);
        let (fill, halo) = (o.handle_fill.to_egui(), o.halo.to_egui());
        GradientLineStyle {
            line: Stroke::new(g.line, fill),
            halo: Stroke::new(g.halo, halo),
            dash: (g.dash, g.gap),
            end_ring: g.end_ring,
            end: Stroke::new(g.end_width, fill),
            end_halo: Stroke::new(g.end_halo, halo),
            stop: g.stop,
            stop_ring: Stroke::new(g.stop_ring, fill),
            stop_halo: Stroke::new(h.halo, halo),
            diamond: g.diamond,
            diamond_fill: fill,
            diamond_halo: Stroke::new(h.halo, halo),
            selected: o.handle_stroke.to_egui(),
            backing: fill,
            hit: g.hit,
            click_slop: g.click_slop,
            tear_off: g.tear_off,
        }
    }

    /// Draws the gradient over `view`, on the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> GradientOverlayResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let style = GradientOverlay::style(&Tokens::of(ui));
        let editable = self.editable && ui.is_enabled();
        let selected = if editable { Self::selected(ui.ctx(), id) } else { None };
        let (start, end) = (view.to_screen(self.line.start), view.to_screen(self.line.end));
        let painter = ui.painter_at(view.rect);
        self.paint(&painter, &style, start, end, selected);
        PartFocus::new(id, &self.names.group).focusable(false).show(ui, canvas);
        GradientOverlayResponse { id }
    }

    fn paint(&self, painter: &Painter, style: &GradientLineStyle, start: Pos2, end: Pos2, selected: Option<StopRef>) {
        self.paint_extent(painter, style, start, end);
        painter.extend(line_shapes(style, start, end));
        if self.midpoints {
            for i in 0..self.stops.color.len() {
                if let Some(t) = midpoint_location(self.stops, i) {
                    let on = selected == Some(StopRef::Midpoint(StopRow::Color, i));
                    painter.add(diamond(style, at_screen(start, end, t, self.reverse), on));
                }
            }
        }
        for p in [start, end] {
            painter.add(Shape::circle_stroke(p, style.end_ring + (style.end.width + style.end_halo.width) / 2.0, style.end_halo_stroke()));
            painter.add(Shape::circle_stroke(p, style.end_ring, style.end));
        }
        for (i, stop) in self.stops.color.iter().enumerate() {
            let at = at_screen(start, end, stop.location, self.reverse);
            let on = selected == Some(StopRef::Color(i));
            painter.extend(self.stop_shapes(style, at, Color32::from(stop.color), on));
        }
    }

    fn paint_extent(&self, painter: &Painter, style: &GradientLineStyle, start: Pos2, end: Pos2) {
        let d = end - start;
        match self.extent {
            GradientExtent::None => {}
            GradientExtent::Circle => {
                painter.extend(outlined(style, |stroke| vec![Shape::circle_stroke(start, d.length(), stroke)]));
            }
            GradientExtent::Square => {
                let turned = vec2(-d.y, d.x);
                let corners = vec![start + d, start + turned, start - d, start - turned];
                painter.extend(outlined(style, |stroke| vec![Shape::closed_line(corners.clone(), stroke)]));
            }
            GradientExtent::Mirror => {
                let far = start - d;
                painter.extend(Shape::dashed_line(&[start, far], style.line, style.dash.0, style.dash.1));
            }
        }
    }

    /// A colour stop: the disc, the ring just outside it, the halo ring outside that.
    fn stop_shapes(&self, style: &GradientLineStyle, at: Pos2, color: Color32, selected: bool) -> Vec<Shape> {
        let mut out = Vec::with_capacity(4);
        let (r, ring) = (style.stop, style.stop_ring.width);
        out.push(Shape::circle_stroke(at, r + ring + style.stop_halo.width / 2.0, style.stop_halo));
        match self.rules.rows {
            StopRows::Split => out.push(Shape::circle_filled(at, r, Color32::from_rgb(color.r(), color.g(), color.b()))),
            StopRows::Combined => {
                out.push(Shape::circle_filled(at, r, style.backing));
                out.push(Shape::circle_filled(at, r, color));
            }
        }
        let ring_color = if selected { style.selected } else { style.stop_ring.color };
        out.push(Shape::circle_stroke(at, r + ring / 2.0, Stroke::new(ring, ring_color)));
        out
    }
}

/// What [`GradientOverlay::show`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientOverlayResponse {
    /// The overlay's id, also its AccessKit group's.
    pub id: Id,
}

impl GradientLineStyle {
    /// The halo ring under an end handle's ring: wide enough to show `end_halo` each side of it.
    fn end_halo_stroke(&self) -> Stroke {
        Stroke::new(self.end.width + 2.0 * self.end_halo.width, self.end_halo.color)
    }
}

const SELECTED: &str = "craft_ui::canvas::GradientOverlay::selected";

/// Strokes in the line's style: the halo under, then the line.
fn outlined(style: &GradientLineStyle, shape: impl Fn(Stroke) -> Vec<Shape>) -> Vec<Shape> {
    let halo = Stroke::new(style.line.width + 2.0 * style.halo.width, style.halo.color);
    let mut out = shape(halo);
    out.extend(shape(style.line));
    out
}

fn line_shapes(style: &GradientLineStyle, start: Pos2, end: Pos2) -> Vec<Shape> {
    outlined(style, |stroke| vec![Shape::line_segment([start, end], stroke)])
}

/// A midpoint diamond centred on `at`.
fn diamond(style: &GradientLineStyle, at: Pos2, selected: bool) -> Shape {
    let r = style.diamond;
    let points = (0..4).map(|k| at + vec2((k as f32 * FRAC_PI_2).cos(), (k as f32 * FRAC_PI_2).sin()) * r).collect();
    let fill = if selected { style.selected } else { style.diamond_fill };
    Shape::convex_polygon(points, fill, style.diamond_halo)
}
