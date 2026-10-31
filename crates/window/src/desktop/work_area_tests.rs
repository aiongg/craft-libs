//! The visible part of the window.

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// A 1440 × 900 window on a 1366 × 768 monitor is cut to the monitor less TASKBAR; full screen and an unknown monitor give the content rect.
fn visible_part_cuts_to_the_monitor_less_the_taskbar() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// fits is false for 1440 × 900 on 1366 × 768 and true for 1280 × 720; a result under 160 pt falls back to the content rect.
fn fits_and_the_untrusted_fallback() {
    todo!()
}
