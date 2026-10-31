//! The playback clock and the follow.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// position advances by (now - start) * speed, backwards at negative speed.
#[test]
#[ignore = "skeleton"]
fn plays_at_speed_from_the_anchor() {
    todo!()
}

// Prerolling until frames_ready or 0.5 s, then Started.
#[test]
#[ignore = "skeleton"]
fn preroll_waits_for_frames_or_the_timeout() {
    todo!()
}

// Looped at the loop end, Stopped at stop_at.
#[test]
#[ignore = "skeleton"]
fn loops_and_stops_at_the_stop_point() {
    todo!()
}

// sync_external drives the position until it stands still for a second.
#[test]
#[ignore = "skeleton"]
fn external_clock_leads_until_it_stalls() {
    todo!()
}

// Page moves the view to 5 % when the playhead passes 95 %.
#[test]
#[ignore = "skeleton"]
fn page_follow_jumps_at_ninety_five_percent() {
    todo!()
}

// Center centres once there, Continuous from the first frame.
#[test]
#[ignore = "skeleton"]
fn center_and_continuous_follow() {
    todo!()
}

// a view moved by something else holds until the playhead is back in view or playback restarts.
#[test]
#[ignore = "skeleton"]
fn a_manual_scroll_holds_the_follow() {
    todo!()
}
