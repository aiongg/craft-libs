//! The tolerance rule of layout memory: when a measured size asks for a discard pass.

use emath::Vec2;

/// The tolerance in points under which a measured size equals the one placement used.
pub const MEASURE_TOLERANCE: f32 = 0.5;

/// Whether a measured size asks for a discard pass. `recorded` is the size placement used this frame
/// (`None` on a block's first appearance), `real` the size the block measured, `changed_last_frame` whether
/// the block's size also changed in the previous frame. A first appearance, or a change after a settled
/// frame, asks for a discard pass. A change that follows a change is placed from last frame's value and
/// asks for none, so a size that changes every frame runs one pass.
pub fn measured_change_asks_for_discard(recorded: Option<Vec2>, real: Vec2, changed_last_frame: bool) -> bool {
    match recorded {
        None => true,
        Some(placed) => (placed - real).abs().max_elem() > MEASURE_TOLERANCE && !changed_last_frame,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_first_appearance_or_a_change_after_a_settled_frame_asks() {
        let placed = Vec2::new(100.0, 20.0);
        assert!(measured_change_asks_for_discard(None, placed, false));
        assert!(measured_change_asks_for_discard(None, placed, true));
        assert!(!measured_change_asks_for_discard(Some(placed), Vec2::new(100.4, 20.0), false));
        assert!(measured_change_asks_for_discard(Some(placed), Vec2::new(100.0, 21.0), false));
        assert!(!measured_change_asks_for_discard(Some(placed), Vec2::new(100.0, 21.0), true));
    }
}
