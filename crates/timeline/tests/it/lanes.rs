//! The keyframe lane, the marker strip, the overview and the pure lane rules.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::{Selection, SelectionChange};
use craft_timeline::ui::{
    KeyframeId, KeyframeInterpolation, KeyframeLane, KeyframeMark, KeyframeNavigator, LabelPalette, MarkerEdit, MarkerStrip, OverviewRun,
    ThumbnailMode, TimelineOverview, TimelineView, overview_runs, thumbnail_tiles,
};
use craft_timeline::{FrameRate, Marker, MarkerColor, MarkerId, Tick, TimeFormat, TimeRange, TimeView, format_time};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Rect, vec2};

const ROOM: egui::Vec2 = vec2(600.0, 60.0);
const POINTS_PER_SECOND: f64 = 50.0;

fn seconds(at: f64) -> Tick {
    Tick::from_seconds_f64(at)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(FrameRate::FPS_24)
}

/// Thumbnails: none, the head, head and tail, or tiles along the clip, only those in view.
#[trial]
fn thumbnail_tiles_follow_the_mode() {
    let (duration, frame) = (seconds(10.0), seconds(1.0 / 24.0));
    let tiles = |mode| thumbnail_tiles(mode, 0.0, 100.0, 20.0, (0.0, 100.0), duration, frame);
    assert!(tiles(ThumbnailMode::Off).is_empty());
    assert_eq!(tiles(ThumbnailMode::Head).len(), 1);
    assert_eq!(tiles(ThumbnailMode::HeadAndTail).len(), 2);
    assert_eq!(tiles(ThumbnailMode::Continuous).len(), 5);
    let cut = thumbnail_tiles(ThumbnailMode::Continuous, 0.0, 100.0, 20.0, (40.0, 60.0), duration, frame);
    assert!(cut.iter().all(|(x, _)| *x < 60.0 && *x + 20.0 > 40.0), "only the tiles in view: {cut:?}");
}

/// A drum edit holds tens of thousands of clips under a point wide: touching clips of one colour are one run.
#[trial]
fn runs_merge_touching_clips_of_one_colour() {
    let [red, blue, ..] = LabelPalette::default().0;
    let clip = |at: f64, color| (TimeRange::new(seconds(at), seconds(1.0)), color);
    let tracks = vec![vec![clip(0.0, red), clip(1.0, red), clip(2.0, blue), clip(5.0, blue)]];
    let runs: Vec<OverviewRun> = overview_runs(&tracks, seconds(10.0), vec2(200.0, 10.0));
    assert_eq!(runs.len(), 3, "two touching reds merge, the blue clips do not touch: {runs:?}");
    assert_eq!(runs[0].color, red);
    assert!(runs[0].rect.width() > runs[1].rect.width() * 1.5);
}

/// A click centres the window at the pointer.
#[trial]
fn click_centres_the_view() {
    let view = Rc::new(RefCell::new(TimeView::new(Tick::ZERO, POINTS_PER_SECOND)));
    let shown = view.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut view = shown.borrow_mut();
        let [red, ..] = LabelPalette::default().0;
        let runs = overview_runs(&[vec![(TimeRange::new(seconds(0.0), seconds(20.0)), red)]], seconds(40.0), vec2(ROOM.x, 30.0));
        TimelineOverview::new("Overview", &runs, Bind::bound(&mut *view), seconds(40.0)).shown_width(100.0).show(ui);
    });
    fixture.room(ROOM);
    fixture.settle();
    let overview = fixture.by_role(Role::IMAGE).into_iter().next().expect("the overview is an image");
    let at = overview.rect.min + vec2(overview.rect.width() * 0.75, overview.rect.height() / 2.0);
    fixture.click_at(at);
    let start = view.borrow().start;
    let (earliest, latest) = (seconds(30.0), seconds(40.0));
    assert!(start > earliest && start < latest, "the window is centred at three quarters of 50 s: {start:?}");
}

/// What a lane fixture reports.
#[derive(Default)]
struct Seen {
    selection: Selection<KeyframeId>,
    changes: Vec<SelectionChange<KeyframeId>>,
    deltas: Vec<Tick>,
    deleted: bool,
    double_clicked: Option<KeyframeId>,
}

/// The options of the lane or strip, earliest in time first: the tree lists nodes in no time order.
fn options_by_time(fixture: &Fixture) -> Vec<Target> {
    let mut options = fixture.by_role(Role::LIST_BOX_OPTION);
    options.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    options
}

fn keys() -> Vec<KeyframeMark> {
    let key = |id: u64, at: f64| KeyframeMark::new(KeyframeId::new(id), seconds(at), KeyframeInterpolation::Linear);
    vec![key(1, 1.0), key(2, 3.0), key(3, 5.0)]
}

fn lane() -> (Fixture, Rc<RefCell<Seen>>) {
    lane_in(None, Tick::ZERO)
}

/// A lane whose keys stay inside `bounds` and whose Shift-drags snap to `playhead`.
fn lane_in(bounds: Option<TimeRange>, playhead: Tick) -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let shown = seen.clone();
    let view = Rc::new(RefCell::new(TimeView::new(Tick::ZERO, POINTS_PER_SECOND)));
    let mut fixture = Fixture::new(move |ui| {
        let (mut seen, mut view) = (shown.borrow_mut(), view.borrow_mut());
        let keys = keys();
        let selection = seen.selection.clone();
        let area = Rect::from_min_size(ui.max_rect().min, vec2(ROOM.x, 28.0));
        let out = TimelineView::new("Timeline", Bind::bound(&mut *view), format(), seconds(10.0))
            .show(ui, |ui, _| {
                let lane = KeyframeLane::new("Keyframes", &keys, &selection).playhead(playhead);
                match bounds {
                    Some(bounds) => lane.bounds(bounds),
                    None => lane,
                }
                .show_at(ui, area)
            })
            .inner;
        if let Some(out) = out {
            if let Some(change) = out.select {
                seen.changes.push(change.clone());
                match change {
                    SelectionChange::Set(items) => seen.selection = Selection::new(items.clone(), items.last().copied(), items.last().copied()),
                    SelectionChange::Toggle(key) => {
                        let mut items = seen.selection.items.clone();
                        match items.iter().position(|k| *k == key) {
                            Some(at) => drop(items.remove(at)),
                            None => items.push(key),
                        }
                        seen.selection = Selection::new(items, Some(key), Some(key));
                    }
                    _ => seen.selection = Selection::default(),
                }
            }
            if let Some(&delta) = out.delta.value() {
                seen.deltas.push(delta);
            }
            seen.deleted |= out.delete;
            seen.double_clicked = seen.double_clicked.or(out.double_clicked);
        }
    });
    fixture.room(ROOM);
    fixture.settle();
    (fixture, seen)
}

/// A click replaces the selection with the key, Shift+click toggles it, a click on empty lane clears.
#[trial]
fn click_and_marquee_select_keys() {
    let (mut fixture, seen) = lane();
    let options = options_by_time(&fixture);
    assert_eq!(options.len(), 3);
    fixture.click(&options[1]);
    assert_eq!(seen.borrow().selection.items, vec![KeyframeId::new(2)]);
    fixture.hold(Modifiers::SHIFT);
    fixture.click(&options[0]);
    assert_eq!(seen.borrow().selection.items.len(), 2, "Shift+click adds the key");
    fixture.hold(Modifiers::NONE);
    let list = fixture.by_role(Role::LIST_BOX).into_iter().next().expect("the lane is a list box");
    fixture.click_at(list.rect.right_center() - vec2(4.0, 0.0));
    assert!(seen.borrow().selection.items.is_empty(), "a click on empty lane clears");
}

/// Left and Right select the previous and next key, Shift+Right nudges by a frame, Delete asks to delete.
#[trial]
fn keyboard_selects_nudges_and_deletes() {
    let (mut fixture, seen) = lane();
    let options = options_by_time(&fixture);
    fixture.click(&options[0]);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(seen.borrow().selection.items, vec![KeyframeId::new(2)], "Right picks the next key");
    fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    let frame = FrameRate::FPS_24.frame_duration();
    assert_eq!(seen.borrow().deltas.last().copied(), Some(frame), "Shift+Right nudges the selection one frame");
    fixture.key(Key::Delete, Modifiers::NONE);
    assert!(seen.borrow().deleted);
}

/// A double-click on a key is reported.
#[trial]
fn double_click_on_a_key_is_reported() {
    let (mut fixture, seen) = lane();
    let options = options_by_time(&fixture);
    fixture.click(&options[2]);
    fixture.click(&options[2]);
    assert_eq!(seen.borrow().double_clicked, Some(KeyframeId::new(3)));
}

/// What a strip fixture reports.
#[derive(Default)]
struct Strip {
    edits: Vec<MarkerEdit>,
    moved: Vec<Tick>,
}

fn markers() -> Vec<Marker> {
    let marker = |id: u64, at: f64| Marker {
        id: MarkerId(id),
        start: seconds(at),
        duration: Tick::ZERO,
        name: format!("M{id}"),
        comment: String::new(),
        color: MarkerColor(1),
    };
    vec![marker(1, 2.0), marker(2, 6.0)]
}

fn strip() -> (Fixture, Rc<RefCell<Strip>>) {
    let seen = Rc::new(RefCell::new(Strip::default()));
    let shown = seen.clone();
    let view = Rc::new(RefCell::new(TimeView::new(Tick::ZERO, POINTS_PER_SECOND)));
    let mut fixture = Fixture::new(move |ui| {
        let (mut seen, mut view) = (shown.borrow_mut(), view.borrow_mut());
        let markers = markers();
        let area = Rect::from_min_size(ui.max_rect().min, vec2(ROOM.x, 28.0));
        let out = TimelineView::new("Timeline", Bind::bound(&mut *view), format(), seconds(10.0))
            .show(ui, |ui, _| MarkerStrip::new("Markers", &markers).show_at(ui, area))
            .inner;
        if let Some(out) = out {
            seen.edits.extend(out.edits);
            if let Some(span) = out.span.value() {
                seen.moved.push(span.start);
            }
        }
    });
    fixture.room(ROOM);
    fixture.settle();
    (fixture, seen)
}

/// Dragging a marker moves it by the distance dragged.
#[trial]
fn dragging_a_marker_moves_it() {
    let (mut fixture, seen) = strip();
    let options = options_by_time(&fixture);
    fixture.drag(&options[0], vec2(POINTS_PER_SECOND as f32, 0.0));
    let last = seen.borrow().moved.last().copied().expect("the drag proposes a start");
    assert!(last > seconds(2.5) && last < seconds(3.5), "dragged one second right of 2 s: {last:?}");
}

/// Cmd-click deletes and a double-click opens.
#[trial]
fn command_click_deletes_and_double_click_opens() {
    let (mut fixture, seen) = strip();
    let options = options_by_time(&fixture);
    fixture.hold(Modifiers::COMMAND);
    fixture.click(&options[1]);
    fixture.hold(Modifiers::NONE);
    assert!(seen.borrow().edits.contains(&MarkerEdit::Delete(MarkerId(2))));
    fixture.click(&options[0]);
    fixture.click(&options[0]);
    assert!(seen.borrow().edits.contains(&MarkerEdit::Open(MarkerId(1))));
}

/// Right picks the next marker, Enter opens it, Delete deletes it.
#[trial]
fn keys_pick_open_and_delete_markers() {
    let (mut fixture, seen) = strip();
    let options = options_by_time(&fixture);
    fixture.click(&options[0]);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    assert!(seen.borrow().edits.contains(&MarkerEdit::Open(MarkerId(2))));
    fixture.key(Key::Delete, Modifiers::NONE);
    assert!(seen.borrow().edits.contains(&MarkerEdit::Delete(MarkerId(2))));
}

/// The option of the key at `at` seconds, named by its time as timecode.
fn key_at(fixture: &Fixture, at: f64) -> Target {
    let name = format_time(seconds(at), &format());
    fixture.by_label(&name).unwrap_or_else(|| panic!("no key named {name}"))
}

/// A dragged key moves by whole frames; with Shift it snaps to the playhead within a few points and not farther.
#[trial]
fn dragging_keys_snaps_to_frames_and_with_shift_to_the_playhead() {
    let frames = |count| FrameRate::FPS_24.tick_of(count);
    let (mut fixture, seen) = lane_in(None, seconds(4.0));
    let key = key_at(&fixture, 3.0);
    fixture.drag(&key, vec2(37.0, 0.0));
    assert_eq!(seen.borrow().deltas.last().copied(), Some(frames(18)), "0.74 s is 17.8 frames: the nearest frame");

    let (mut fixture, seen) = lane_in(None, seconds(4.0));
    let key = key_at(&fixture, 3.0);
    fixture.hold(Modifiers::SHIFT);
    fixture.drag(&key, vec2(48.0, 0.0));
    assert_eq!(seen.borrow().deltas.last().copied(), Some(seconds(1.0)), "2 points from the playhead: snapped to it");

    let (mut fixture, seen) = lane_in(None, seconds(4.0));
    let key = key_at(&fixture, 3.0);
    fixture.hold(Modifiers::SHIFT);
    fixture.drag(&key, vec2(38.0, 0.0));
    assert_eq!(seen.borrow().deltas.last().copied(), Some(frames(18)), "12 points away: only the frame grid");
}

/// A drag past the end of the bounds stops at the bound.
#[trial]
fn keys_stay_inside_the_bounds() {
    let (mut fixture, seen) = lane_in(Some(TimeRange::new(Tick::ZERO, seconds(5.5))), Tick::ZERO);
    let key = key_at(&fixture, 3.0);
    fixture.drag(&key, vec2(4.0 * POINTS_PER_SECOND as f32, 0.0));
    assert_eq!(seen.borrow().deltas.last().copied(), Some(seconds(2.5)), "the key stops at 5.5 s");
}

/// The navigator reports the previous and the toggle buttons; the next button is dimmed without a key after the playhead.
#[trial]
fn navigator_reports_its_buttons() {
    let pressed = Rc::new(RefCell::new((0, 0, 0)));
    let shown = pressed.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = KeyframeNavigator::new("Navigator", true, false, false).show(ui);
        let mut counts = shown.borrow_mut();
        counts.0 += usize::from(out.previous);
        counts.1 += usize::from(out.next);
        counts.2 += usize::from(out.toggle);
    });
    fixture.settle();
    for label in ["Go to previous keyframe", "Go to next keyframe", "Add/Remove Keyframe"] {
        let button = fixture.by_label(label).unwrap_or_else(|| panic!("no button {label}"));
        fixture.click(&button);
    }
    assert_eq!(*pressed.borrow(), (1, 0, 1));
}
