//! The tolerance rule of layout memory: when a measured size asks for a discard pass.

use emath::Vec2;

/// The tolerance in points under which a measured size equals the one placement used: half a physical pixel.
pub fn measure_tolerance(pixels_per_point: f32) -> f32 {
    0.5 / pixels_per_point
}

/// Whether a measured size asks for a discard pass. `recorded` is the size placement used this frame
/// (`None` on a block's first appearance), `real` the size the block measured, `changed_last_frame` whether
/// the block's size also changed in the previous frame, `tolerance` the size under which two sizes are equal
/// ([`measure_tolerance`]). A first appearance, or a change after a settled
/// frame, asks for a discard pass. A change that follows a change is placed from last frame's value and
/// asks for none, so a size that changes every frame runs one pass.
pub fn measured_change_asks_for_discard(recorded: Option<Vec2>, real: Vec2, changed_last_frame: bool, tolerance: f32) -> bool {
    match recorded {
        None => true,
        Some(placed) => (placed - real).abs().max_elem() > tolerance && !changed_last_frame,
    }
}

/// A size that moves from one value to another over a duration, evaluated at the reader's clock.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SizeTween {
    /// The size at `start`.
    pub from: Vec2,
    /// The size at `start + duration` and after.
    pub to: Vec2,
    /// The clock reading, in seconds, when the tween began.
    pub start: f64,
    /// How long it runs, in seconds.
    pub duration: f64,
}

impl SizeTween {
    /// A tween from `from` to `to` that begins at `start` and runs for `duration`.
    pub const fn new(from: Vec2, to: Vec2, start: f64, duration: f64) -> Self {
        SizeTween { from, to, start, duration }
    }

    /// The size at clock reading `now`, eased in and out.
    pub fn at(&self, now: f64) -> Vec2 {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = ((now - self.start) / self.duration).clamp(0.0, 1.0) as f32;
        self.from + (self.to - self.from) * (t * t * (3.0 - 2.0 * t))
    }

    /// Whether the tween has reached its end at `now`.
    pub fn is_done(&self, now: f64) -> bool {
        now >= self.start + self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tween_runs_from_start_to_end_and_stays() {
        let tween = SizeTween::new(Vec2::ZERO, Vec2::new(100.0, 40.0), 1.0, 0.5);
        assert_eq!(tween.at(0.0), Vec2::ZERO);
        assert_eq!(tween.at(1.25), Vec2::new(50.0, 20.0));
        assert_eq!(tween.at(9.0), Vec2::new(100.0, 40.0));
        assert!(!tween.is_done(1.4) && tween.is_done(1.5));
    }

    #[test]
    fn relayout_asks_only_on_first_or_real_change() {
        let placed = Vec2::new(100.0, 20.0);
        let tolerance = measure_tolerance(1.0);
        // A first appearance asks; so does a real change after a settled frame, but not on the frame of a change.
        assert!(measured_change_asks_for_discard(None, placed, false, tolerance));
        assert!(measured_change_asks_for_discard(Some(placed), Vec2::new(100.0, 21.0), false, tolerance));
        assert!(!measured_change_asks_for_discard(Some(placed), Vec2::new(100.0, 21.0), true, tolerance));
        // Sub-pixel jitter stays quiet, and the tolerance shrinks as physical pixels get smaller.
        let jitter = Vec2::new(100.3, 20.0);
        assert!(!measured_change_asks_for_discard(Some(placed), jitter, false, measure_tolerance(1.0)));
        assert!(measured_change_asks_for_discard(Some(placed), jitter, false, measure_tolerance(2.0)));
    }
}
