//! What the path overlay draws besides outlines: the selected segment's look ([`SegmentMark`]),
//! whether anchors show ([`AnchorsShown`]), and the hover look of the part a press would take
//! ([`HitMark`]).

use egui::{Painter, Stroke};
use kurbo::{BezPath, ParamCurve as _};

use super::{BezierHit, BezierStyle, EditPath, adjusted, outline};
use craft_ui::canvas::CanvasView;
use craft_ui::primitives::{Handle, HandleShape};

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
    path: &'a EditPath,
    hit: BezierHit,
}

impl<'a> HitMark<'a> {
    /// The mark of `hit` on `path` (a hit on another path draws nothing).
    pub fn new(path: &'a EditPath, hit: BezierHit) -> Self {
        Self { path, hit }
    }

    /// Draws the mark over `view` with `style`'s sizes.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &BezierStyle) {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:171-188 `draw_hit` (segment, then
        // `draw_anchor` at `t`; the library keeps its own segment and anchor looks, step7-pen lead
        // calls 3 and 5).
        let (path, hit) = (self.path, self.hit);
        if hit.path() != path.key {
            return;
        }
        let ppp = painter.ctx().pixels_per_point();
        let color = path.color.unwrap_or(style.color);
        let tolerance = f64::from(style.flatten);
        let (anchor, _, control) = style.looks(path.color);
        let (point, shape, base) = match hit {
            BezierHit::Segment { segment, t, .. } => {
                if let Some(c) = path.subpaths.get(segment.subpath).and_then(|sub| sub.segment(segment.knot)) {
                    let mut seg = BezPath::new();
                    seg.move_to(c.p0);
                    seg.curve_to(c.p1, c.p2, c.p3);
                    let seg = view.affine() * seg;
                    outline(painter, seg.clone(), tolerance, Stroke::new(style.selected_segment + 2.0 * style.highlight_halo, style.halo));
                    outline(painter, seg, tolerance, Stroke::new(style.selected_segment, color));
                    painter.extend(Handle::shapes_with(&anchor, HandleShape::Square, view.to_screen(c.eval(t.clamp(0.0, 1.0))), ppp));
                }
                return;
            }
            BezierHit::Anchor { knot, .. } => (path.knot(knot).map(|k| k.anchor), HandleShape::Square, anchor),
            BezierHit::Handle { knot, side, .. } => (path.knot(knot).map(|k| k.handle(side)), HandleShape::Rotate, control),
        };
        if let Some(at) = point {
            let look = adjusted(base, |h| h.size = style.highlight_anchor);
            painter.extend(Handle::shapes_with(&look, shape, view.to_screen(at), ppp));
        }
    }
}
