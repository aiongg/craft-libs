//! Clip edges, trims and snapping: which edge a press grabs, which trim it starts, and the edit
//! a drag reports. The edit rules (ripple, linked partners, overwrite) stay the app's: the library
//! reports the gesture, the app applies it.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::model::{ClipId, TrackId};
use crate::time::{Tick, TimeRange, TimeView};

/// The widest an edge zone gets, in points, on each side of a clip edge; at most a third of the
/// clip's width (FilmCraft `timeline_hit.rs:21`).
pub const EDGE_PT: f32 = 7.0;

/// Within this distance of a cut with a clip on its other side, the Selection tool rolls instead
/// of rippling when its no-modifier trims are on (FilmCraft `timeline_hit.rs:18`).
pub const ROLL_PT: f32 = 2.5;

/// How near a snap target must be, in points (FilmCraft `timeline.rs:83`; EffectCraft snaps the
/// work area at 8).
pub const SNAP_PT: f32 = 9.0;

/// A clip's start (In) or end (Out).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClipEdge {
    In,
    Out,
}

/// What a drag at a clip edge or body does (Premiere's trim tools).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrimKind {
    /// The edge moves, opening or covering a gap.
    Trim,
    /// The edge moves and later clips follow.
    Ripple,
    /// The cut between two touching clips moves.
    Roll,
    /// The clip's media moves inside it; its place stays.
    Slip,
    /// The clip moves; its neighbours' touching edges follow.
    Slide,
    /// The clip's speed changes to fill its new length (Rate Stretch).
    Stretch,
}

/// The clip edge a pointer grabs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeHit {
    pub clip: ClipId,
    pub edge: ClipEdge,
    /// The pointer is inside the clip (not in the zone past its edge).
    pub inside: bool,
    /// Points from the edge.
    pub distance: f32,
}

/// The edge zone of a clip `width` points wide: [`EDGE_PT`], at most a third of the width;
/// `None` for a clip without a finite positive width.
// filmcraft: crates/ui-egui/src/panels/timeline_hit.rs:195 `zone`.
pub fn edge_zone(width: f32) -> Option<f32> {
    todo!()
}

/// The edge of `clips` (one track's, in any order) a pointer at `x` grabs. Of several in reach,
/// the one the pointer is inside of beats one it is outside of, then the nearer (FilmCraft
/// #259: two touching clips share the cut, and the press goes to the clip it is over).
// filmcraft: crates/ui-egui/src/panels/timeline_hit.rs:200 `edge_at`, `:167` `Reach::beats`.
pub fn edge_at(clips: &[(ClipId, TimeRange)], view: &TimeView, left: f32, x: f32) -> Option<EdgeHit> {
    todo!()
}

/// The trim the Selection tool starts at an edge: Cmd+Shift rolls, Cmd ripples, else a regular
/// trim. With `no_modifier` (Premiere's Trim setting "Allow Selection tool to choose Roll and
/// Ripple trims without modifier key") no modifier is needed: on the cut (within [`ROLL_PT`],
/// a clip on the other side) it rolls, elsewhere on the edge it ripples.
// filmcraft: crates/ui-egui/src/panels/timeline_hit.rs:26 `selection_trim_kind`.
pub fn selection_trim(command: bool, shift: bool, no_modifier: bool, distance: f32, has_neighbour: bool) -> TrimKind {
    todo!()
}

/// An edit a clip drag reports, as the value of an `EditEvent` gesture: the app previews each
/// update and applies the commit (one undo step).
#[derive(Clone, Debug, PartialEq)]
pub enum ClipEdit {
    /// Clips moved by `delta` and `track_delta` tracks. `new_track`: dropped in the empty space
    /// past the last track of their kind (FilmCraft #1104), the app adds a track there. `copy`:
    /// Alt was held on the last drag frame, the drop copies.
    Move { clips: Vec<ClipId>, delta: Tick, track_delta: i32, new_track: bool, copy: bool },
    /// An edge moved by `delta` with `kind` ([`TrimKind::Trim`], `Ripple` or `Stretch`).
    /// `linked`: linked partners trim along (false when Alt was held as the drag started).
    Trim { clip: ClipId, edge: ClipEdge, kind: TrimKind, delta: Tick, linked: bool },
    /// The cut between `left` and `right` moved by `delta`.
    Roll { left: ClipId, right: ClipId, delta: Tick },
    /// The clip's media moved by `delta` inside it.
    Slip { clip: ClipId, delta: Tick },
    /// The clip moved by `delta`, its neighbours' edges with it.
    Slide { clip: ClipId, delta: Tick },
    /// The Razor tool cut `clip` at `at` (snapped to a frame).
    Cut { clip: ClipId, at: Tick },
    /// Box selection: the clips the marquee touches; `add` keeps the selection from before
    /// (Shift or Cmd).
    Select { clips: Vec<ClipId>, add: bool },
}

/// The clips a marquee from `a` to `b` (screen points) touches, given each visible clip's track
/// row and time range.
// filmcraft: crates/ui-egui/src/panels/timeline/interact.rs:114 `marquee_clips`.
pub fn marquee_clips(
    clips: &[(ClipId, TrackId, TimeRange)],
    rows: &[(TrackId, f32, f32)],
    view: &TimeView,
    left: f32,
    a: [f32; 2],
    b: [f32; 2],
) -> Vec<ClipId> {
    todo!()
}

/// Times a drag snaps to: clip starts and ends (but the dragged clips'), the playhead (but not
/// for the playhead's own drag), markers, In and Out, work-area ends, keyframes. The app fills
/// it per gesture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapTargets {
    times: Vec<Tick>,
}

impl SnapTargets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, t: Tick) {
        self.times.push(t);
    }

    /// Both ends of each range.
    pub fn ranges(&mut self, ranges: impl IntoIterator<Item = TimeRange>) {
        for r in ranges {
            self.times.push(r.start);
            self.times.push(r.end());
        }
    }

    pub fn times(&self) -> &[Tick] {
        &self.times
    }
}

impl Extend<Tick> for SnapTargets {
    fn extend<I: IntoIterator<Item = Tick>>(&mut self, iter: I) {
        self.times.extend(iter);
    }
}

/// `t` snapped to the nearest target within `tolerance` points at this zoom, and the target it
/// snapped to (the timeline draws the snap line there); `t` and `None` when none is in reach.
// filmcraft: crates/ui-egui/src/panels/timeline/snap.rs:15 `snap_to` (distance in screen
// points, nearest wins).
pub fn snap_time(targets: &SnapTargets, t: Tick, view: &TimeView, tolerance: f32) -> (Tick, Option<Tick>) {
    todo!()
}
