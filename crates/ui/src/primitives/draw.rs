//! `Draw`: what a component paints content with. It takes only `StyleColor` or `DataColor`.

use super::record::PaintRecord;
use super::rule::RuleKind;
use super::surface::Surface;
use super::text::{TextEnv, TextLayout, TextRun};
use crate::style::{Corners, DataColor, Ink, StyleColor, SurfaceKind, UiTokens, ink_color, rule_look, surface_style};
use craft_i18n::Direction;
use egui::epaint::{CircleShape, RectShape};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};
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
        TextEnv { tokens: self.env.tokens, direction: self.env.direction, ink: ink_color(self.env.tokens, self.env.ink) }
    }

    fn width(&self, width: StrokeWidth) -> f32 {
        let stroke = &self.env.tokens.stroke;
        match width {
            StrokeWidth::Hairline => 1.0 / self.painter.pixels_per_point().max(f32::EPSILON),
            StrokeWidth::Thin => stroke.thin,
            StrokeWidth::Medium => stroke.medium,
            StrokeWidth::Thick => stroke.thick,
        }
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

    /// Paints a laid-out run at `pos`. Records the first baseline and, when the layout cut text, the full text, as
    /// `text` does.
    pub fn galley(&self, pos: Pos2, layout: &TextLayout) {
        let mut record = self.record.borrow_mut();
        record.baseline.get_or_insert(pos.y + layout.first_baseline);
        if layout.elided && record.elided.is_none() {
            record.elided = Some(layout.full.clone());
        }
        drop(record);
        self.painter.galley(pos, layout.galley.clone(), ink_color(self.env.tokens, self.env.ink));
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
