//! The canvas outside a convex hole, as convex pieces.

use kurbo::Point;

/// Convex pieces that cover the canvas quad outside the convex `hole` exactly once: the canvas clipped by each
/// hole edge's outer half-plane in turn. Empty for a hole with fewer than three finite points or no area; the whole
/// canvas for a hole outside it.
pub fn outside_pieces(canvas: [Point; 4], hole: &[Point]) -> Vec<Vec<Point>> {
    let _ = (canvas, hole);
    todo!("tl.T42")
}
