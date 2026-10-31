//! Behaviour tests of `pacing`.
use super::*;

// A 100 fps cap lets the first frame start at once and spaces the next ones 10 ms apart; a late frame waits nothing.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn frame_cap_spaces_frames_by_the_minimum_interval() {
    let mut cap = FrameCap::new(100);
    let t0 = std::time::Instant::now();
    assert_eq!(cap.wait_at(t0), Duration::ZERO);
    assert_eq!(cap.wait_at(t0 + Duration::from_millis(3)), Duration::from_millis(7));
    // The second frame started at 10 ms, so the third may start at 20 ms.
    assert_eq!(cap.wait_at(t0 + Duration::from_millis(10)), Duration::from_millis(10));
    assert_eq!(cap.wait_at(t0 + Duration::from_millis(50)), Duration::ZERO);
    assert_eq!(cap.wait_at(t0 + Duration::from_millis(55)), Duration::from_millis(5));
}

// Minimized or occluded viewports read Hidden; unknown visibility reads Shown.
#[test]
fn window_state_is_hidden_only_when_the_host_says_so() {
    let info = |minimized, occluded| egui::ViewportInfo { minimized, occluded, ..Default::default() };
    assert_eq!(WindowState::of(&info(None, None)), WindowState::Shown);
    assert_eq!(WindowState::of(&info(Some(false), Some(false))), WindowState::Shown);
    assert_eq!(WindowState::of(&info(Some(true), None)), WindowState::Hidden);
    assert_eq!(WindowState::of(&info(Some(false), Some(true))), WindowState::Hidden);
}

// On a manual clock a slice ends when the clock passes its budget; a budget without a hidden share gives no slice while hidden.
#[test]
fn work_slices_follow_the_window_state() {
    let mut clock = Clock::manual(10.0);
    let budget = WorkBudget { shown: Duration::from_millis(30), hidden: None };
    assert_eq!(budget.slice(WindowState::Hidden, &clock), None);

    let slice = budget.slice(WindowState::Shown, &clock).expect("shown slice");
    assert!(slice.more(&clock));
    clock.advance(0.029);
    assert!(slice.more(&clock));
    clock.advance(0.001);
    assert!(!slice.more(&clock));

    let hidden = WorkBudget::DEFAULT.slice(WindowState::Hidden, &clock).expect("hidden slice");
    assert!(hidden.more(&clock));
}
