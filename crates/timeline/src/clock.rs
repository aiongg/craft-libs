//! The playback clock and the view that follows the playhead.
//!
//! [`PlaybackClock`] turns play, stop, speed, loops and stop points into the playhead time at
//! `now`. The app passes `now` (monotonic seconds) on every call: egui's `input().time` is the
//! previous full pass's value during a logic-only pass (EffectCraft `clock.rs:3`), and tests
//! drive a manual clock. An audio device that knows where it is drives the clock through
//! [`PlaybackClock::sync_external`].
//!
//! [`Follow`] scrolls a [`TimeView`] with the playhead while playing, unless something else moved
//! the view during playback.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::time::{Tick, TimeRange, TimeView};

/// How long playback waits for the first frames before its clock starts anyway, in seconds
/// (FilmCraft `frames.rs:442`). The frames it waits for are [`crate::frames::PREROLL_FRAMES`].
pub const PREROLL_TIMEOUT_S: f64 = 0.5;

/// Seconds an external clock may stand still while playing before the wall clock takes over (a
/// device that stopped consuming samples; FilmCraft `audio.rs:7` `AUDIO_STALL_S`).
pub const EXTERNAL_STALL_S: f64 = 1.0;

/// Where playback is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    /// Play was pressed; the clock waits for the first frames since `since` (seconds).
    Prerolling {
        since: f64,
    },
    Playing,
}

/// What a clock step crossed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockEvent {
    /// Preroll ended (frames ready or the timeout passed): the clock runs from this pass.
    Started,
    /// The playhead passed the loop's end and went back to its start.
    Looped,
    /// Forward playback reached the stop point and stopped there.
    Stopped(Tick),
}

/// The transport's clock: play at a speed (negative plays backwards; J/K/L shuttle speeds),
/// stop, preroll, loop over a range, stop at a point (Play In to Out), and an external clock.
///
/// ```ignore
/// // On Space:
/// clock.toggle(now, doc.playhead());
/// // Each pass:
/// let ready = frames.preroll_ready(&key, clock.speed(), last_frame);
/// for event in clock.step(now, ready) { /* Looped: request the loop start's frames */ }
/// doc.set_playhead(clock.position(now));
/// ```
#[derive(Clone, Debug, Default)]
pub struct PlaybackClock {
    state: PlaybackState,
    speed: f64,
    loop_range: Option<TimeRange>,
    stop_at: Option<Tick>,
    /// Monotonic time (s) and timeline time when the clock (re)started.
    anchor_time: f64,
    anchor: Tick,
    /// The external clock's last reading, and when (monotonic s) it last moved.
    external: Option<(Tick, f64)>,
}

impl PlaybackClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Playing or prerolling.
    pub fn is_playing(&self) -> bool {
        self.state != PlaybackState::Stopped
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Loop over `range` while playing (Premiere's Loop button with In and Out set; the whole
    /// sequence without them). `None` stops looping.
    pub fn set_loop(&mut self, range: Option<TimeRange>) {
        self.loop_range = range;
    }

    /// Forward playback stops at `at` (Play In to Out, Play from Playhead to Out Point).
    pub fn set_stop_at(&mut self, at: Option<Tick>) {
        self.stop_at = at;
    }

    /// Start playing from `from` at `speed` (1.0 normal). With `preroll` the state is
    /// [`PlaybackState::Prerolling`] until [`step`](Self::step) sees the frames ready or
    /// [`PREROLL_TIMEOUT_S`] pass; without it the clock runs at once. Playing again while playing
    /// changes the speed and re-anchors at the current position (shuttle).
    // filmcraft: crates/ui-egui/src/app/playback.rs:18 `play` (anchor time and tick, preroll).
    pub fn play(&mut self, now: f64, from: Tick, speed: f64, preroll: bool) {
        todo!()
    }

    /// Stop and return where the playhead stopped.
    // filmcraft: crates/ui-egui/src/app/playback.rs:97 `stop`.
    pub fn stop(&mut self, now: f64) -> Tick {
        todo!()
    }

    /// Space: stop when playing (returns the position), else play at 1.0 from `from` with
    /// preroll.
    // filmcraft: crates/ui-egui/src/app/playback.rs:10 `toggle_play`.
    pub fn toggle(&mut self, now: f64, from: Tick) -> Option<Tick> {
        todo!()
    }

    /// Move the playhead to `t` while playing (a click in the ruler): re-anchors there; the
    /// state is unchanged.
    pub fn seek(&mut self, now: f64, t: Tick) {
        todo!()
    }

    /// The playhead at `now`: the anchor while stopped or prerolling; while playing the anchor
    /// plus `(now − anchor_time) × speed`, or the external clock's reading while it moves, folded
    /// into the loop range and held at the stop point.
    pub fn position(&self, now: f64) -> Tick {
        todo!()
    }

    /// Advance the state machine for this pass. `frames_ready` is whether the frames preroll
    /// waits for are there ([`FrameServer::preroll_ready`](crate::frames::FrameServer::preroll_ready)).
    /// Returns what happened this pass, in order (a long pass can loop and then stop).
    pub fn step(&mut self, now: f64, frames_ready: bool) -> Vec<ClockEvent> {
        todo!()
    }

    /// An external clock (the audio device's played-sample count, as a time) read at `now`. While
    /// it keeps moving it is the playhead; when it stands still for [`EXTERNAL_STALL_S`] while
    /// playing, the wall clock takes over from its last reading (FilmCraft's audio clock,
    /// `lib.rs:141`).
    pub fn sync_external(&mut self, position: Tick, now: f64) {
        todo!()
    }
}

/// How the view follows the playhead while playing. SoundCraft's five Scrolling modes; FilmCraft's
/// Timeline Playback Auto-Scrolling is `None` (No Scroll), `Page` (Page Scroll) and `Center`
/// (Smooth Scroll).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FollowMode {
    /// The view stays where it is.
    None,
    /// The view stays while playing and jumps to the playhead when playback stops.
    AfterPlayback,
    /// When the playhead passes 95 % of the width (or is left of the view) the view moves so it
    /// is 5 % from the left.
    #[default]
    Page,
    /// The playhead stays in the middle from the first frame; the content moves under it.
    Continuous,
    /// The playhead moves to the middle, then stays there while the content moves.
    Center,
}

/// The follow state of one view: whether something other than the follow moved the view during
/// playback (a pan, the scroll bar, the overview), which holds the follow until the playhead is
/// back in view or playback stops or starts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Follow {
    hold: bool,
    /// The view start seen last pass and the start the follow last asked for.
    last_start: Option<Tick>,
    last_target: Option<Tick>,
}

impl Follow {
    /// Forget the hold: on play, on stop, and when the user picks a scrolling mode again.
    pub fn reset(&mut self) {
        *self = Follow::default();
    }

    /// Whether the follow is held.
    pub fn is_held(&self) -> bool {
        self.hold
    }

    /// The new view start for this pass, if the view should move. `playing` is the clock's; a
    /// view whose start differs from both the start seen last pass and the follow's own last
    /// target was moved by something else, so the follow holds and returns `None`. A playhead
    /// back in view clears the hold. While stopped, [`FollowMode::AfterPlayback`] reveals the
    /// playhead once (on the pass playback stopped).
    // soundcraft: crates/ui-egui/src/edit_window/scroll.rs:121 `follow_update` (the hold);
    // filmcraft: crates/ui-egui/src/panels/timeline.rs:513 (the 95 % / 5 % page rule and the
    // centred smooth scroll).
    pub fn update(&mut self, mode: FollowMode, playing: bool, view: &TimeView, width: f32, playhead: Tick) -> Option<Tick> {
        todo!()
    }
}
