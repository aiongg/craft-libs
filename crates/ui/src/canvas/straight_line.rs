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
//! StraightLinePreview::new(last_stroke_end, pointer).paint(painter, view, &LineGuideStyle::of(tokens));
//! # }
//! ```
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Painter};
use kurbo::Point;

use super::CanvasView;
use crate::theme::UiTokens;

/// The look of a guide line drawn on the document (the Shift-click line, the Ruler line and its
/// protractor arm): a wide halo under a thin line, from the `line_guide` token family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineGuideStyle {
    /// Width of the line in points: `line_guide.width` (1).
    pub width: f32,
    /// Width of the halo under it: `line_guide.halo` (3; PhotoCraft drew the Shift-click line's
    /// 2.5 and the Ruler's 3, unified to 3).
    pub halo: f32,
    /// The line's colour: `overlay.stroke`.
    pub line: Color32,
    /// The halo's colour: `overlay.halo`.
    pub halo_color: Color32,
}

impl LineGuideStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // new: token task TK1 adds `line_guide.width` and `line_guide.halo`; the colours are the
        // existing `color.overlay.stroke` and `color.overlay.halo`.
        todo!()
    }
}

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
        // dark first; the footprint circle it adds for the Standard and Precise cursors is left to the
        // app's cursor).
        todo!()
    }
}
