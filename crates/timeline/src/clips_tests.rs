//! Clip edges, trims and snapping.

use crate::clips::{ClipEdge, EdgeZones, SnapTargets, TrimKind, edge_at, edge_zone, marquee_clips, selection_trim, snap_time};
use crate::model::{ClipId, TrackId};
use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView};

const ZONES: EdgeZones = EdgeZones { edge: 7.0, roll: 2.5 };

fn seconds(s: i64) -> Tick {
    Tick(s * TICKS_PER_SECOND)
}

fn range(from: i64, to: i64) -> TimeRange {
    TimeRange::from_bounds(seconds(from), seconds(to))
}

/// 10 points a second from zero.
fn view() -> TimeView {
    TimeView::new(Tick::ZERO, 10.0)
}

// edge_zone is the edge size, at most a third of the width, None for zero width.
#[test]
fn edge_zone_is_a_third_of_narrow_clips() {
    assert_eq!(edge_zone(100.0, &ZONES), Some(7.0));
    assert_eq!(edge_zone(12.0, &ZONES), Some(4.0));
    assert_eq!(edge_zone(0.0, &ZONES), None);
    assert_eq!(edge_zone(f32::NAN, &ZONES), None);
}

// edge_at prefers the edge whose clip the pointer is inside.
#[test]
fn the_clip_under_the_pointer_wins_a_shared_cut() {
    let clips = [(ClipId(2), range(5, 10)), (ClipId(1), range(0, 5))];
    // the cut is at x = 50
    let left_side = edge_at(&clips, &view(), 0.0, 48.0, &ZONES).expect("edge");
    assert_eq!((left_side.clip, left_side.edge, left_side.inside), (ClipId(1), ClipEdge::Out, true));
    let right_side = edge_at(&clips, &view(), 0.0, 52.0, &ZONES).expect("edge");
    assert_eq!((right_side.clip, right_side.edge, right_side.inside), (ClipId(2), ClipEdge::In, true));
    let on_the_cut = edge_at(&clips, &view(), 0.0, 50.0, &ZONES).expect("edge");
    assert_eq!((on_the_cut.clip, on_the_cut.edge), (ClipId(2), ClipEdge::In));
    assert_eq!(edge_at(&clips, &view(), 0.0, 25.0, &ZONES), None);
}

// Cmd+Shift roll, Cmd ripple, no-modifier roll on the cut and ripple elsewhere.
#[test]
fn selection_trim_follows_modifiers_and_the_setting() {
    assert_eq!(selection_trim(true, true, false, 5.0, false, &ZONES), TrimKind::Roll);
    assert_eq!(selection_trim(true, false, false, 5.0, true, &ZONES), TrimKind::Ripple);
    assert_eq!(selection_trim(false, false, false, 0.0, true, &ZONES), TrimKind::Trim);
    assert_eq!(selection_trim(false, false, true, 2.0, true, &ZONES), TrimKind::Roll);
    assert_eq!(selection_trim(false, false, true, 2.0, false, &ZONES), TrimKind::Ripple);
    assert_eq!(selection_trim(false, false, true, 4.0, true, &ZONES), TrimKind::Ripple);
}

// snap_time snaps within the tolerance and reports the target.
#[test]
fn snap_time_takes_the_nearest_target_in_reach() {
    let mut targets = SnapTargets::new();
    targets.ranges([range(2, 4)]);
    targets.push(seconds(4) + Tick(TICKS_PER_SECOND / 10));
    // targets at 2 s and 4 s (20 and 40 points): 0.5 s is 5 points from the first
    assert_eq!(snap_time(&targets, seconds(2) + Tick(TICKS_PER_SECOND / 2), &view(), 9.0), (seconds(2), Some(seconds(2))));
    let far = seconds(3);
    assert_eq!(snap_time(&targets, far, &view(), 9.0), (far, None));
    // the nearer of two in reach wins
    let near_four = seconds(4) + Tick(TICKS_PER_SECOND / 25);
    assert_eq!(snap_time(&targets, near_four, &view(), 9.0), (seconds(4), Some(seconds(4))));
}

// marquee_clips returns the clips in the rectangle.
#[test]
fn marquee_selects_the_clips_it_touches() {
    let clips = [(ClipId(1), TrackId(1), range(0, 5)), (ClipId(2), TrackId(1), range(5, 10)), (ClipId(3), TrackId(2), range(0, 10))];
    let rows = [(TrackId(1), 0.0, 20.0), (TrackId(2), 20.0, 40.0)];
    // x 40..60 is 4 s to 6 s, across the cut; y stays in the first row
    assert_eq!(marquee_clips(&clips, &rows, &view(), 0.0, [60.0, 5.0], [40.0, 10.0]), vec![ClipId(1), ClipId(2)]);
    // down into the second row
    assert_eq!(marquee_clips(&clips, &rows, &view(), 0.0, [10.0, 10.0], [20.0, 30.0]), vec![ClipId(1), ClipId(3)]);
    // a rectangle inside the second clip selects only it
    assert_eq!(marquee_clips(&clips, &rows, &view(), 0.0, [50.0, 5.0], [60.0, 6.0]), vec![ClipId(2)]);
}
