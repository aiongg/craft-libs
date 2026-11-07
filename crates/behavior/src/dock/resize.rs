//! A pointer-anchored resize of a floating window.

use emath::{Pos2, Rect, Vec2, pos2};

/// A pointer-anchored resize. The opposite edges and handle grab offset stay fixed
/// through boundary overshoot; viewport constraints never translate the window.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct AnchoredResize {
    start: Rect,
    offset: Vec2,
    direction: Vec2,
}

impl AnchoredResize {
    /// A resize of `start` grabbed at `pointer` by the handle on `direction`: each axis is -1 (the start
    /// edge), 0 (not resized) or 1 (the end edge). `None` for a rect or pointer that is not finite, or no direction.
    pub fn new(start: Rect, pointer: Pos2, direction: Vec2) -> Option<Self> {
        if !start.is_finite()
            || !start.is_positive()
            || !pointer.is_finite()
            || ![-1.0, 0.0, 1.0].contains(&direction.x)
            || ![-1.0, 0.0, 1.0].contains(&direction.y)
            || direction == Vec2::ZERO
        {
            return None;
        }
        let edge = pos2(if direction.x < 0.0 { start.left() } else { start.right() }, if direction.y < 0.0 { start.top() } else { start.bottom() });
        Some(Self { start, offset: pointer - edge, direction })
    }

    /// The window rect with the moving edges at `pointer`, kept inside `viewport` and between the sizes `min` and
    /// `max`. `None` suspends resizing when the fixed anchor cannot accommodate the minimum.
    pub fn rect(self, pointer: Pos2, viewport: Rect, min: Vec2, max: Vec2) -> Option<Rect> {
        if !pointer.is_finite()
            || !viewport.is_finite()
            || !viewport.is_positive()
            || !min.is_finite()
            || !max.is_finite()
            || min.x <= 0.0
            || min.y <= 0.0
            || max.x < min.x
            || max.y < min.y
            || !viewport.contains_rect(self.start)
            || self.start.width() < min.x
            || self.start.height() < min.y
        {
            return None;
        }
        let desired = pointer - self.offset;
        if !desired.is_finite() {
            return None;
        }
        let mut rect = self.start;
        for (axis, direction) in [(0, self.direction.x), (1, self.direction.y)] {
            if direction == 0.0 {
                continue;
            }
            let (lo, hi) = if direction < 0.0 {
                ((self.start.max[axis] - max[axis]).max(viewport.min[axis]), self.start.max[axis] - min[axis])
            } else {
                (self.start.min[axis] + min[axis], (self.start.min[axis] + max[axis]).min(viewport.max[axis]))
            };
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                return None;
            }
            if direction < 0.0 {
                rect.min[axis] = desired[axis].clamp(lo, hi);
            } else {
                rect.max[axis] = desired[axis].clamp(lo, hi);
            }
        }
        Some(rect)
    }
}
