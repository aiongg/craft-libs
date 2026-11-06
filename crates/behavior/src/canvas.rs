//! The transform a canvas body is shown through.

use emath::{Pos2, Vec2};

/// A canvas body's mapping from canvas coordinates to its layer: `screen = canvas * scale + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTransform {
    /// Layer points per canvas unit.
    pub scale: f32,
    /// Where the canvas origin lies in the layer.
    pub offset: Vec2,
}

impl CanvasTransform {
    /// The position in the layer of a canvas point.
    pub fn to_layer(&self, canvas: Pos2) -> Pos2 {
        Pos2::new(canvas.x * self.scale + self.offset.x, canvas.y * self.scale + self.offset.y)
    }

    /// The canvas point under a position in the layer.
    pub fn to_canvas(&self, layer: Pos2) -> Pos2 {
        Pos2::new((layer.x - self.offset.x) / self.scale, (layer.y - self.offset.y) / self.scale)
    }
}
