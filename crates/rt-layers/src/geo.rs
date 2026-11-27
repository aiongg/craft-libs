//! kurbo-shaped helpers over emath's `Rect`, so the experiment's geometry code reads as written.

use craft_ui_rt::{Pos2, Rect};

/// A rect from its edges, `x0..x1` by `y0..y1`.
pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1))
}
