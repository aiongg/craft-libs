//! `TimelineRuler`: the tick ladder, the labels, scrubbing, the arrow keys and the work area.

use craft_behavior::{CancelReason, EditEvent};
use craft_timeline::ui::{DragEvent, TimelineRuler, TimelineView};
use craft_timeline::{FrameRate, Tick, TimeFormat, TimeRange, TimeView, format_time};
use craft_ui::canvas::StepLadder;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect};
use std::cell::RefCell;
use std::rc::Rc;

/// Points per second of the ruler's view.
const POINTS_PER_SECOND: f64 = 100.0;

/// The ruler's name, which names its AccessKit node.
const RULER: &str = "Time ruler";

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn rate() -> FrameRate {
    FrameRate::new(24, 1)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(rate())
}

#[trial]
fn ruler_steps_keep_labels_and_ticks_apart() {
    let ladder = StepLadder::new(24, 110.0, 12.0);
    for frame_points in [0.01, 0.2, 1.0, 3.7, 10.0, 24.0, 90.0, 400.0] {
        let (tick, label) = ladder.steps(frame_points);
        assert!(label as f64 * frame_points >= 110.0 || label == 24 * 3600, "labels at {frame_points}: {label}");
        assert!(tick as f64 * frame_points >= 12.0 || tick == label, "ticks at {frame_points}: {tick}");
        assert_eq!(label % tick, 0, "every label sits on a tick");
    }
    assert_eq!(ladder.steps(10.0), (2, 12), "12-frame labels with a tick every 2 frames");
    assert!(StepLadder::new(i64::MAX, 110.0, 12.0).steps(1.0).1 >= 110, "a huge base saturates");
}

/// What a shown ruler reports.
struct Shown {
    fixture: Fixture,
    playhead: Rc<RefCell<Tick>>,
    edits: Rc<RefCell<Vec<EditEvent<Tick>>>>,
    work: Rc<RefCell<Vec<DragEvent<TimeRange>>>>,
}

/// A ruler over `work_area` at 100 points a second, inside a timeline view that ends at a minute.
fn shown(work_area: Option<TimeRange>) -> Shown {
    let playhead = Rc::new(RefCell::new(Tick::ZERO));
    let edits: Rc<RefCell<Vec<EditEvent<Tick>>>> = Rc::default();
    let work: Rc<RefCell<Vec<DragEvent<TimeRange>>>> = Rc::default();
    let (head, seen_edits, seen_work) = (Rc::clone(&playhead), Rc::clone(&edits), Rc::clone(&work));
    let mut fixture = Fixture::new(move |ui| {
        let mut view = TimeView::new(Tick::ZERO, POINTS_PER_SECOND);
        let mut at = *head.borrow();
        TimelineView::new("Time", Bind::bound(&mut view), format(), secs(60.0)).show(ui, |ui, _cx| {
            let ruler = TimelineRuler::new(RULER, Bind::bound(&mut at));
            let ruler = match work_area {
                Some(area) => ruler.work_area(area),
                None => ruler,
            };
            let out = ruler.show(ui);
            seen_edits.borrow_mut().extend(out.playhead.event.clone());
            seen_work.borrow_mut().extend(out.work_area.clone());
        });
        *head.borrow_mut() = at;
    });
    fixture.room(egui::vec2(480.0, 120.0));
    fixture.settle();
    Shown { fixture, playhead, edits, work }
}

/// The point at `seconds` in the ruler `rect`, below its work-area band.
fn at(rect: Rect, seconds: f64) -> Pos2 {
    Pos2::new(rect.left() + seconds as f32 * POINTS_PER_SECOND as f32, rect.center().y + rect.height() * 0.2)
}

/// The pointer travel along the ruler that spans `seconds`.
fn travel(seconds: f64) -> egui::Vec2 {
    egui::vec2(seconds as f32 * POINTS_PER_SECOND as f32, 0.0)
}

/// Where a scrub starts and ends, in seconds.
const SCRUB_FROM: f64 = 1.0;
const SCRUB_TO: f64 = 3.0;

#[trial]
fn ruler_ticks_label_in_the_format() {
    let shown = shown(None);
    let texts: Vec<&str> = shown.fixture.texts().iter().map(|painted| painted.text.as_str()).collect();
    for seconds in [0.0, 1.0, 2.0] {
        let label = format_time(secs(seconds), &format());
        assert!(texts.contains(&label.as_str()), "{label} is labelled; the ruler shows {texts:?}");
    }
}

#[trial]
fn dragging_the_ruler_scrubs_the_playhead() {
    let mut shown = shown(None);
    let rect = shown.fixture.by_label(RULER).expect("the ruler has a node").rect;
    shown.fixture.drag_at(at(rect, SCRUB_FROM), at(rect, SCRUB_TO) - at(rect, SCRUB_FROM));
    let scrubbed_to = secs(SCRUB_TO);
    {
        let events = shown.edits.borrow();
        assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
        assert!(events.iter().any(|event| matches!(event, EditEvent::Update { .. })), "{events:?}");
        assert!(matches!(events.last(), Some(EditEvent::Commit { after, .. }) if *after == scrubbed_to), "{events:?}");
    }
    assert_eq!(*shown.playhead.borrow(), scrubbed_to);

    let mut cancelled = self::shown(None);
    cancelled.edits.borrow_mut().clear();
    let (from, to) = (at(rect, SCRUB_FROM), at(rect, SCRUB_FROM + 1.0));
    cancelled.fixture.frame(vec![pointer::moved(from)]);
    cancelled.fixture.frame(vec![pointer::press(from)]);
    cancelled.fixture.frame(vec![pointer::moved(to)]);
    cancelled.fixture.key(Key::Escape, Modifiers::NONE);
    let events = cancelled.edits.borrow();
    assert!(events.iter().any(|event| matches!(event, EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{events:?}");
}

#[trial]
fn arrow_keys_step_the_playhead_one_frame() {
    let mut shown = shown(None);
    let ruler = shown.fixture.by_label(RULER).expect("the ruler has a node");
    shown.fixture.access(AccessActionKind::Focus, &ruler);
    shown.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(*shown.playhead.borrow(), rate().tick_of(1), "an arrow steps one frame");
    shown.fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(*shown.playhead.borrow(), rate().tick_of(6), "Shift steps five");
    let sets = shown.edits.borrow().iter().filter(|event| matches!(event, EditEvent::Set { .. })).count();
    assert_eq!(sets, 2, "each key is one step");
}

/// A work area two seconds long from the second second.
fn work_area() -> TimeRange {
    TimeRange::new(secs(2.0), secs(2.0))
}

#[trial]
fn dragging_the_work_area_moves_it_or_an_end() {
    let mut shown = shown(Some(work_area()));
    let (area, shift, stretch) = (work_area(), 0.5, 1.0);
    let bar = shown.fixture.by_label("Work Area").expect("the work-area bar has a node").center();
    let moved = TimeRange::new(area.start + secs(shift), area.duration);
    shown.fixture.drag_at(bar, travel(shift));
    {
        let events = shown.work.borrow();
        assert!(matches!(events.first(), Some(DragEvent::Begin(_))), "{events:?}");
        assert!(matches!(events.last(), Some(DragEvent::Commit(range)) if *range == moved), "{events:?}");
    }

    let mut ended = self::shown(Some(work_area()));
    let end = ended.fixture.by_label("Work Area End").expect("the end has a node").center();
    ended.fixture.drag_at(end, travel(stretch));
    let stretched = TimeRange::from_bounds(area.start, area.end() + secs(stretch));
    let events = ended.work.borrow();
    assert!(matches!(events.last(), Some(DragEvent::Commit(range)) if *range == stretched), "{events:?}");
}

#[trial]
fn double_clicking_the_work_area_sets_it_to_the_visible_span() {
    let mut shown = shown(Some(work_area()));
    let rect = shown.fixture.by_label(RULER).expect("the ruler has a node").rect;
    let bar = shown.fixture.by_label("Work Area").expect("the work-area bar has a node");
    shown.fixture.click(&bar).click(&bar);
    let view = TimeView::new(Tick::ZERO, POINTS_PER_SECOND);
    let expected = TimeRange::from_bounds(view.start, view.time_at(rect.right(), rect.left()));
    let events = shown.work.borrow();
    assert!(events.contains(&DragEvent::Set(expected)), "{events:?}");
}

/// The name of the node that holds keyboard focus.
fn focused_name(fixture: &Fixture) -> Option<String> {
    let id = fixture.focused()?;
    fixture.tree().name_of(NodeKey::of(id)).map(str::to_owned)
}

/// Tab visits the time area, then the ruler, in reading order, and Shift+Tab walks back.
#[trial]
fn tab_visits_the_time_area_then_the_ruler() {
    let mut shown = shown(None);
    let mut seen = Vec::new();
    for _ in 0..2 {
        shown.fixture.key(Key::Tab, Modifiers::NONE);
        shown.fixture.settle();
        seen.push(focused_name(&shown.fixture));
    }
    assert_eq!(seen, [Some("Time".to_owned()), Some(RULER.to_owned())], "the time area first, then the ruler inside it");
    shown.fixture.key(Key::Tab, Modifiers::SHIFT);
    shown.fixture.settle();
    assert_eq!(focused_name(&shown.fixture), Some("Time".to_owned()), "Shift+Tab returns to the time area");
}
