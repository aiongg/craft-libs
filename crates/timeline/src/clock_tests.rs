//! The playback clock and the follow.

use super::clock::{ClockEvent, EXTERNAL_STALL_S, Follow, FollowMode, PREROLL_TIMEOUT_S, PlaybackClock, PlaybackState};
use super::time::{Tick, TimeRange, TimeView};

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn playing(from: f64, speed: f64) -> PlaybackClock {
    let mut clock = PlaybackClock::new();
    clock.play(0.0, secs(from), speed, false);
    clock
}

// position advances by (now - start) * speed, backwards at negative speed.
#[test]
fn plays_at_speed_from_the_anchor() {
    let clock = playing(10.0, 1.0);
    assert_eq!(clock.position(2.0), secs(12.0));
    let clock = playing(10.0, 2.0);
    assert_eq!(clock.position(2.0), secs(14.0));
    let clock = playing(10.0, -1.0);
    assert_eq!(clock.position(2.0), secs(8.0));
    assert_eq!(clock.position(20.0), Tick::ZERO, "backwards playback holds at the start");
    let mut clock = playing(10.0, 1.0);
    assert_eq!(clock.stop(3.0), secs(13.0));
    assert_eq!(clock.position(9.0), secs(13.0), "a stopped clock stands still");
}

// Prerolling until frames_ready or 0.5 s, then Started.
#[test]
fn preroll_waits_for_frames_or_the_timeout() {
    let mut clock = PlaybackClock::new();
    assert_eq!(clock.toggle(1.0, secs(5.0)), None);
    assert_eq!(clock.state(), PlaybackState::Prerolling { since: 1.0 });
    assert!(clock.step(1.2, false).is_empty());
    assert_eq!(clock.position(1.4), secs(5.0), "the playhead waits");
    assert_eq!(clock.step(1.3, true), [ClockEvent::Started]);
    assert_eq!(clock.position(2.3), secs(6.0), "the clock runs from the pass that started it");

    let mut clock = PlaybackClock::new();
    clock.play(0.0, Tick::ZERO, 1.0, true);
    assert!(clock.step(PREROLL_TIMEOUT_S - 0.01, false).is_empty());
    assert_eq!(clock.step(PREROLL_TIMEOUT_S, false), [ClockEvent::Started]);
    assert_eq!(clock.toggle(1.0, Tick::ZERO), Some(secs(0.5)));
}

// Looped at the loop end, Stopped at stop_at.
#[test]
fn loops_and_stops_at_the_stop_point() {
    let mut clock = playing(1.0, 1.0);
    clock.set_loop(Some(TimeRange::from_bounds(secs(1.0), secs(3.0))));
    assert_eq!(clock.position(2.5), secs(1.5), "folded into the loop before the pass sees it");
    assert_eq!(clock.step(2.5, true), [ClockEvent::Looped]);
    assert!(clock.step(3.0, true).is_empty());
    assert_eq!(clock.position(3.0), secs(2.0));

    let mut clock = playing(0.0, 1.0);
    clock.set_stop_at(Some(secs(2.0)));
    assert!(clock.step(1.0, true).is_empty());
    assert_eq!(clock.position(5.0), secs(2.0));
    assert_eq!(clock.step(5.0, true), [ClockEvent::Stopped(secs(2.0))]);
    assert_eq!(clock.state(), PlaybackState::Stopped);
    assert_eq!(clock.position(9.0), secs(2.0));
}

// sync_external drives the position until it stands still for a second.
#[test]
fn external_clock_leads_until_it_stalls() {
    let mut clock = playing(0.0, 1.0);
    clock.sync_external(secs(0.9), 1.0);
    assert_eq!(clock.position(1.0), secs(0.9), "the device is ahead of the wall clock");
    clock.sync_external(secs(1.9), 2.0);
    clock.sync_external(secs(1.9), 2.5);
    assert_eq!(clock.position(2.5), secs(1.9));
    let stalled = 2.0 + EXTERNAL_STALL_S;
    assert_eq!(clock.position(stalled), secs(1.9));
    assert_eq!(clock.position(stalled + 0.5), secs(2.4), "the wall clock continues from the last reading");
    clock.sync_external(secs(2.6), stalled + 0.6);
    assert_eq!(clock.position(stalled + 0.6), secs(2.6), "a device that moves again leads again");
}

fn view(start: f64) -> TimeView {
    TimeView::new(secs(start), 100.0)
}

// Page moves the view to 5 % when the playhead passes 95 %.
#[test]
fn page_follow_jumps_at_ninety_five_percent() {
    let mut follow = Follow::default();
    let width = 1000.0;
    assert_eq!(follow.update(FollowMode::Page, true, &view(0.0), width, secs(9.4)), None);
    assert_eq!(follow.update(FollowMode::Page, true, &view(0.0), width, secs(9.6)), Some(secs(9.1)));
    assert_eq!(follow.update(FollowMode::None, true, &view(0.0), width, secs(12.0)), None);
    assert_eq!(follow.update(FollowMode::AfterPlayback, true, &view(0.0), width, secs(12.0)), None);
    assert_eq!(follow.update(FollowMode::AfterPlayback, false, &view(0.0), width, secs(12.0)), Some(secs(11.5)), "revealed once, as playback stops");
    assert_eq!(follow.update(FollowMode::AfterPlayback, false, &view(0.0), width, secs(12.0)), None);
}

// Center centres once there, Continuous from the first frame.
#[test]
fn center_and_continuous_follow() {
    let width = 1000.0;
    let mut center = Follow::default();
    assert_eq!(center.update(FollowMode::Center, true, &view(10.0), width, secs(12.0)), None, "left of the middle: stays");
    assert_eq!(center.update(FollowMode::Center, true, &view(10.0), width, secs(15.5)), Some(secs(10.5)));
    let mut continuous = Follow::default();
    assert_eq!(continuous.update(FollowMode::Continuous, true, &view(10.0), width, secs(12.0)), Some(secs(7.0)));
}

// a view moved by something else holds until the playhead is back in view or playback restarts.
#[test]
fn a_manual_scroll_holds_the_follow() {
    let width = 1000.0;
    let mut follow = Follow::default();
    assert_eq!(follow.update(FollowMode::Page, true, &view(0.0), width, secs(1.0)), None);
    // The user scrolls away; the playhead is off the left edge.
    assert_eq!(follow.update(FollowMode::Page, true, &view(30.0), width, secs(1.0)), None);
    assert!(follow.is_held());
    assert_eq!(follow.update(FollowMode::Page, true, &view(30.0), width, secs(1.5)), None, "the view stays where the user left it");
    // Back in view: the hold ends and the follow works again.
    assert_eq!(follow.update(FollowMode::Page, true, &view(40.0), width, secs(45.0)), None);
    assert!(!follow.is_held());
    assert_eq!(follow.update(FollowMode::Page, true, &view(40.0), width, secs(49.8)), Some(secs(49.3)));
    // A pan held again, then play restarts.
    assert_eq!(follow.update(FollowMode::Page, true, &view(0.0), width, secs(49.8)), None);
    assert!(follow.is_held());
    follow.reset();
    assert_eq!(follow.update(FollowMode::Page, true, &view(0.0), width, secs(49.8)), Some(secs(49.3)));
}
