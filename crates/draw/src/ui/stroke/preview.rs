//! [`StrokePreview`]: a stroke style drawn as a line sample with egui shapes.

use egui::{Color32, Painter, Rect, Shape};

use super::StrokeStyle;

/// A line sample of a [`StrokeStyle`] in a rect, drawn with egui shapes (the library has no
/// vector rasteriser; PhotoCraft's samples use its engine). Lead call 20: the outline is dashed
/// with `kurbo::dash` from [`StrokeStyle::to_kurbo`] (the dashes scaled to the sample's width),
/// then each dash is filled as convex pieces: a quad per flattened segment, a fan for round joins
/// and caps, a triangle for bevels, a wedge for miters within the limit, a quad for square caps.
///
/// - Open ([`closed`](Self::closed) false, the default): a horizontal line through the rect's
///   centre from 4 points inside its left edge to 4 inside its right, aligned to the centre
///   whatever the style says; drawn left to right in every reading direction.
/// - Closed: a rectangle inset [`inset`](Self::inset) points from the rect, so corners and
///   alignment show (inside or outside a closed path's line by half the width).
/// - Opacity multiplies the colour's alpha. A style that fails [`StrokeStyle::validate`] draws
///   nothing.
pub struct StrokePreview<'a> {
    #[expect(dead_code, reason = "skeleton: read by task PE-A4")]
    value: &'a StrokeStyle,
    #[expect(dead_code, reason = "skeleton: read by task PE-A4")]
    width: f32,
    #[expect(dead_code, reason = "skeleton: read by task PE-A4")]
    color: Color32,
    closed: bool,
    inset: f32,
}

impl<'a> StrokePreview<'a> {
    /// A sample of `value` drawn `width` points wide (`stroke_editor.sample_stroke`, 2.5) in
    /// `color` (`text.default`).
    pub fn new(value: &'a StrokeStyle, width: f32, color: Color32) -> Self {
        Self { value, width, color, closed: false, inset: 12.0 }
    }

    /// Draws a closed rectangle in place of an open line.
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// The closed sample's inset from the rect, in points (`stroke_editor.sample_inset`, 12 by
    /// default).
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = inset;
        self
    }

    /// The sample's shapes in `rect` (screen points): every dash as filled convex polygons and
    /// circle fans, so tests count them.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-A4)")]
    pub fn shapes(&self, rect: Rect) -> Vec<Shape> {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:307-354 `paint_sample` (the
        // rectangle at inset 12 :321-322, the line 4 from each end at mid-height :324, centred when
        // open :314-316, width 2.5 :332). new: egui shapes from `kurbo::dash` (lead call 20).
        todo!()
    }

    /// Paints [`shapes`](Self::shapes) for `rect` with `painter`.
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        painter.extend(self.shapes(rect));
    }
}
