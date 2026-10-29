//! The stop rules and the default sampler: pure functions of the stops.

use craft_theme::Srgba;

use super::stops::{self, StopRules, default_sampler, sample};
use super::{ColorStop, GradientStops, OpacityStop, StopRow, StopRows};

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
