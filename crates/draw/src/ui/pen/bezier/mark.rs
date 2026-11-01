//! What the path overlay draws besides outlines: the selected segment's look ([`SegmentMark`]),
//! whether anchors show ([`AnchorsShown`]), and the hover look of the part a press would take
//! ([`HitMark`]).

use egui::Painter;

use super::{BezierHit, BezierStyle, EditPath};
use craft_ui::canvas::CanvasView;

/// How a clicked segment ([`PathSelection::segment`](super::PathSelection::segment)) shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SegmentMark {
    /// Its two handles only, no thicker line (PhotoCraft, Photoshop).
    #[default]
    HandlesOnly,
    /// Its two handles and the segment in `bezier.selected_segment`.
    Line,
}

/// Whether the overlay draws anchors and handles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnchorsShown {
    /// Anchors, and the handles [`HandlesShown`](super::HandlesShown) names.
    #[default]
    All,
    /// Outlines only: no anchor, no handle, no AccessKit part (a path under a transform box,
    /// PhotoCraft's `draw_outline`). Use it with [`BezierTool::Show`](super::BezierTool::Show).
    Hidden,
}

/// The hover look of the part of a path a press would take, drawn by the overlay for the Add,
/// Delete, Auto Add/Delete and Curvature tools and by the pen draft for Auto Add/Delete on the
/// draft, and callable by a host on any path ([`BezierOverlay::highlight`](super::BezierOverlay::highlight)
/// draws it for a host-chosen hit):
///
/// - a segment: the segment in `bezier.selected_segment` over an `overlay.halo` line
///   `handle.halo` wider on each side, then a hollow anchor (`bezier.anchor`) at the segment's
///   point at `t`, where a click adds one (gap row 37);
/// - an anchor: the anchor enlarged to `bezier.close_anchor`;
/// - a handle's end: the end enlarged to `bezier.close_anchor`.
///
/// Colours are the path's own or the style's, as the overlay draws that path.
pub struct HitMark<'a> {
    #[expect(dead_code, reason = "skeleton: read by task PE-A3")]
    path: &'a EditPath,
    #[expect(dead_code, reason = "skeleton: read by task PE-A3")]
    hit: BezierHit,
}

impl<'a> HitMark<'a> {
    /// The mark of `hit` on `path` (a hit on another path draws nothing).
    pub fn new(path: &'a EditPath, hit: BezierHit) -> Self {
        Self { path, hit }
    }

    /// Draws the mark over `view` with `style`'s sizes.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-A3)")]
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &BezierStyle) {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:171-188 `draw_hit` (segment, then
        // `draw_anchor` at `t`; the library keeps its own segment and anchor looks, step7-pen lead
        // calls 3 and 5). new: moves the body of `BezierOverlay::paint_highlight` here and adds the
        // anchor at `t`; `paint_highlight` then calls this.
        todo!()
    }
}
