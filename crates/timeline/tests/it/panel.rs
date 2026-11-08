//! The time area and the track stack: keys, wheel, follow and the move rules.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::EditEvent;
use craft_timeline::ui::stack::{MoveTarget, RowGeom, Side, resolve_move};
use craft_timeline::ui::view::TimelineView;
use craft_timeline::{FollowMode, FrameRate, Tick, TimeFormat, TimeView, TrackKind};
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Key, Modifiers, MouseWheelUnit, Vec2};

const RATE: FrameRate = FrameRate::FPS_29_97;

struct Rig {
    view: TimeView,
    playhead: Tick,
    playing: bool,
    points: Vec<Tick>,
    moved: Vec<Tick>,
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn rig() -> (Fixture, Rc<RefCell<Rig>>) {
    let state = Rc::new(RefCell::new(Rig {
        view: TimeView::new(Tick::ZERO, 40.0),
        playhead: secs(1.0),
        playing: false,
        points: vec![secs(2.0), secs(6.0)],
        moved: Vec::new(),
    }));
    let shown = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut rig = shown.borrow_mut();
        let format = TimeFormat::timecode(RATE);
        let points = rig.points.clone();
        let out = TimelineView::new("Time", Bind::given(rig.view), format, secs(30.0))
            .follow(FollowMode::Page, rig.playing, rig.playhead)
            .edit_points(&points)
            .show(ui, |_, _| {})
            .output;
        if let Some(next) = out.view.event.as_ref().and_then(EditEvent::value) {
            rig.view = *next;
        }
        if let Some(EditEvent::Set { after, .. }) = out.playhead {
            rig.playhead = after;
            rig.moved.push(after);
        }
    });
    fixture.idle(2);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    (fixture, state)
}

fn frame() -> Tick {
    RATE.tick_of(1)
}

/// Left and Right step the playhead a frame, Shift five; Alt+Right is not Right.
#[trial]
fn arrows_step_the_playhead_by_exact_chords() {
    let (mut fixture, state) = rig();
    let start = state.borrow().playhead;
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, start + frame());
    fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(state.borrow().playhead, start + frame() + RATE.tick_of(5));
    let before = state.borrow().moved.len();
    fixture.key(Key::ArrowRight, Modifiers::ALT);
    assert_eq!(state.borrow().moved.len(), before, "Alt+Right is a clip nudge, not a playhead step");
}

/// Home and End go to the start and the end; Up and Down go to the previous and next edit point.
#[trial]
fn home_end_and_edit_points_move_the_playhead() {
    let (mut fixture, state) = rig();
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, secs(2.0));
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, secs(6.0));
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, secs(2.0));
    fixture.key(Key::End, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, secs(30.0));
    fixture.key(Key::Home, Modifiers::NONE);
    assert_eq!(state.borrow().playhead, Tick::ZERO);
}

/// `=` and `-` zoom about the playhead and `\` fits the sequence.
#[trial]
fn zoom_keys_zoom_about_the_playhead_and_fit_the_sequence() {
    let (mut fixture, state) = rig();
    let before = state.borrow().view.points_per_second;
    fixture.key(Key::Equals, Modifiers::NONE);
    fixture.idle(30);
    assert!(state.borrow().view.points_per_second > before);
    fixture.key(Key::Minus, Modifiers::NONE);
    fixture.key(Key::Minus, Modifiers::NONE);
    fixture.idle(30);
    assert!(state.borrow().view.points_per_second < before);
    fixture.key(Key::Backslash, Modifiers::NONE);
    fixture.idle(30);
    assert_eq!(state.borrow().view.start, Tick::ZERO);
}

fn wheel(fixture: &mut Fixture, delta: Vec2, modifiers: Modifiers) {
    let at = fixture.shown().center();
    fixture.frame(vec![Event::PointerMoved(at), Event::MouseWheel { unit: MouseWheelUnit::Point, delta, modifiers }]);
}

/// Alt+wheel zooms about the pointer by `1 + dy * 0.01`; Cmd or Ctrl and the wheel scroll time sideways.
#[trial]
fn the_wheel_maps_as_filmcraft_does() {
    let (mut fixture, state) = rig();
    let before = state.borrow().view.points_per_second;
    wheel(&mut fixture, Vec2::new(0.0, 10.0), Modifiers::ALT);
    fixture.idle(30);
    let zoomed = state.borrow().view.points_per_second;
    assert!((zoomed / before - 1.1).abs() < 0.02, "a 10 point wheel zooms by 10 percent, got {}", zoomed / before);

    state.borrow_mut().view = TimeView::new(secs(10.0), 40.0);
    fixture.idle(2);
    wheel(&mut fixture, Vec2::new(0.0, 40.0), Modifiers::COMMAND);
    let start = state.borrow().view.start;
    assert!(start < secs(10.0), "a downward Cmd+wheel moves the view to later times as FilmCraft does: {start:?}");
}

/// While playing the view follows the playhead; a jump while stopped reveals it.
#[trial]
fn view_follows_the_playhead_while_playing() {
    let (mut fixture, state) = rig();
    {
        let mut rig = state.borrow_mut();
        rig.playing = true;
        rig.playhead = secs(60.0).min(secs(29.0));
    }
    fixture.idle(4);
    assert!(state.borrow().view.start > Tick::ZERO, "the view paged to the playhead");

    {
        let mut rig = state.borrow_mut();
        rig.playing = false;
        rig.view = TimeView::new(Tick::ZERO, 40.0);
        rig.playhead = secs(1.0);
    }
    fixture.idle(2);
    state.borrow_mut().playhead = secs(28.0);
    fixture.idle(4);
    assert!(state.borrow().view.start > Tick::ZERO, "a jump while stopped reveals the playhead");
}

fn rows() -> Vec<RowGeom> {
    // V2 and V1 above the divider (V1 nearest), A1 and A2 below it.
    vec![
        RowGeom::new(TrackKind::Video, Side::Upper, 1, 0.0, 40.0),
        RowGeom::new(TrackKind::Video, Side::Upper, 0, 40.0, 80.0),
        RowGeom::new(TrackKind::Audio, Side::Lower, 0, 90.0, 130.0),
        RowGeom::new(TrackKind::Audio, Side::Lower, 1, 130.0, 170.0),
    ]
}

/// A move takes its track fields from the pointer's row: the same kind counts rows, another kind stays, and the empty
/// space past the last track of the kind adds one.
#[trial]
fn a_move_gets_its_track_fields_from_the_pointer() {
    let rows = rows();
    let bounds = (0.0, 170.0);
    let up = resolve_move(&rows, &[rows[1]], 20.0, bounds);
    assert_eq!(up, MoveTarget { track_delta: 1, new_track: false, refused: false }, "V1 to V2 is one row away from the divider");
    let across = resolve_move(&rows, &[rows[1]], 100.0, bounds);
    assert_eq!(across, MoveTarget::STAY, "a video clip over an audio row stays");
    let past = resolve_move(&rows, &[rows[0]], -5.0, (-10.0, 170.0));
    assert!(past.new_track && !past.refused, "above the top video track a drop adds one");
    assert_eq!(resolve_move(&rows, &[rows[1]], 500.0, bounds), MoveTarget::STAY, "outside the panes nothing changes");
}

/// A move from or onto a locked track is refused and stays where it was.
#[trial]
fn moves_onto_or_from_locked_tracks_are_refused() {
    let mut rows = rows();
    rows[0] = rows[0].locked();
    let onto = resolve_move(&rows, &[rows[1]], 20.0, (0.0, 170.0));
    assert!(onto.refused && onto.track_delta == 0);
    let from = resolve_move(&rows, &[rows[0]], 60.0, (0.0, 170.0));
    assert!(from.refused, "a clip on a locked track does not move");
}

/// A selected clip on a locked track out of view refuses a new track.
#[trial]
fn a_selected_clip_on_a_locked_track_out_of_view_refuses_the_new_track() {
    let rows = rows();
    let locked_away = rows[1].locked();
    let target = resolve_move(&rows, &[rows[0], locked_away], -5.0, (-10.0, 170.0));
    assert!(target.refused && !target.new_track);
}
