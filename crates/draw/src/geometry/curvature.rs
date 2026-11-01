//! The Curvature Pen's handles through a list of anchors.
//!
//! One rule draws, previews and edits Curvature paths everywhere: the pen draft's Curvature mode,
//! `BezierTool::Curvature` on finished paths, `EditSubpath::reflow`, and an app's engine.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-A2)")]

use std::ops::RangeInclusive;

use kurbo::Point;

/// The range a Curvature insert's segment parameter `t` is clamped to, so an anchor added right
/// at a segment's end still lands inside it (PhotoCraft's Curvature press). Add Anchor Point and
/// Auto Add take the hit's `t` as it is.
// pc: crates/ui-egui/src/tools/direct_select.rs:418 (`t.clamp(0.01, 0.99)`).
pub const CURVATURE_T_RANGE: RangeInclusive<f64> = 0.01..=0.99;

/// The Curvature Pen's handles for `anchors`: for each anchor its (in, out) handle points, so the
/// curve passes smoothly through every anchor (Catmull-Rom style, PhotoCraft's chordal rule).
///
/// - An inner anchor (every anchor when `closed`) lies on handles parallel to the chord from its
///   previous to its next neighbour (so they are collinear), the in handle a third of the
///   distance to the previous anchor long, the out handle a third of the distance to the next.
///   When the neighbours coincide, the direction is from the anchor to the next one.
/// - An open path's first anchor aims its out handle at the second anchor's in handle, and the
///   last anchor its in handle at the second-to-last anchor's out handle, each a third of the
///   distance to that neighbour long. The ends' other handles stay on their anchors.
/// - `corners[i]` true keeps anchor `i`'s handles on the anchor (a corner point); an index past
///   `corners` is smooth.
/// - Fewer than three anchors give straight segments: every handle on its anchor.
/// - Coincident points give handles on the anchor, never NaN.
///
/// The result has one entry per anchor.
pub fn curvature_handles(anchors: &[Point], corners: &[bool], closed: bool) -> Vec<(Point, Point)> {
    // pc: crates/vector/src/edit.rs:281-327 `curvature_knots` (constants and order as there: inner
    // anchors first, then the open ends from their neighbours' handles; `1e-9` as the zero length).
    todo!()
}
