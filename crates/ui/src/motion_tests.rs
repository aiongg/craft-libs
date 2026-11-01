//! Caret and outline motion.

use egui::{Rect, pos2, vec2};

use craft_theme::{Easing, Motion};

use crate::motion::*;

fn token(millis: u32) -> Motion {
    Motion { millis, easing: Easing::Enter }
}

const VIEWPORT: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 600.0));

fn bar(x: f32, y: f32) -> CaretBar {
    [pos2(x, y), pos2(x, y + 16.0)]
}

fn frame(target: CaretBar, cause: u64, now: f64, frame_nr: u64) -> CaretFrame {
    CaretFrame { target, cause, view: 1, shown: true, viewport: VIEWPORT, now, frame_nr, keys: false, pointer: false, since_blink: 0.0 }
}

/// A caret resting at `x` on frame 1, time 10.
fn resting(glide: &mut CaretGlide, x: f32) {
    glide.frame(frame(bar(x, 100.0), 1, 10.0, 1));
}

// `ease`: the token's curve, 0 at 0, 1 at 1, ahead of linear early on; a non-finite input is 1.
#[test]
fn the_glide_curve_is_the_tokens() {
    for easing in [Easing::Standard, Easing::Enter, Easing::Exit] {
        assert!(ease(easing, 0.0).abs() < 1e-6 && (ease(easing, 1.0) - 1.0).abs() < 1e-6);
        assert!((ease(easing, f32::NAN) - 1.0).abs() < 1e-6 && (ease(easing, 2.0) - 1.0).abs() < 1e-6 && ease(easing, -1.0).abs() < 1e-6);
    }
    assert!(ease(Easing::Enter, 0.07) > 1.0 / 3.0 && ease(Easing::Exit, 0.5) < 0.5);
}

// `CaretGlide::with_motion`: the glide follows the token's easing, not a fixed curve.
#[test]
fn the_caret_glide_reads_the_tokens_easing() {
    let halfway = |easing| {
        let mut glide = CaretGlide::with_motion(Motion { millis: 100, easing });
        resting(&mut glide, 100.0);
        glide.frame(CaretFrame { keys: true, ..frame(bar(200.0, 100.0), 2, 10.1, 2) });
        glide.frame(frame(bar(200.0, 100.0), 2, 10.15, 3)).bar[0].x
    };
    assert!(halfway(Easing::Enter) > 150.0 && halfway(Easing::Exit) < 150.0);
}

// `CaretGlide::reset`: a caret that reappears after being hidden, with no edit, is drawn in place.
#[test]
fn a_hidden_caret_reappears_without_fading_in_as_an_edit() {
    let mut glide = CaretGlide::with_motion(token(100));
    resting(&mut glide, 100.0);
    glide.frame(CaretFrame { shown: false, keys: true, ..frame(bar(100.0, 100.0), 1, 10.1, 2) });
    let back = glide.frame(CaretFrame { keys: true, ..frame(bar(100.0, 100.0), 1, 10.2, 3) });
    assert_eq!((back.moving, glide.is_gliding()), (false, false), "the cause did not change, so it is no edit");
}

// `blink`: 1 for the first 0.53 s, then fades off over the fade time, off, and fades back on.
#[test]
fn a_resting_caret_blinks_with_fades() {
    let fade = 0.1;
    assert_eq!((blink(0.0, fade), blink(0.52, fade)), (1.0, 1.0));
    let half = blink(CARET_BLINK_SECS + 0.05, fade);
    assert!((half - 0.5).abs() < 1e-4, "{half}");
    assert_eq!(blink(CARET_BLINK_SECS + 0.3, fade), 0.0);
    let on_again = blink(2.0 * CARET_BLINK_SECS + 0.05, fade);
    assert!((on_again - 0.5).abs() < 1e-4, "{on_again}");
    assert_eq!(blink(2.0 * CARET_BLINK_SECS + 0.3, fade), 1.0);
    assert_eq!(blink(CARET_BLINK_SECS + 0.01, 0.0), 0.0, "no fade is a hard blink");
    assert_eq!(blink(f64::NAN, fade), 1.0);
}

// `CaretGlide::frame`: a changed cause with `keys` glides from the old place (half way after a short time); with `pointer` it is at the target at once.
#[test]
fn a_key_move_glides_and_a_click_jumps() {
    let mut keyed = CaretGlide::with_motion(token(100));
    resting(&mut keyed, 100.0);
    let start = keyed.frame(CaretFrame { keys: true, ..frame(bar(200.0, 100.0), 2, 10.1, 2) });
    assert_eq!(start.bar, bar(100.0, 100.0));
    assert!(start.moving);
    let later = keyed.frame(frame(bar(200.0, 100.0), 2, 10.1 + 0.02, 3));
    let x = later.bar[0].x;
    assert!(x > 100.0 && x < 200.0, "mid-glide at {x}");
    assert!(keyed.is_gliding());
    let done = keyed.frame(frame(bar(200.0, 100.0), 2, 10.1 + 0.2, 4));
    assert_eq!(done.bar, bar(200.0, 100.0));
    assert!(!keyed.is_gliding());

    let mut clicked = CaretGlide::with_motion(token(100));
    resting(&mut clicked, 100.0);
    let at = clicked.frame(CaretFrame { pointer: true, ..frame(bar(200.0, 100.0), 2, 10.1, 2) });
    assert_eq!(at.bar, bar(200.0, 100.0));
    assert!(!clicked.is_gliding());
}

// `CaretGlide::frame`: a new `view`, or a `frame_nr` two ahead, puts the caret at its target.
#[test]
fn a_view_change_or_a_skipped_frame_never_glides() {
    let mut zoomed = CaretGlide::with_motion(token(100));
    resting(&mut zoomed, 100.0);
    let look = zoomed.frame(CaretFrame { keys: true, view: 2, ..frame(bar(200.0, 100.0), 2, 10.1, 2) });
    assert_eq!(look.bar, bar(200.0, 100.0));

    let mut skipped = CaretGlide::with_motion(token(100));
    resting(&mut skipped, 100.0);
    let look = skipped.frame(CaretFrame { keys: true, ..frame(bar(200.0, 100.0), 2, 10.1, 3) });
    assert_eq!(look.bar, bar(200.0, 100.0));
    assert!(!skipped.is_gliding());
}

// `CaretGlide::with_motion(token(0))`: the caret is at its target, opacity 0 or 1.
#[test]
fn zero_seconds_jumps_and_blinks_hard() {
    let mut glide = CaretGlide::with_motion(token(0));
    resting(&mut glide, 100.0);
    let moved = glide.frame(CaretFrame { keys: true, ..frame(bar(200.0, 100.0), 2, 10.1, 2) });
    assert_eq!((moved.bar, moved.opacity, moved.moving), (bar(200.0, 100.0), 1.0, false));
    let off = glide.frame(CaretFrame { since_blink: CARET_BLINK_SECS + 0.01, ..frame(bar(200.0, 100.0), 2, 10.2, 3) });
    assert_eq!(off.opacity, 0.0);
}

fn outline(selection: u64, target: Rect, now: f64, frame_nr: u64) -> OutlineFrame {
    OutlineFrame { place: 1, selection, target, view: VIEWPORT, now, frame_nr, animate: true }
}

fn cell(x: f32, y: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(64.0, 16.0))
}

// `OutlineGlide::frame`: a new selection glides; the same selection moved on screen is drawn on its cells.
#[test]
fn the_outline_glides_edge_by_edge_and_stays_on_scrolled_cells() {
    let mut glide = OutlineGlide::with_motion(token(120));
    glide.frame(outline(1, cell(100.0, 100.0), 10.0, 1));
    // The selection grows to the right: only the right edge travels.
    let wide = Rect::from_min_max(pos2(100.0, 100.0), pos2(292.0, 116.0));
    let start = glide.frame(outline(2, wide, 10.1, 2));
    assert_eq!(start, cell(100.0, 100.0));
    assert!(glide.is_gliding());
    let mid = glide.frame(outline(2, wide, 10.1 + 0.03, 3));
    assert_eq!(mid.min, wide.min);
    assert!(mid.max.x > 164.0 && mid.max.x < 292.0, "{}", mid.max.x);
    let done = glide.frame(outline(2, wide, 10.1 + 0.2, 4));
    assert_eq!(done, wide);
    assert!(!glide.is_gliding());

    // The same cells scrolled: on their cells, no glide.
    let scrolled = wide.translate(vec2(0.0, -40.0));
    assert_eq!(glide.frame(outline(2, scrolled, 10.5, 5)), scrolled);
    assert!(!glide.is_gliding());
}

// `OutlineGlide::frame`: another `place`, or `animate` false, puts the outline on its cells.
#[test]
fn no_glide_between_places_or_with_animate_off() {
    let mut glide = OutlineGlide::with_motion(token(120));
    glide.frame(outline(1, cell(100.0, 100.0), 10.0, 1));
    let elsewhere = OutlineFrame { place: 2, ..outline(2, cell(300.0, 200.0), 10.1, 2) };
    assert_eq!(glide.frame(elsewhere), cell(300.0, 200.0));
    assert!(!glide.is_gliding());

    let still = OutlineFrame { animate: false, ..outline(3, cell(500.0, 300.0), 10.2, 3) };
    assert_eq!(glide.frame(still), cell(500.0, 300.0));
    assert!(!glide.is_gliding());
}

// `OutlineGlide::with_motion`: the glide takes the given time, and none at 0.
#[test]
fn the_outline_glide_takes_the_time_it_is_given() {
    let wide = Rect::from_min_max(pos2(100.0, 100.0), pos2(292.0, 116.0));
    let mut quick = OutlineGlide::with_motion(token(0));
    quick.frame(outline(1, cell(100.0, 100.0), 10.0, 1));
    assert_eq!(quick.frame(outline(2, wide, 10.1, 2)), wide);
    assert!(!quick.is_gliding());

    let mut slow = OutlineGlide::with_motion(token(1000));
    slow.frame(outline(1, cell(100.0, 100.0), 10.0, 1));
    slow.frame(outline(2, wide, 10.1, 2));
    assert!(slow.frame(outline(2, wide, 10.1 + 0.5, 3)) != wide);
    assert!(slow.is_gliding());
}
