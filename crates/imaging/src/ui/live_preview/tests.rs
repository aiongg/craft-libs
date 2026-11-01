//! Tests of the live preview (task AD-11).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// One session: the same settings hash gives the same key twice; another hash another key; the
// key carries the lane's tag bit and never equals `PreviewKey::NONE`.
#[test]
#[ignore = "skeleton"]
fn live_preview_same_settings_give_the_same_key() {
    todo!()
}

// A Dialog and a Properties preview with the same session and settings give different keys, and
// each treats the other's key as foreign (`owns` false, `switch` None).
#[test]
#[ignore = "skeleton"]
fn live_preview_keys_of_two_lanes_never_match() {
    todo!()
}

// After a switch to a preview, `begin` with another session: the old key is foreign, `uncovered`
// returns None, and the new region applies.
#[test]
#[ignore = "skeleton"]
fn live_preview_a_new_session_forgets_the_covered_area() {
    todo!()
}

// NONE to a preview and one preview to another: the region ∩ visible, remembered as covered.
#[test]
#[ignore = "skeleton"]
fn live_preview_switch_between_previews_returns_the_visible_part_of_the_region() {
    todo!()
}

// A preview back to NONE: the whole region, visible or not; nothing stays covered.
#[test]
#[ignore = "skeleton"]
fn live_preview_switch_back_to_none_returns_the_whole_region() {
    todo!()
}

// A key of an earlier session or another lane on either side gives None; so do NONE to NONE and
// no session.
#[test]
#[ignore = "skeleton"]
fn live_preview_switch_with_a_foreign_key_returns_none() {
    todo!()
}

// After a switch, a pan that shows more of the region gives that visible part once; the same
// view again gives None; a view inside the covered rect gives None; another key gives None.
#[test]
#[ignore = "skeleton"]
fn live_preview_uncovered_returns_the_newly_visible_part_once() {
    todo!()
}

// A key change at t: settling(t + 0.1, 0.2) is true, settling(t + 0.25, 0.2) false; with the last
// key NONE it is false at once.
#[test]
#[ignore = "skeleton"]
fn live_preview_settling_ends_after_the_given_time() {
    todo!()
}

// factor at zoom 1, 0.5, 0.2, 0.01 on a 6000 × 4000 document: 1, 2, 4, 64; 1 for a document of
// 4,000,000 pixels, with reduced previews off, and for zoom 0 or NaN. `scaled` rounds the far
// edges out.
#[test]
#[ignore = "skeleton"]
fn proxy_plan_factor_follows_zoom_and_is_one_for_small_documents() {
    todo!()
}

// A drag that starts at 1.0 keeps 1.0 on every pass while `gpu` flips and render times stay
// under 2 × budget; a drag of the CPU starts and stays at 0.6.
#[test]
#[ignore = "skeleton"]
fn draft_scale_keeps_its_scale_for_the_whole_drag() {
    todo!()
}

// Full-size drafts with a median after the first above 32 ms (more than 3 samples) drop the
// running drag to 0.6.
#[test]
#[ignore = "skeleton"]
fn draft_scale_gives_up_mid_drag_far_over_budget() {
    todo!()
}

// Two drags of one pair over 20 ms: the first leaves the next at full size, the second makes the
// next 0.6 until 30 s later; another item's same control stays full size.
#[test]
#[ignore = "skeleton"]
fn draft_scale_two_slow_drags_in_a_row_make_the_next_small_until_the_retry_time() {
    todo!()
}

// After the retry time, a full-size drag within budget forgets the verdict (the next drag is full
// size at once); a slow retry doubles the wait to 60 s, capped at 240 s.
#[test]
#[ignore = "skeleton"]
fn draft_scale_a_fast_retry_forgets_the_verdict() {
    todo!()
}
