//! Tests of the tip angle and roundness widget (task BR-BS2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Ports PhotoCraft's: a press within 7 pt of a roundness dot grabs Roundness, elsewhere Angle.
#[test]
#[ignore = "skeleton"]
fn ellipse_grab_prefers_the_dots_within_7_pt() {
    todo!()
}

// Ports PhotoCraft's `ellipse_widget_drags_angle_and_roundness`: an Angle drag points the angle at
// the pointer in whole degrees; a Roundness drag sets |along minor| / radius, clamped to 0.01..=1.
#[test]
#[ignore = "skeleton"]
fn ellipse_drag_sets_angle_and_roundness() {
    todo!()
}

// A drag reports Begin, Update and Commit; Esc during the drag reports Cancel.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
#[ignore = "skeleton"]
fn ellipse_drag_is_one_gesture() {
    todo!()
}

// Arrows turn 1° (Shift 15°) and wrap at ±180; Alt+arrows change the roundness by 1 %; each a Set.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
#[ignore = "skeleton"]
fn ellipse_keys_turn_and_squash() {
    todo!()
}
