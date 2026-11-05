//! Tests of the live preview (task AD-11).

use craft_ui::canvas::PixelRect;

use super::{DraftScale, LivePreview, PreviewKey, PreviewLane, PreviewSession, ProxyPlan, settings_hash};

const REGION: PixelRect = PixelRect::new(0, 0, 100, 100);
const VIEW: PixelRect = PixelRect::new(0, 0, 50, 50);

fn session(n: u64) -> PreviewSession {
    PreviewSession::new(n, 1, 7)
}

fn started(lane: PreviewLane, n: u64) -> LivePreview {
    let mut live = LivePreview::new(lane);
    live.begin(session(n), REGION);
    live
}

// One session: the same settings hash gives the same key twice; another hash another key; the
// key carries the lane's tag bit and never equals `PreviewKey::NONE`.
#[test]
fn live_preview_same_settings_give_the_same_key() {
    let mut live = started(PreviewLane::Dialog, 1);
    let a = live.key(settings_hash(&[b"levels", b"1"]), 0.0);
    let again = live.key(settings_hash(&[b"levels", b"1"]), 1.0);
    let other = live.key(settings_hash(&[b"levels", b"2"]), 2.0);
    assert_eq!(a, again);
    assert_ne!(a, other);
    assert!(!a.is_none());
    assert_eq!(a.raw() >> 59 & 1, 1);
    assert_eq!(PreviewKey::from_raw(a.raw()), a);
    assert!(live.owns(a) && live.owns(PreviewKey::NONE));
    let mut props = started(PreviewLane::Properties, 1);
    assert_eq!(props.key(5, 0.0).raw() >> 58 & 1, 1);
}

// A Dialog and a Properties preview with the same session and settings give different keys, and
// each treats the other's key as foreign (`owns` false, `switch` None).
#[test]
fn live_preview_keys_of_two_lanes_never_match() {
    let mut dialog = started(PreviewLane::Dialog, 1);
    let mut props = started(PreviewLane::Properties, 1);
    let kd = dialog.key(42, 0.0);
    let kp = props.key(42, 0.0);
    assert_ne!(kd, kp);
    assert!(!dialog.owns(kp) && !props.owns(kd));
    assert_eq!(dialog.switch(PreviewKey::NONE, kp, VIEW), None);
    assert_eq!(props.switch(kd, PreviewKey::NONE, VIEW), None);
}

// After a switch to a preview, `begin` with another session: the old key is foreign, `uncovered`
// returns None, and the new region applies.
#[test]
fn live_preview_a_new_session_forgets_the_covered_area() {
    let mut live = started(PreviewLane::Dialog, 1);
    let key = live.key(9, 0.0);
    assert!(live.switch(PreviewKey::NONE, key, VIEW).is_some());
    let region = PixelRect::new(10, 10, 20, 20);
    live.begin(session(2), region);
    assert!(!live.owns(key));
    assert_eq!(live.uncovered(key, REGION), None);
    assert_eq!(live.region(), region);
    assert!(!live.settling(0.1, 10.0));
}

// NONE to a preview and one preview to another: the region ∩ visible, remembered as covered.
#[test]
fn live_preview_switch_between_previews_returns_the_visible_part_of_the_region() {
    let mut live = started(PreviewLane::Dialog, 1);
    let (a, b) = (live.key(1, 0.0), live.key(2, 0.0));
    assert_eq!(live.switch(PreviewKey::NONE, a, VIEW), Some(VIEW));
    assert_eq!(live.switch(a, b, PixelRect::new(40, 40, 200, 200)), Some(PixelRect::new(40, 40, 100, 100)));
    // A target out of view gives an empty rect, not None.
    assert!(live.switch(b, a, PixelRect::new(500, 500, 600, 600)).is_some_and(|r| r.is_empty()));
}

// A preview back to NONE: the whole region, visible or not; nothing stays covered.
#[test]
fn live_preview_switch_back_to_none_returns_the_whole_region() {
    let mut live = started(PreviewLane::Dialog, 1);
    let key = live.key(1, 0.0);
    live.switch(PreviewKey::NONE, key, VIEW);
    assert_eq!(live.switch(key, PreviewKey::NONE, PixelRect::new(500, 500, 600, 600)), Some(REGION));
    assert_eq!(live.uncovered(key, REGION), None);
}

// A key of an earlier session or another lane on either side gives None; so do NONE to NONE and
// no session.
#[test]
fn live_preview_switch_with_a_foreign_key_returns_none() {
    let mut old = started(PreviewLane::Dialog, 1);
    let stale = old.key(1, 0.0);
    let mut live = started(PreviewLane::Dialog, 2);
    let key = live.key(1, 0.0);
    let mut props = started(PreviewLane::Properties, 2);
    let other_lane = props.key(1, 0.0);
    assert_eq!(live.switch(stale, key, VIEW), None);
    assert_eq!(live.switch(key, stale, VIEW), None);
    assert_eq!(live.switch(PreviewKey::NONE, other_lane, VIEW), None);
    assert_eq!(live.switch(PreviewKey::NONE, PreviewKey::NONE, VIEW), None);
    let mut none = LivePreview::new(PreviewLane::Dialog);
    assert_eq!(none.switch(PreviewKey::NONE, key, VIEW), None);
    assert_eq!(none.key(1, 0.0), PreviewKey::NONE);
}

// After a switch, a pan that shows more of the region gives that visible part once; the same
// view again gives None; a view inside the covered rect gives None; another key gives None.
#[test]
fn live_preview_uncovered_returns_the_newly_visible_part_once() {
    let mut live = started(PreviewLane::Dialog, 1);
    let (key, other) = (live.key(1, 0.0), live.key(2, 0.0));
    live.switch(PreviewKey::NONE, key, VIEW);
    assert_eq!(live.uncovered(key, VIEW), None);
    let panned = PixelRect::new(25, 0, 75, 50);
    assert_eq!(live.uncovered(key, panned), Some(panned));
    assert_eq!(live.uncovered(key, panned), None);
    assert_eq!(live.uncovered(key, PixelRect::new(30, 0, 60, 40)), None);
    assert_eq!(live.uncovered(other, PixelRect::new(0, 0, 100, 100)), None);
}

// A key change at t: settling(t + 0.1, 0.2) is true, settling(t + 0.25, 0.2) false; with the last
// key NONE it is false at once.
#[test]
fn live_preview_settling_ends_after_the_given_time() {
    let mut live = started(PreviewLane::Dialog, 1);
    live.key(1, 10.0);
    assert!(live.settling(10.1, 0.2));
    assert!(!live.settling(10.25, 0.2));
    live.key(1, 10.2);
    assert!(!live.settling(10.25, 0.2), "an equal key does not restart the timer");
    live.key(2, 11.0);
    assert!(live.settling(11.1, 0.2));
    let mut none = LivePreview::new(PreviewLane::Dialog);
    none.key(1, 11.0);
    assert!(!none.settling(11.0, 0.2));
}

// factor at zoom 1, 0.5, 0.2, 0.01 on a 6000 × 4000 document: 1, 2, 4, 64; 1 for a document of
// 4,000,000 pixels, with reduced previews off, and for zoom 0 or NaN. `scaled` rounds the far
// edges out.
#[test]
fn proxy_plan_factor_follows_zoom_and_is_one_for_small_documents() {
    let big = 6000 * 4000;
    let factors = [1.0, 0.5, 0.2, 0.01].map(|zoom| ProxyPlan::factor(zoom, big, true));
    assert_eq!(factors, [1, 2, 4, 64]);
    assert_eq!(ProxyPlan::factor(0.01, ProxyPlan::MIN_PIXELS, true), 1);
    assert_eq!(ProxyPlan::factor(0.01, big, false), 1);
    assert_eq!(ProxyPlan::factor(0.0, big, true), 1);
    assert_eq!(ProxyPlan::factor(f32::NAN, big, true), 1);
    assert_eq!(ProxyPlan::scaled(PixelRect::new(5, -3, 17, 9), 4), PixelRect::new(1, -1, 5, 3));
}

/// One drag of `control` on `item`: `drafts` full-size renders of `ms` each, started at `start`
/// and ended at `end`. Returns the scale the drag started with.
fn drag(d: &mut DraftScale, item: u64, start: f64, end: f64, ms: f64, drafts: u64) -> Option<f32> {
    let scale = d.scale(Some((item, "exposure")), true, Some(0), start);
    for key in 1..=drafts + 1 {
        d.observe(key, ms, None);
    }
    d.scale(None, true, None, end);
    scale
}

// A drag that starts at 1.0 keeps 1.0 on every pass while `gpu` flips and render times stay
// under 2 × budget; a drag of the CPU starts and stays at 0.6.
#[test]
fn draft_scale_keeps_its_scale_for_the_whole_drag() {
    let mut d = DraftScale::default();
    assert_eq!(d.scale(None, true, None, 0.0), None);
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 0.0), Some(1.0));
    for (i, gpu) in [false, true, false].into_iter().enumerate() {
        d.observe(i as u64 + 1, 25.0, None);
        assert_eq!(d.scale(Some((1, "exposure")), gpu, Some(0), 0.1), Some(1.0));
    }
    d.scale(None, true, None, 1.0);
    assert_eq!(d.scale(Some((2, "exposure")), false, Some(0), 2.0), Some(DraftScale::SMALL));
    d.observe(1, 1.0, None);
    assert_eq!(d.scale(Some((2, "exposure")), true, Some(0), 2.1), Some(DraftScale::SMALL));
}

// Full-size drafts with a median after the first above 32 ms (more than 3 samples) drop the
// running drag to 0.6.
#[test]
fn draft_scale_gives_up_mid_drag_far_over_budget() {
    let mut d = DraftScale::default();
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 0.0), Some(1.0));
    for key in 1..=5 {
        d.observe(key, 50.0, Some(0.0));
    }
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 1.0), Some(DraftScale::SMALL));
}

// Two drags of one pair over 20 ms: the first leaves the next at full size, the second makes the
// next 0.6 until 30 s later; another item's same control stays full size.
#[test]
fn draft_scale_two_slow_drags_in_a_row_make_the_next_small_until_the_retry_time() {
    let mut d = DraftScale::default();
    assert_eq!(drag(&mut d, 1, 0.0, 10.0, 25.0, 4), Some(1.0));
    assert_eq!(drag(&mut d, 1, 20.0, 100.0, 25.0, 4), Some(1.0));
    assert_eq!(d.scale(Some((2, "exposure")), true, Some(0), 110.0), Some(1.0));
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 110.0), Some(DraftScale::SMALL));
    d.scale(None, true, None, 111.0);
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 131.0), Some(1.0));
}

// After the retry time, a full-size drag within budget forgets the verdict (the next drag is full
// size at once); a slow retry doubles the wait to 60 s, capped at 240 s.
#[test]
fn draft_scale_a_fast_retry_forgets_the_verdict() {
    let slow_twice = |d: &mut DraftScale| {
        drag(d, 1, 0.0, 10.0, 25.0, 4);
        drag(d, 1, 20.0, 100.0, 25.0, 4);
    };
    let mut d = DraftScale::default();
    slow_twice(&mut d);
    assert_eq!(drag(&mut d, 1, 140.0, 150.0, 5.0, 4), Some(1.0));
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 151.0), Some(1.0));

    let mut d = DraftScale::default();
    slow_twice(&mut d);
    assert_eq!(drag(&mut d, 1, 140.0, 150.0, 25.0, 4), Some(1.0));
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 200.0), Some(DraftScale::SMALL));
    d.scale(None, true, None, 201.0);
    assert_eq!(d.scale(Some((1, "exposure")), true, Some(0), 211.0), Some(1.0));
}
