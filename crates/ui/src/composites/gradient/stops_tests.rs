//! The stop rules and the default sampler: pure functions of the stops.

use craft_theme::Srgba;

use super::stops::{self, BarOp, StopKey, StopRules, default_sampler, sample};
use super::{ColorStop, GradientStops, OpacityStop, StopRef, StopRow, StopRows};

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn red_to_blue() -> GradientStops {
    GradientStops::new(vec![ColorStop { tag: 7, ..ColorStop::new(0.0, RED) }, ColorStop { tag: 9, ..ColorStop::new(1.0, BLUE) }])
}

fn close(a: Srgba, b: Srgba) -> bool {
    [a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a].iter().all(|d| d.abs() < 1e-5)
}

#[test]
fn a_new_stop_takes_the_colour_there_and_resets_the_segment_it_splits() {
    let rules = StopRules::default();
    let mut stops = red_to_blue();
    stops.color[0].midpoint = 0.2;
    let sampler = default_sampler(StopRows::Split);
    let (out, at) = stops::add(&stops, StopRow::Color, 0.5, &sampler, &rules).unwrap();
    assert_eq!(at, 1);
    assert!(close(out.color[1].color, sample(&stops, StopRows::Split, 0.5)));
    assert_eq!((out.color[0].midpoint, out.color[1].midpoint, out.color[1].tag), (0.5, 0.5, 0));
    assert!(out.opacity.is_empty(), "a colour edit leaves the implicit opacity alone");

    // The first opacity edit writes out the two stops an empty list stands for.
    let (out, at) = stops::add(&stops, StopRow::Opacity, 0.3, &sampler, &rules).unwrap();
    assert_eq!((at, out.opacity.len()), (1, 3));
    assert_eq!(out.opacity[1].opacity, 1.0);

    let max = StopRules { max: 2, ..StopRules::default() };
    assert!(stops::add(&stops, StopRow::Color, 0.5, &sampler, &max).is_none());
    let combined = StopRules { rows: StopRows::Combined, ..StopRules::default() };
    assert!(stops::add(&stops, StopRow::Opacity, 0.5, &sampler, &combined).is_none());
}

#[test]
fn a_moved_stop_passes_its_neighbours_and_keeps_its_tag_and_the_selection_index() {
    let rules = StopRules::default();
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop { tag: 8, ..ColorStop::new(0.5, Srgba::BLACK) });
    let (out, at) = stops::move_to(&stops, StopRow::Color, 0, 0.75, &rules).unwrap();
    assert_eq!(at, 1);
    assert_eq!(out.color.iter().map(|s| s.tag).collect::<Vec<_>>(), [8, 7, 9]);

    // Locations clamp and keep their decimals; the bar rounds pointer locations itself.
    let (out, at) = stops::move_to(&stops, StopRow::Color, 2, 0.33333, &rules).unwrap();
    assert_eq!((at, out.color[at].location, out.color[at].tag), (1, 0.33333, 9));
    assert_eq!(stops::move_to(&stops, StopRow::Color, 2, 1.5, &rules).unwrap().0.color[2].location, 1.0);
}

#[test]
fn a_stop_keeps_its_place_among_stops_at_its_location_unless_it_crossed_them() {
    let rules = StopRules::default();
    let tags = |s: &GradientStops| s.color.iter().map(|s| s.tag).collect::<Vec<_>>();
    // A hard edge at 1.0: a nudge past the end clamps and changes nothing.
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop { tag: 8, ..ColorStop::new(1.0, Srgba::BLACK) });
    let (out, at) = stops::move_to(&stops, StopRow::Color, 1, 1.01, &rules).unwrap();
    assert_eq!((out, at), (stops.clone(), 1));

    // Reaching a stop's location from below it stays below, from above it stays above.
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop { tag: 8, ..ColorStop::new(0.5, Srgba::BLACK) });
    let (out, at) = stops::move_to(&stops, StopRow::Color, 0, 0.5, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![7, 8, 9], 0));
    let (out, at) = stops::move_to(&stops, StopRow::Color, 2, 0.5, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![7, 8, 9], 2));
    // Moving past the tie crosses it.
    let tied = stops::move_to(&stops, StopRow::Color, 0, 0.5, &rules).unwrap().0;
    let (out, at) = stops::move_to(&tied, StopRow::Color, 0, 0.6, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![8, 7, 9], 1));
}

#[test]
fn remove_stops_at_the_minimum_and_selects_the_stop_that_takes_the_index() {
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop::new(0.5, Srgba::BLACK));
    let rules = StopRules::default();
    assert_eq!(stops::remove(&stops, StopRow::Color, 1, &rules).unwrap().1, 1);
    assert_eq!(stops::remove(&stops, StopRow::Color, 2, &rules).unwrap().1, 1, "the last stop selects the new last");
    let two = stops::remove(&stops, StopRow::Color, 1, &rules).unwrap().0;
    assert!(stops::remove(&two, StopRow::Color, 0, &rules).is_none());
    // The implicit opacity stops count: two at the minimum of two, removable at one.
    assert!(stops::remove(&two, StopRow::Opacity, 0, &rules).is_none());
    let one = StopRules { min: 1, ..rules };
    assert_eq!(stops::remove(&two, StopRow::Opacity, 0, &one).unwrap().0.opacity.len(), 1);
}

#[test]
fn a_copy_keeps_the_source_tag_and_midpoint_and_midpoints_clamp() {
    let rules = StopRules::default();
    let mut stops = red_to_blue();
    stops.color[1].midpoint = 0.3;
    let (out, at) = stops::copy(&stops, StopRow::Color, 1, 0.5, &rules).unwrap();
    assert_eq!((at, out.color[1].tag, out.color[1].midpoint, out.color[1].color), (1, 9, 0.3, BLUE));
    assert_eq!(out.color[0].midpoint, 0.5);

    let (out, at) = stops::set_midpoint(&stops, StopRow::Color, 0, 0.0, &rules).unwrap();
    assert_eq!((at, out.color[0].midpoint), (0, 0.05));
    assert!(stops::set_midpoint(&stops, StopRow::Color, 1, 0.5, &rules).is_none(), "the last stop has no midpoint");
}

#[test]
fn halfway_goes_between_the_selection_and_its_neighbour_or_across_the_widest_gap() {
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop::new(0.25, Srgba::BLACK));
    let color = |sel| stops::halfway(&stops, StopRow::Color, sel);
    assert_eq!(color(Some(0)), 0.125);
    assert_eq!(color(Some(1)), 0.625);
    assert_eq!(color(Some(2)), 0.625, "the last stop looks back");
    assert_eq!(color(None), 0.625, "the widest gap is 0.25 to 1");
    assert_eq!(stops::halfway(&stops, StopRow::Opacity, None), 0.5, "the implicit 100 % stops at 0 and 1");
}

#[test]
fn the_default_sampler_bends_each_segment_to_its_midpoint_and_splits_alpha_by_layout() {
    let mut stops = red_to_blue();
    stops.color[0].midpoint = 0.25;
    let half = sample(&stops, StopRows::Split, 0.25);
    assert!(close(half, Srgba::rgb(0.5, 0.0, 0.5)), "{half:?}");
    assert!(close(sample(&stops, StopRows::Split, -1.0), RED) && close(sample(&stops, StopRows::Split, 2.0), BLUE));

    // Split: alpha from the opacity stops, the colours' own ignored.
    stops.color[0].color.a = 0.0;
    stops.opacity = vec![OpacityStop::new(0.0, 0.0), OpacityStop::new(1.0, 1.0)];
    assert!((sample(&stops, StopRows::Split, 0.5).a - 0.5).abs() < 1e-5);
    stops.opacity.clear();
    assert_eq!(sample(&stops, StopRows::Split, 0.0).a, 1.0);
    // Combined: the colours' alpha.
    assert_eq!(sample(&stops, StopRows::Combined, 0.0).a, 0.0);
}

/// Tagged stops at `(location, tag)`.
fn tagged(stops: &[(f32, u64)]) -> GradientStops {
    GradientStops::new(stops.iter().map(|&(at, tag)| ColorStop { tag, ..ColorStop::new(at, RED) }).collect())
}

fn places(stops: &GradientStops) -> Vec<(f32, u64)> {
    stops.color.iter().map(|s| (s.location, s.tag)).collect()
}

/// A bar's operation applies to the stop its key names on the stops of its turn, which another
/// control changed, and keeps the selection on its stop.
#[test]
fn a_bar_operation_applies_to_its_stop_on_the_stops_of_its_turn() {
    let (rules, sampler) = (StopRules::default(), default_sampler(StopRows::Split));
    let base = tagged(&[(0.0, 1), (0.25, 2), (0.5, 3), (1.0, 4)]);
    let key = |i| StopKey::of(&base, StopRow::Color, i).unwrap();
    // Another control moved stop 2 past stop 3.
    let now = tagged(&[(0.0, 1), (0.5, 3), (0.75, 2), (1.0, 4)]);
    let apply = |op: BarOp, selected: &mut Option<StopRef>| stops::apply(&op, &now, selected, &rules, &sampler).map(|s| places(&s));

    let mut selected = Some(StopRef::Color(2));
    assert_eq!(apply(BarOp::Nudge { at: key(1), by: 0.125 }, &mut selected), Some(vec![(0.0, 1), (0.5, 3), (0.875, 2), (1.0, 4)]));
    assert_eq!(selected, Some(StopRef::Color(2)), "on the moved stop");

    let mut selected = Some(StopRef::Color(2));
    assert_eq!(apply(BarOp::Nudge { at: key(2), by: 0.375 }, &mut selected), Some(vec![(0.0, 1), (0.75, 2), (0.875, 3), (1.0, 4)]));
    assert_eq!(selected, Some(StopRef::Color(1)), "the selected stop the move passed keeps the selection");

    let mut selected = Some(StopRef::Color(2));
    assert_eq!(apply(BarOp::Add { row: StopRow::Color, after: Some(key(0)) }, &mut selected).map(|p| p[1]), Some((0.25, 0)));
    assert_eq!(selected, Some(StopRef::Color(1)), "the added stop");

    let mut selected = Some(StopRef::Color(2));
    assert_eq!(apply(BarOp::Remove(key(3)), &mut selected), Some(vec![(0.0, 1), (0.5, 3), (0.75, 2)]));
    assert_eq!(selected, Some(StopRef::Color(2)));
    let mut selected = Some(StopRef::Color(2));
    assert_eq!(apply(BarOp::Remove(key(1)), &mut selected), Some(vec![(0.0, 1), (0.5, 3), (1.0, 4)]));
    assert_eq!(selected, Some(StopRef::Color(2)), "the stop that takes the removed one's index");

    let mut selected = Some(StopRef::Color(1));
    let gone = StopKey { tag: 9, ..key(1) };
    assert_eq!(apply(BarOp::Nudge { at: gone, by: 0.125 }, &mut selected), None, "no stop with its tag");
    assert_eq!(selected, Some(StopRef::Color(1)));
}

#[test]
fn an_untagged_key_with_two_candidates_finds_none() {
    let base = tagged(&[(0.0, 0), (0.25, 0), (0.5, 0), (1.0, 0)]);
    let key = StopKey::of(&base, StopRow::Color, 1).unwrap();
    assert_eq!(key.find(&base), Some(1));
    assert_eq!(key.find(&tagged(&[(0.0, 0), (0.5, 0), (0.625, 0), (1.0, 0)])), Some(2), "the one stop that moved");
    assert_eq!(key.find(&tagged(&[(0.0, 0), (0.375, 0), (0.625, 0), (1.0, 0)])), None, "two stops moved");
    assert_eq!(key.find(&tagged(&[(0.0, 0), (0.25, 0), (0.25, 0), (1.0, 0)])), None, "another stop moved to its location");
    let (rules, sampler) = (StopRules::default(), default_sampler(StopRows::Split));
    let two_moved = tagged(&[(0.0, 0), (0.375, 0), (0.625, 0), (1.0, 0)]);
    assert_eq!(stops::apply(&BarOp::Nudge { at: key, by: 0.125 }, &two_moved, &mut None, &rules, &sampler), None, "the operation drops");

    // Stops that share a location are told apart by their order there.
    let mut tied = tagged(&[(0.0, 0), (0.5, 0), (0.5, 0), (1.0, 0)]);
    tied.color[1].color = BLUE;
    let second = StopKey::of(&tied, StopRow::Color, 2).unwrap();
    assert_eq!(second.find(&tied), Some(2));
    let out = stops::apply(&BarOp::Nudge { at: second, by: 0.125 }, &tied, &mut None, &rules, &sampler).expect("found");
    assert_eq!((places(&out)[2], out.color[1].color, out.color[2].color), ((0.625, 0), BLUE, RED));
}

/// An untagged stop that another control removed isn't mistaken for the one that moved instead:
/// its Nudge waits behind its Remove and drops, and the stop the scrub moved stays.
#[test]
fn an_untagged_key_does_not_find_another_stop_when_its_own_is_gone() {
    let (rules, sampler) = (StopRules::default(), default_sampler(StopRows::Split));
    let base = tagged(&[(0.0, 0), (0.25, 0), (0.5, 0), (1.0, 0)]);
    let (s, t) = (StopKey::of(&base, StopRow::Color, 1).unwrap(), StopKey::of(&base, StopRow::Color, 2).unwrap());
    // A scrub moved the stop at 0.5 to 0.4, then the stop at 0.25 was removed.
    let scrubbed = tagged(&[(0.0, 0), (0.25, 0), (0.4, 0), (1.0, 0)]);
    let removed = stops::apply(&BarOp::Remove(s.clone()), &scrubbed, &mut None, &rules, &sampler).expect("found");
    assert_eq!(places(&removed), [(0.0, 0), (0.4, 0), (1.0, 0)]);
    assert_eq!(stops::apply(&BarOp::Nudge { at: s, by: 0.125 }, &removed, &mut None, &rules, &sampler), None, "the nudge drops");
    assert_eq!(t.find(&removed), None, "the moved stop's own key doesn't find it with a neighbour gone");
}
