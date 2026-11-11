//! The gradient stop bar: the stop rules and the default sampler, then the editor by pointer, keys, fields and
//! AccessKit.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::EditEvent;
use craft_draw::ui::gradient::stops::{self, StopRules, default_sampler, sample};
use craft_draw::ui::gradient::{ColorStop, GradientEditor, GradientStops, OpacityStop, StopDrop, StopRef, StopRow, StopRows};
use craft_theme::Srgba;
use craft_ui::contract::Bind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn red_to_blue() -> GradientStops {
    GradientStops::new(vec![ColorStop::new(0.0, RED).with_tag(7), ColorStop::new(1.0, BLUE).with_tag(9)])
}

fn close(a: Srgba, b: Srgba) -> bool {
    [a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a].iter().all(|d| d.abs() < 1e-5)
}

#[trial]
fn a_new_stop_takes_the_colour_there_and_resets_the_segment_it_splits() {
    let rules = StopRules::default();
    let stops = GradientStops::new(vec![ColorStop::new(0.0, RED).with_tag(7).with_midpoint(0.2), ColorStop::new(1.0, BLUE).with_tag(9)]);
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

    assert!(stops::add(&stops, StopRow::Color, 0.5, &sampler, &StopRules::default().stops(2, 2)).is_none());
    assert!(stops::add(&stops, StopRow::Opacity, 0.5, &sampler, &StopRules::new(StopRows::Combined)).is_none());
}

#[trial]
fn a_stop_keeps_its_place_among_stops_at_its_location_unless_it_crossed_them() {
    let rules = StopRules::default();
    let tags = |s: &GradientStops| s.color.iter().map(|s| s.tag).collect::<Vec<_>>();
    // A hard edge at 1.0: a nudge past the end clamps and changes nothing.
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop::new(1.0, Srgba::BLACK).with_tag(8));
    let (out, at) = stops::move_to(&stops, StopRow::Color, 1, 1.01, &rules).unwrap();
    assert_eq!((out, at), (stops.clone(), 1));

    // Reaching a stop's location from below it stays below, from above it stays above.
    let mut stops = red_to_blue();
    stops.color.insert(1, ColorStop::new(0.5, Srgba::BLACK).with_tag(8));
    let (out, at) = stops::move_to(&stops, StopRow::Color, 0, 0.5, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![7, 8, 9], 0));
    let (out, at) = stops::move_to(&stops, StopRow::Color, 2, 0.5, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![7, 8, 9], 2));
    // Moving past the tie crosses it.
    let tied = stops::move_to(&stops, StopRow::Color, 0, 0.5, &rules).unwrap().0;
    let (out, at) = stops::move_to(&tied, StopRow::Color, 0, 0.6, &rules).unwrap();
    assert_eq!((tags(&out), at), (vec![8, 7, 9], 1));
}

#[trial]
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

#[trial]
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

/// What the editor reported, pass by pass.
#[derive(Default)]
struct Seen {
    events: Vec<&'static str>,
    requested: Vec<usize>,
    dropped: Vec<StopDrop>,
    selected: Option<StopRef>,
}

struct Rig {
    fixture: Fixture,
    stops: Rc<RefCell<GradientStops>>,
    seen: Rc<RefCell<Seen>>,
}

fn kind<T>(event: &EditEvent<T>) -> &'static str {
    match event {
        EditEvent::Begin { .. } => "begin",
        EditEvent::Update { .. } => "update",
        EditEvent::Commit { .. } => "commit",
        EditEvent::Cancel { .. } => "cancel",
        EditEvent::Set { .. } => "set",
        _ => "other",
    }
}

fn rig(start: GradientStops, rules: StopRules, fields: bool) -> Rig {
    let stops = Rc::new(RefCell::new(start));
    let seen = Rc::new(RefCell::new(Seen::default()));
    let (shared, report) = (stops.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut stops = shared.borrow_mut();
        let out = GradientEditor::new("Gradient", Bind::bound(&mut stops)).rules(rules.clone()).fields(fields).drop_target::<u32>().show(ui);
        let mut seen = report.borrow_mut();
        seen.events.extend(out.edit.events.iter().map(kind));
        seen.requested.extend(out.color_requested.iter().map(|c| c.index));
        seen.dropped.extend(out.dropped.iter().map(|(place, _)| *place));
        seen.selected = out.selected;
    });
    fixture.settle();
    Rig { fixture, stops, seen }
}

fn three() -> GradientStops {
    GradientStops::new(vec![
        ColorStop::new(0.0, RED).with_tag(1),
        ColorStop::new(0.5, Srgba::BLACK).with_tag(2),
        ColorStop::new(1.0, BLUE).with_tag(3),
    ])
}

impl Rig {
    fn marker(&self, name: &str) -> craft_ui::testing::Target {
        self.fixture.by_label(name).unwrap_or_else(|| panic!("{name} is in the tree"))
    }

    fn tags(&self) -> Vec<u64> {
        self.stops.borrow().color.iter().map(|s| s.tag).collect()
    }
}

#[trial]
fn a_drag_past_a_neighbour_is_one_gesture_and_the_selection_follows_the_stop() {
    let mut rig = rig(three(), StopRules::default(), false);
    let first = rig.marker("Color stop 1 of 3");
    rig.seen.borrow_mut().events.clear();
    rig.fixture.drag(&first, Vec2::new(250.0, 0.0));
    rig.fixture.idle(2);
    assert_eq!(rig.tags(), [2, 1, 3], "the first stop passed the middle one");
    assert_eq!(rig.seen.borrow().selected, Some(StopRef::Color(1)), "the selection followed the dragged stop");
    let events = rig.seen.borrow().events.clone();
    assert_eq!(events.first(), Some(&"begin"));
    assert_eq!(events.iter().filter(|e| **e == "begin").count(), 1, "one gesture");
    assert!(matches!(events.last(), Some(&"commit") | Some(&"set")), "{events:?}");
}

#[trial]
fn a_stop_torn_off_goes_and_stays_removed_when_released_off_the_bar_but_not_at_the_minimum() {
    let mut rig = rig(three(), StopRules::default(), false);
    let middle = rig.marker("Color stop 2 of 3");
    rig.fixture.drag(&middle, Vec2::new(0.0, 120.0));
    rig.fixture.idle(2);
    assert_eq!(rig.tags(), [1, 3], "dragged far off the ramp, the stop is removed");
    let first = rig.marker("Color stop 1 of 2");
    rig.fixture.drag(&first, Vec2::new(0.0, 120.0));
    rig.fixture.idle(2);
    assert_eq!(rig.stops.borrow().color.len(), 2, "at the row's minimum the stop stays");
}

#[trial]
fn plus_and_minus_walk_the_markers_and_up_and_down_change_rows() {
    let stops = GradientStops::with_opacity(
        vec![ColorStop::new(0.0, RED), ColorStop::new(1.0, BLUE)],
        vec![OpacityStop::new(0.0, 1.0), OpacityStop::new(1.0, 1.0)],
    );
    let mut rig = rig(stops, StopRules::default(), false);
    let first = rig.marker("Color stop 1 of 2");
    rig.fixture.click(&first);
    rig.fixture.idle(2);
    rig.fixture.key(Key::Plus, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(rig.seen.borrow().selected, Some(StopRef::Midpoint(StopRow::Color, 0)), "+ walks to the midpoint between the stops");
    rig.fixture.key(Key::Plus, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(rig.seen.borrow().selected, Some(StopRef::Color(1)));
    rig.fixture.key(Key::Minus, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(rig.seen.borrow().selected, Some(StopRef::Midpoint(StopRow::Color, 0)));
    rig.fixture.key(Key::ArrowUp, Modifiers::NONE);
    rig.fixture.idle(2);
    assert!(matches!(rig.seen.borrow().selected, Some(StopRef::Opacity(_))), "Up selects the nearest opacity stop");
}

#[trial]
fn color_only_editors_have_no_opacity_markers_and_the_fields_edit_the_selected_stop() {
    let mut rig = rig(three(), StopRules::new(StopRows::ColorOnly), true);
    assert!(rig.fixture.by_label("Opacity stop 1 of 2").is_none(), "no opacity row");
    let first = rig.marker("Color stop 1 of 3");
    rig.fixture.click(&first);
    rig.fixture.idle(2);
    let location = rig.fixture.by_label("Location").expect("the Location field is shown for the selected stop");
    rig.fixture.click(&location);
    rig.fixture.key(Key::A, Modifiers::COMMAND);
    rig.fixture.type_text("80");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    rig.fixture.idle(3);
    assert_eq!(rig.tags(), [2, 1, 3], "the first stop moved to 80 %, past the middle one");
    assert_eq!(rig.seen.borrow().selected, Some(StopRef::Color(1)), "the selection stayed on the moved stop");
}
