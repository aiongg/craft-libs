//! `Draw`: what a component paints content with. It takes only `StyleColor` or `DataColor`.

use super::content::DocText;
use super::record::PaintRecord;
use super::rule::RuleKind;
use super::surface::Surface;
use super::text::{TextEnv, TextLayout, TextRun};
use crate::fonts::stack_facts;
use crate::style::{Corners, DataColor, Ink, StyleColor, SurfaceKind, UiTokens, ink_color, rule_look, surface_style};
use craft_i18n::Direction;
use egui::epaint::{CircleShape, RectShape};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::cell::RefCell;

/// A shape `Draw::fill` and `Draw::stroke` paint.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PaintShape {
    /// A rect with corners.
    Rect(Rect, CornerRadius),
    /// A circle.
    Circle(Pos2, f32),
    /// A convex polygon.
    Polygon(Vec<Pos2>),
    /// An open path.
    Path(Vec<Pos2>),
}

/// A line width from the theme, so content never picks a literal one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StrokeWidth {
    /// One physical pixel.
    Hairline,
    /// `stroke.thin`.
    Thin,
    /// `stroke.medium`.
    Medium,
    /// `stroke.thick`.
    Thick,
}

impl StrokeWidth {
    /// The width in points on a display of `pixels_per_point`: the one place a named width becomes a number.
    pub(crate) fn points(self, tokens: &UiTokens, pixels_per_point: f32) -> f32 {
        let stroke = &tokens.stroke;
        match self {
            StrokeWidth::Hairline => 1.0 / pixels_per_point.max(f32::EPSILON),
            StrokeWidth::Thin => stroke.thin,
            StrokeWidth::Medium => stroke.medium,
            StrokeWidth::Thick => stroke.thick,
        }
    }
}

/// The lengths of a dashed stroke, in points along the path.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Dashes {
    /// The length of one dash.
    pub dash: f32,
    /// The length of the gap after a dash.
    pub gap: f32,
    /// How far along the pattern the path starts (animates marching ants).
    pub phase: f32,
}

impl Dashes {
    /// Dashes of `dash` with gaps of `gap`, starting at the pattern's beginning.
    pub fn new(dash: f32, gap: f32) -> Self {
        Dashes { dash, gap, phase: 0.0 }
    }

    /// Dashes of `length` with gaps of the same length, starting at the pattern's beginning.
    pub fn even(length: f32) -> Self {
        Dashes { dash: length, gap: length, phase: 0.0 }
    }

    /// The pattern advanced by `phase` points.
    pub fn phase(mut self, phase: f32) -> Self {
        self.phase = phase;
        self
    }
}

/// What a `Draw` reads from the env of the component it paints for.
#[derive(Clone, Copy)]
pub(crate) struct DrawEnv<'a> {
    pub tokens: &'a UiTokens,
    pub direction: Direction,
    pub ink: Ink,
}

/// The content-paint API `PaintUi::draw` gives.
pub struct Draw<'a> {
    painter: &'a Painter,
    record: &'a RefCell<PaintRecord>,
    env: DrawEnv<'a>,
}

impl<'a> Draw<'a> {
    pub(crate) fn new(painter: &'a Painter, record: &'a RefCell<PaintRecord>, env: DrawEnv<'a>) -> Self {
        Draw { painter, record, env }
    }

    fn text_env(&self) -> TextEnv<'a> {
        TextEnv {
            tokens: self.env.tokens,
            direction: self.env.direction,
            ink: ink_color(self.env.tokens, self.env.ink),
            fonts: stack_facts(self.painter.ctx()),
        }
    }

    fn width(&self, width: StrokeWidth) -> f32 {
        width.points(self.env.tokens, self.painter.pixels_per_point())
    }

    /// Lays `run` out in `rect`'s width and paints it at the reading start of `rect`, top aligned. Records the
    /// elision and the first baseline.
    pub fn text(&self, rect: Rect, run: &TextRun<'_>) -> TextLayout {
        let layout = run.layout(self.painter.ctx(), &self.text_env(), rect.width());
        let at_end = self.env.direction == Direction::Rtl && !run.ltr;
        let x = if at_end { rect.right() - layout.size().x } else { rect.left() };
        self.galley(Pos2::new(x, rect.top()), &layout);
        layout
    }

    /// Lays `text` out in `rect`'s width and paints it at the reading start of `rect`, top aligned, as `text` does for
    /// a run. Each run keeps its own font, size and colour.
    pub fn doc(&self, rect: Rect, text: &DocText<'_>) -> TextLayout {
        let text = DocText { wrap: Some(rect.width()), ..*text };
        let layout = text.layout(self.painter.ctx(), self.env.tokens, self.env.direction);
        let layout = TextLayout { galley: super::text::anchored_at_start(layout.galley.clone()), ..layout };
        let x = if self.env.direction == Direction::Rtl { rect.right() - layout.size().x } else { rect.left() };
        self.galley(Pos2::new(x, rect.top()), &layout);
        layout
    }

    /// Lays `run` out in `max_width` without painting it: the one text layout path for painters that measure before
    /// they paint (the same layout `text` paints with).
    pub fn layout(&self, run: &TextRun<'_>, max_width: f32) -> TextLayout {
        run.layout(self.painter.ctx(), &self.text_env(), max_width)
    }

    /// Paints a laid-out run at `pos`. Records the first baseline and, when the layout cut text, the full text, as
    /// `text` does.
    pub fn galley(&self, pos: Pos2, layout: &TextLayout) {
        let mut record = self.record.borrow_mut();
        record.baseline.get_or_insert(pos.y + layout.first_baseline);
        if layout.elided && record.elided.is_none() {
            record.elided = Some(layout.full.clone());
        }
        drop(record);
        super::text::note_painted(self.painter.ctx(), layout);
        // A run cut to nothing (an elided label in no room) has no glyphs, and egui gives it NaN bounds.
        if !layout.galley.mesh_bounds.is_finite() {
            return;
        }
        let ink = ink_color(self.env.tokens, self.env.ink);
        if layout.rotation == 0.0 {
            self.painter.galley(pos, layout.galley.clone(), ink);
        } else {
            let shape = egui::epaint::TextShape::new(pos, layout.galley.clone(), ink).with_angle_and_anchor(layout.rotation, egui::Align2::LEFT_TOP);
            self.painter.add(shape);
        }
    }

    /// Paints `icon` centred in `rect`, in the colour or ink the spec names, mirrored under right-to-left when it is
    /// directional.
    pub fn icon(&self, rect: Rect, icon: &super::IconSpec<'_>) {
        if let super::IconSource::Swatch(color) = &icon.source {
            let side = rect.width().min(rect.height());
            let shape = PaintShape::Rect(Rect::from_center_size(rect.center(), Vec2::splat(side)), CornerRadius::ZERO);
            let edge = icon.color.map_or_else(|| ink_color(self.env.tokens, icon.ink), StyleColor::egui);
            self.data_fill(&shape, DataColor::new(Color32::from(*color)));
            self.paint(&shape, None, Some(Stroke::new(self.width(StrokeWidth::Thin), edge)));
            return;
        }
        super::icon::paint(self.painter, self.env.tokens, self.env.direction, rect, icon);
    }

    /// Paints the reset dot of `look` centred on `center`.
    pub fn reset_mark(&self, look: &crate::style::ResetMarkLook, center: Pos2) {
        self.painter.extend(super::ResetMark::shapes(look, center));
    }

    /// Paints the line of `kind` from `from` to `to`.
    pub fn rule(&self, kind: RuleKind, from: Pos2, to: Pos2) {
        let look = rule_look(self.env.tokens, kind);
        self.painter.extend(super::rule::Rule::shapes(&look, from, to, self.painter.pixels_per_point()));
    }

    /// Paints a surface of `kind` (tracks, plot wells).
    pub fn surface(&self, rect: Rect, kind: SurfaceKind, corners: Corners) {
        let style = surface_style(self.env.tokens, kind);
        self.painter.extend(Surface::shapes(&style, None, rect, corners, self.env.direction, self.painter.pixels_per_point()));
    }

    /// Fills `shape`.
    pub fn fill(&self, shape: &PaintShape, color: StyleColor) {
        self.paint(shape, Some(color.egui()), None);
    }

    /// Strokes `shape`.
    pub fn stroke(&self, shape: &PaintShape, width: StrokeWidth, color: StyleColor) {
        self.paint(shape, None, Some(Stroke::new(self.width(width), color.egui())));
    }

    /// Fills `shape` with a colour from data (a swatch well, a layer label dot, an avatar).
    pub fn data_fill(&self, shape: &PaintShape, color: DataColor) {
        self.paint(shape, Some(color.0), None);
    }

    /// Strokes `shape` with a colour from data (a chart series, a curve).
    pub fn data_stroke(&self, shape: &PaintShape, width: StrokeWidth, color: DataColor) {
        self.paint(shape, None, Some(Stroke::new(self.width(width), color.0)));
    }

    /// Strokes the open path `path` as dashes (copy marks, drag previews, reference lines). [`kit::dashed`] is the one
    /// implementation; the kit painter's `dashed` calls it too.
    pub fn dashed(&self, path: &[Pos2], pattern: Dashes, width: StrokeWidth, color: StyleColor) {
        self.painter.extend(super::kit::dashed(path, pattern, self.width(width), color.egui()));
    }

    /// Runs `paint` with a `Draw` whose painting is cut to `rect` (inside the clip already in force): scrolled content
    /// under a frozen band, a bracket cut at a lane's edge.
    pub fn clipped(&self, rect: Rect, paint: impl FnOnce(&Draw<'_>)) {
        let painter = self.painter.with_clip_rect(rect);
        paint(&Draw { painter: &painter, record: self.record, env: self.env });
    }

    fn paint(&self, shape: &PaintShape, fill: Option<Color32>, stroke: Option<Stroke>) {
        let fill_color = fill.unwrap_or(Color32::TRANSPARENT);
        let stroke = stroke.unwrap_or(Stroke::NONE);
        let shape = match shape {
            PaintShape::Rect(rect, radius) => Shape::Rect(RectShape::new(*rect, *radius, fill_color, stroke, StrokeKind::Inside)),
            PaintShape::Circle(center, radius) => Shape::Circle(CircleShape { center: *center, radius: *radius, fill: fill_color, stroke }),
            PaintShape::Polygon(points) => Shape::convex_polygon(points.clone(), fill_color, stroke),
            PaintShape::Path(points) => Shape::line(points.clone(), stroke),
        };
        self.painter.add(shape);
    }
}
