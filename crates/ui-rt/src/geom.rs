//! Geometry. Public positions, vectors, sizes and rects are emath's (`Pos2`, `Vec2`, `Rect`), as in
//! every craft component; a size is a `Vec2`. kurbo appears only for paths ([`crate::Item::Path`]).

use emath::{Rect, Vec2};

/// A size in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SizeU {
    pub width: u32,
    pub height: u32,
}

impl SizeU {
    pub const fn new(width: u32, height: u32) -> SizeU {
        SizeU { width, height }
    }

    /// The logical size at `scale`.
    pub fn to_logical(self, scale: f32) -> Vec2 {
        Vec2::new(self.width as f32 / scale, self.height as f32 / scale)
    }

    /// `size` (logical points) in whole pixels at `scale`.
    pub fn from_logical(size: Vec2, scale: f32) -> SizeU {
        SizeU::new((size.x * scale).round().max(0.0) as u32, (size.y * scale).round().max(0.0) as u32)
    }
}

/// A rect in physical pixels, `x0..x1` by `y0..y1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RectU {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl RectU {
    pub const fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> RectU {
        RectU { x0, y0, x1, y1 }
    }

    /// The whole of a `size` target.
    pub const fn of_size(size: SizeU) -> RectU {
        RectU::new(0, 0, size.width, size.height)
    }

    pub fn width(&self) -> u32 {
        self.x1.saturating_sub(self.x0)
    }

    pub fn height(&self) -> u32 {
        self.y1.saturating_sub(self.y0)
    }

    pub fn area(&self) -> u64 {
        u64::from(self.width()) * u64::from(self.height())
    }

    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    /// Shares at least one pixel with `other`.
    pub fn overlaps(&self, other: RectU) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    /// Lies wholly inside `other`.
    pub fn within(&self, other: RectU) -> bool {
        self.x0 >= other.x0 && self.y0 >= other.y0 && self.x1 <= other.x1 && self.y1 <= other.y1
    }

    pub fn union(&self, other: RectU) -> RectU {
        RectU { x0: self.x0.min(other.x0), y0: self.y0.min(other.y0), x1: self.x1.max(other.x1), y1: self.y1.max(other.y1) }
    }

    /// The rect in logical points at `scale`.
    pub fn to_logical(self, scale: f32) -> Rect {
        Rect::from_min_max(emath::pos2(self.x0 as f32 / scale, self.y0 as f32 / scale), emath::pos2(self.x1 as f32 / scale, self.y1 as f32 / scale))
    }
}

/// Distances from the four physical edges of a rect, in logical points (a safe area, padding).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Margin {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Margin {
    pub const ZERO: Margin = Margin::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Margin {
        Margin { left, top, right, bottom }
    }

    pub const fn same(v: f32) -> Margin {
        Margin::new(v, v, v, v)
    }

    pub fn sum(&self) -> Vec2 {
        Vec2::new(self.left + self.right, self.top + self.bottom)
    }

    /// `rect` with the margin taken off its edges.
    pub fn shrink(&self, rect: Rect) -> Rect {
        Rect::from_min_max(rect.min + Vec2::new(self.left, self.top), rect.max - Vec2::new(self.right, self.bottom))
    }
}

/// Corner radii of a rounded rect, in logical points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const ZERO: CornerRadii = CornerRadii::same(0.0);

    pub const fn same(r: f32) -> CornerRadii {
        CornerRadii { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
    }

    pub fn is_zero(&self) -> bool {
        *self == CornerRadii::ZERO
    }

    pub fn max(&self) -> f32 {
        self.top_left.max(self.top_right).max(self.bottom_right).max(self.bottom_left)
    }
}

impl From<f32> for CornerRadii {
    fn from(r: f32) -> CornerRadii {
        CornerRadii::same(r)
    }
}

/// `damage` (logical points) as whole physical pixels inside a `size` target: each rect grown out
/// to the pixel grid, and rects that then share pixels joined, so that no pixel is in two rects and
/// a backend may draw each rect on its own.
pub fn damage_pixels(damage: &[Rect], scale: f32, size: SizeU) -> Vec<RectU> {
    let px = |v: f32, max: u32, up: bool| {
        let v = if up { (v * scale).ceil() } else { (v * scale).floor() };
        v.clamp(0.0, max as f32) as u32
    };
    let mut out: Vec<RectU> = Vec::with_capacity(damage.len());
    for r in damage {
        let mut r = RectU {
            x0: px(r.min.x, size.width, false),
            y0: px(r.min.y, size.height, false),
            x1: px(r.max.x, size.width, true),
            y1: px(r.max.y, size.height, true),
        };
        if r.is_empty() {
            continue;
        }
        while let Some(i) = out.iter().position(|o| o.overlaps(r)) {
            r = r.union(out.swap_remove(i));
        }
        out.push(r);
    }
    out
}

/// Whether `r` covers no area (emath's `Rect::is_positive` negated; NaN counts as empty).
pub(crate) fn is_empty(r: Rect) -> bool {
    !(r.min.x < r.max.x && r.min.y < r.max.y)
}

/// Layout constraints in logical points: a node picks a size between `min` and `max`. A max of
/// `f32::INFINITY` leaves that axis unbounded (scroll content).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    pub min: Vec2,
    pub max: Vec2,
}

impl Constraints {
    pub const fn tight(size: Vec2) -> Constraints {
        Constraints { min: size, max: size }
    }

    pub const fn loose(max: Vec2) -> Constraints {
        Constraints { min: Vec2::ZERO, max }
    }

    pub fn is_tight(&self) -> bool {
        self.min == self.max
    }

    /// The same maximum with no minimum.
    pub fn loosen(&self) -> Constraints {
        Constraints { min: Vec2::ZERO, max: self.max }
    }

    /// `size` clamped into the constraints.
    pub fn constrain(&self, size: Vec2) -> Vec2 {
        Vec2::new(size.x.clamp(self.min.x, self.max.x), size.y.clamp(self.min.y, self.max.y))
    }

    /// Tight on the cross axis width, unbounded height: what a vertical list gives its content.
    pub fn width_only(&self) -> Constraints {
        Constraints { min: Vec2::new(self.max.x, 0.0), max: Vec2::new(self.max.x, f32::INFINITY) }
    }

    /// The constraints with `margin` taken off both bounds.
    pub fn deflate(&self, margin: Margin) -> Constraints {
        let d = margin.sum();
        let shrink = |s: Vec2| Vec2::new((s.x - d.x).max(0.0), (s.y - d.y).max(0.0));
        Constraints { min: shrink(self.min), max: shrink(self.max) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_pixels_never_share_a_pixel() {
        // Apart in points, but both reach into pixel row 15 at scale 1.5.
        let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(emath::pos2(x0, y0), emath::pos2(x1, y1));
        let damage = [r(0.0, 0.0, 10.0, 10.2), r(0.0, 10.4, 10.0, 20.0)];
        let px = damage_pixels(&damage, 1.5, SizeU::new(100, 100));
        assert_eq!(px, vec![RectU { x0: 0, y0: 0, x1: 15, y1: 30 }]);
        // Touching rects stay apart; out-of-target parts are cut.
        let damage = [r(0.0, 0.0, 10.0, 10.0), r(0.0, 10.0, 200.0, 20.0)];
        let px = damage_pixels(&damage, 1.0, SizeU::new(100, 100));
        assert_eq!(px.len(), 2);
        assert!(px.iter().all(|r| r.x1 <= 100));
    }
}
