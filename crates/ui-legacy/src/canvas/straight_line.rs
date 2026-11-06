//! The preview of a straight line drawn with Shift-click.
//!
//! While ⇧ is held with a painting tool and nothing is being painted, a thin line runs from the
//! end of the last stroke on this document to the pointer: the line a click will paint (Photoshop's
//! rubber band). Which tools connect and where the last stroke ended are the app's; this draws the
//! line. The Ruler tool's line and protractor arm use the same look ([`LineGuideStyle`]).
//!
//! ```no_run
//! use craft_ui::canvas::straight_line::{LineGuideStyle, StraightLinePreview};
//! # fn demo(painter: &egui::Painter, view: &craft_ui::canvas::CanvasView, tokens: &craft_ui::theme::UiTokens,
//! #         last_stroke_end: kurbo::Point, pointer: kurbo::Point) {
//! StraightLinePreview::new(last_stroke_end, pointer).paint(painter, view, &LineGuideStyle::new(tokens));
//! # }
//! ```
use egui::{Painter, Stroke};
use kurbo::Point;

use super::CanvasView;
pub use crate::style::line_guide::LineGuideStyle;

/// The Shift-click line from `from` (the end of the last stroke) to `to` (the pointer), both in
/// document units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StraightLinePreview {
    from: Point,
    to: Point,
}

impl StraightLinePreview {
    /// A line from `from` to `to` (document units).
    pub fn new(from: Point, to: Point) -> Self {
        Self { from, to }
    }

    /// Paints the line through `view`: the halo stroke `style.halo` wide, then the line
    /// `style.width` wide over it, as two segments. Nothing for a non-finite end. The brush tip at
    /// the pointer is the cursor's, not this line's.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &LineGuideStyle) {
        // pc: crates/ui-egui/src/tools/stroke_constraint.rs:101 `draw_line_preview` (two segments, wide
        // dark first; the footprint circle it adds is left to the app's cursor).
        if !(self.from.x.is_finite() && self.from.y.is_finite() && self.to.x.is_finite() && self.to.y.is_finite()) {
            return;
        }
        let ends = [view.to_screen(self.from), view.to_screen(self.to)];
        painter.line_segment(ends, Stroke::new(style.halo, style.halo_color));
        painter.line_segment(ends, Stroke::new(style.width, style.line));
    }
}
