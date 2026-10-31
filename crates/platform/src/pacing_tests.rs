//! Behaviour tests of `pacing` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A 100 fps cap lets the first frame start at once and spaces the next ones 10 ms apart; a late frame waits nothing.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn frame_cap_spaces_frames_by_the_minimum_interval() {
    todo!()
}

// Minimized or occluded viewports read Hidden; unknown visibility reads Shown.
#[test]
#[ignore = "skeleton"]
fn window_state_is_hidden_only_when_the_host_says_so() {
    todo!()
}

// On a manual clock a slice ends when the clock passes its budget; a budget without a hidden share gives no slice while hidden.
#[test]
#[ignore = "skeleton"]
fn work_slices_follow_the_window_state() {
    todo!()
}
