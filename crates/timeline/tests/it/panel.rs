//! The time area and the track stack: keys, wheel, follow and the move rules.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::EditEvent;
use craft_timeline::ui::stack::{MoveTarget, RowGeom, Side, resolve_move};
use craft_timeline::ui::view::TimelineView;
use craft_timeline::ui::{Timeline, TimelineEdit, TrackSwitch};
use craft_timeline::{ClipId, ClipInfo, TimeRange, TimelineModel, TrackId, TrackInfo};
use craft_timeline::{FollowMode, FrameRate, Tick, TimeFormat, TimeView, TrackKind};
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Fixture, Frames, Target, pointer, trial};
use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, Rect, TouchPhase, Vec2};

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
        playhead: RATE.tick_of(30),
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

/// A wheel turn of `delta` points over the view's middle, carrying `modifiers` the way a device reports them.
fn wheel(fixture: &mut Fixture, delta: Vec2, modifiers: Modifiers) {
    let at = fixture.shown().center();
    let turn = Event::MouseWheel { unit: MouseWheelUnit::Point, delta, phase: TouchPhase::Move, modifiers, source: MouseWheelSource::Wheel };
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![turn]);
}

/// Alt+wheel zooms about the pointer by `1 + dy * 0.01`; Cmd or Ctrl+wheel zooms as the pinch does; Shift+wheel scrolls
/// time sideways; the plain wheel leaves the time alone.
#[trial]
fn the_wheel_maps_to_zoom_and_sideways_scroll() {
    let (mut fixture, state) = rig();
    let before = state.borrow().view.points_per_second;
    wheel(&mut fixture, Vec2::new(0.0, 10.0), Modifiers::ALT);
    fixture.idle(30);
    let zoomed = state.borrow().view.points_per_second;
    assert!((zoomed / before - 1.1).abs() < 0.02, "a 10 point wheel zooms by 10 percent, got {}", zoomed / before);

    state.borrow_mut().view = TimeView::new(secs(10.0), 40.0);
    fixture.idle(2);
    wheel(&mut fixture, Vec2::new(0.0, 40.0), Modifiers::SHIFT);
    let view = state.borrow().view;
    assert!(view.start < secs(10.0), "a downward Shift+wheel moves the view to earlier times as FilmCraft does: {:?}", view.start);
    assert_eq!(view.points_per_second, 40.0, "Shift+wheel does not zoom");

    state.borrow_mut().view = TimeView::new(secs(10.0), 40.0);
    fixture.idle(2);
    wheel(&mut fixture, Vec2::new(0.0, 40.0), Modifiers::NONE);
    assert_eq!(state.borrow().view.start, secs(10.0), "the plain wheel scrolls the tracks, not the time");

    state.borrow_mut().view = TimeView::new(Tick::ZERO, 40.0);
    fixture.idle(2);
    let at = fixture.shown().center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::zoom_wheel(40.0)]);
    fixture.idle(30);
    assert!(state.borrow().view.points_per_second > 40.0, "Cmd+wheel zooms: {:?}", state.borrow().view);
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
    assert_eq!((up.track_delta, up.new_track, up.refused), (1, false, false), "V1 to V2 is one row away from the divider");
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

// The Timeline panel over a model of tracks, one clip in V1 and one in A1.

const V1: TrackId = TrackId(1);
const V2: TrackId = TrackId(2);
const A1: TrackId = TrackId(3);
const C1: TrackId = TrackId(9);
/// The panel's room.
const PANEL_ROOM: Vec2 = Vec2::new(900.0, 500.0);
/// The time the panel's view shows per second of room.
const PANEL_POINTS_PER_SECOND: f64 = 20.0;
/// Audio tracks beyond the room's share, so the audio side scrolls.
const EXTRA_AUDIO: u64 = 12;

struct Sequence {
    tracks: Vec<(TrackId, TrackKind, String)>,
    master: Option<f32>,
    view: TimeView,
    playhead: Tick,
    selection: Vec<ClipId>,
    /// The clips that are switched off.
    disabled: Vec<ClipId>,
    edits: Vec<TimelineEdit>,
}

impl TimelineModel for Sequence {
    fn duration(&self) -> Tick {
        secs(30.0)
    }

    fn format(&self) -> TimeFormat {
        TimeFormat::timecode(RATE)
    }

    fn playhead(&self) -> Tick {
        self.playhead
    }

    fn view(&mut self) -> &mut TimeView {
        &mut self.view
    }

    fn tracks(&self) -> Vec<TrackInfo<'_>> {
        self.tracks
            .iter()
            .map(|(id, kind, name)| TrackInfo {
                id: *id,
                kind: *kind,
                name,
                height: None,
                depth: 0,
                label: None,
                locked: Some(false),
                targeted: Some(true),
                sync_lock: None,
                enabled: Some(true),
                muted: (*kind == TrackKind::Audio).then_some(false),
                solo: (*kind == TrackKind::Audio).then_some(false),
            })
            .collect()
    }

    fn clips(&self, track: TrackId, _: TimeRange) -> Vec<ClipInfo<'_>> {
        let clip = |id: u64| ClipInfo {
            id: ClipId(id),
            track,
            range: TimeRange::from_bounds(secs(2.0), secs(6.0)),
            name: "Clip",
            label: None,
            enabled: !self.disabled.contains(&ClipId(id)),
            source: None,
        };
        match track {
            V1 => vec![clip(1)],
            A1 => vec![clip(2)],
            _ => Vec::new(),
        }
    }

    fn selection(&self) -> &[ClipId] {
        &self.selection
    }

    fn master_volume_db(&self) -> Option<f32> {
        self.master
    }
}

struct Panel {
    fixture: Fixture,
    sequence: Rc<RefCell<Sequence>>,
}

fn panel_with(tracks: Vec<(TrackId, TrackKind, String)>, master: Option<f32>) -> Panel {
    let sequence = Rc::new(RefCell::new(Sequence {
        tracks,
        master,
        view: TimeView::new(Tick::ZERO, PANEL_POINTS_PER_SECOND),
        playhead: Tick::ZERO,
        selection: Vec::new(),
        disabled: Vec::new(),
        edits: Vec::new(),
    }));
    let shown = sequence.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut sequence = shown.borrow_mut();
        let out = Timeline::new("Timeline", &mut *sequence).show(ui);
        sequence.edits.extend(out.edits);
    });
    fixture.room(PANEL_ROOM);
    fixture.settle();
    Panel { fixture, sequence }
}

fn named(tracks: &[(TrackId, TrackKind, &str)]) -> Vec<(TrackId, TrackKind, String)> {
    tracks.iter().map(|(id, kind, name)| (*id, *kind, (*name).to_owned())).collect()
}

/// V2 and V1 over A1.
fn panel() -> Panel {
    panel_with(named(&[(V2, TrackKind::Video, "V2"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]), None)
}

/// V1 over A1 and many more audio tracks.
fn many_audio() -> Panel {
    let mut tracks = named(&[(V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]);
    tracks.extend((2..2 + EXTRA_AUDIO).map(|n| (TrackId(20 + n), TrackKind::Audio, format!("A{n}"))));
    panel_with(tracks, None)
}

impl Panel {
    /// The row of the track called `name`: of the nodes with that name, the widest (the row spans the header and the lane).
    fn row(&self, name: &str) -> Rect {
        let tree = self.fixture.tree();
        tree.keys()
            .into_iter()
            .filter(|key| tree.name_of(*key) == Some(name))
            .filter_map(|key| self.fixture.target(key))
            .map(|target| target.rect)
            .max_by(|a, b| a.width().total_cmp(&b.width()))
            .unwrap_or_else(|| panic!("no row named {name}"))
    }

    /// The node whose name contains `name` and which lies inside `row`.
    fn in_row(&self, name: &str, row: Rect) -> Target {
        let tree = self.fixture.tree();
        tree.keys()
            .into_iter()
            .filter(|key| tree.name_of(*key).is_some_and(|label| label.contains(name)))
            .filter_map(|key| self.fixture.target(key))
            .find(|target| row.contains(target.center()))
            .unwrap_or_else(|| panic!("no node named {name} in {row:?}"))
    }

    fn edits(&self) -> Vec<TimelineEdit> {
        self.sequence.borrow().edits.clone()
    }

    fn heights(&self) -> Vec<(TrackId, f32)> {
        self.edits()
            .into_iter()
            .filter_map(|edit| match edit {
                TimelineEdit::TrackHeight { track, height, .. } => height.value().map(|height| (track, *height)),
                _ => None,
            })
            .collect()
    }

    /// Presses at `from` and drags by `by` in four steps.
    fn drag_by(&mut self, from: egui::Pos2, by: Vec2) {
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![pointer::press(from)]);
        for step in 1..=4 {
            self.fixture.frame(vec![pointer::moved(from + by * (step as f32 / 4.0))]);
        }
        self.fixture.frame(vec![pointer::release(from + by)]);
        self.fixture.idle(2);
    }
}

/// Video tracks stack up from the divider with V1 nearest it, and audio goes down.
#[trial]
fn video_tracks_stack_up_from_the_divider() {
    let panel = panel();
    let (v2, v1, a1) = (panel.row("V2"), panel.row("V1"), panel.row("A1"));
    assert!(v2.bottom() <= v1.top() + SLACK, "V2 lies above V1: {v2:?} {v1:?}");
    assert!(v1.bottom() < a1.top(), "the divider lies between V1 and A1: {v1:?} {a1:?}");
    assert_eq!(v2.height(), v1.height());
}

/// Clicking Lock reports the new state; Alt-click on Solo is exclusive.
#[trial]
fn header_switches_report_their_new_state() {
    let mut panel = panel();
    let lock = panel.in_row("Toggle Track Lock", panel.row("V1"));
    panel.fixture.click(&lock);
    let solo = panel.in_row("Solo Track", panel.row("A1"));
    panel.fixture.hold(Modifiers::ALT);
    panel.fixture.click(&solo);
    panel.fixture.hold(Modifiers::NONE);
    let switches: Vec<_> = panel
        .edits()
        .into_iter()
        .filter_map(|edit| match edit {
            TimelineEdit::Switch { track, switch, value, exclusive, .. } => Some((track, switch, value, exclusive)),
            _ => None,
        })
        .collect();
    assert_eq!(switches, [(V1, TrackSwitch::Locked, true, false), (A1, TrackSwitch::Solo, true, true)]);
}

/// A row resizes from its own height: video rows grow upward, so an edge dragged up adds height, and the top video row
/// has a top edge.
#[trial]
fn rows_resize_from_their_height_and_video_rows_grow_upward() {
    let mut panel = panel();
    let a1 = panel.row("A1");
    let edge = panel.in_row("Drag to change the height of A1", a1);
    panel.drag_by(edge.center(), Vec2::new(0.0, RESIZE_TRAVEL));
    let got = panel.heights();
    assert_eq!(got.last().map(|(track, _)| *track), Some(A1), "{got:?}");
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (a1.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", a1.height());

    let mut panel = self::panel();
    let v1 = panel.row("V1");
    let edge = panel.in_row("Drag to change the height of V1", v1);
    panel.drag_by(edge.center(), Vec2::new(0.0, -RESIZE_TRAVEL));
    let got = panel.heights();
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (v1.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", v1.height());

    let mut panel = self::panel();
    let v2 = panel.row("V2");
    let top = panel.in_row("Drag the top edge to change the height of V2", v2);
    panel.drag_by(top.center(), Vec2::new(0.0, -RESIZE_TRAVEL));
    let got = panel.heights();
    assert_eq!(got.last().map(|(track, _)| *track), Some(V2), "{got:?}");
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (v2.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", v2.height());
}

/// The distance a height edge is dragged in the resize trial.
const RESIZE_TRAVEL: f32 = 10.0;

/// A double click on a header toggles the row between its two named heights.
#[trial]
fn double_clicking_a_header_toggles_its_height() {
    let mut panel = panel();
    let a1 = panel.row("A1");
    let before = a1.height();
    let name = panel
        .fixture
        .by_role(craft_ui::contract::Role::LABEL)
        .into_iter()
        .find(|label| panel.fixture.tree().name_of(label.node) == Some("A1") && a1.contains(label.center()))
        .expect("the header names its track");
    panel.fixture.frame(vec![pointer::moved(name.center())]);
    panel.fixture.click(&name);
    panel.fixture.click(&name);
    let got = panel.heights();
    assert_eq!(got.len(), 1, "one height from the double click: {got:?}");
    assert_eq!(got[0].0, A1);
    assert_ne!(got[0].1, before, "the height changed to the other named height");
}

/// The rounding slack of a measured point.
const SLACK: f32 = 0.5;

/// Captions sit above the video rows; a model with a master volume shows the Mix row below the audio rows.
#[trial]
fn caption_rows_sit_above_video_with_a_mix_row() {
    let panel = panel_with(named(&[(C1, TrackKind::Caption, "C1"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]), Some(-3.0));
    let (c, v1, a1) = (panel.row("C1"), panel.row("V1"), panel.row("A1"));
    assert!(c.bottom() <= v1.top() + SLACK, "{c:?} {v1:?}");
    assert!(v1.bottom() < a1.top(), "{v1:?} {a1:?}");
    let mix = panel.fixture.by_label("Mix").expect("a model with a master volume has a Mix row");
    assert!(mix.rect.top() >= a1.bottom() - SLACK, "the Mix row lies below the audio rows: {:?} {a1:?}", mix.rect);
}

/// The plain wheel scrolls the side of the divider under the pointer.
#[trial]
fn the_wheel_scrolls_the_side_under_the_pointer() {
    let mut panel = many_audio();
    let before = panel.row("A3").top();
    let over = panel.row("A2").center();
    panel.fixture.frame(vec![pointer::moved(over)]);
    panel.fixture.frame(vec![pointer::wheel(-WHEEL_POINTS)]);
    panel.fixture.idle(2);
    let after = panel.row("A3").top();
    assert!((before - after - WHEEL_POINTS).abs() < SLACK, "{before} -> {after}");
}

/// The points one wheel turn moves.
const WHEEL_POINTS: f32 = 30.0;

/// The Menu key and Shift+F10 on a clicked clip report a context menu anchored on that clip; a secondary click on it
/// reports its position.
#[trial]
fn the_menu_keys_report_a_context_menu_anchored_on_the_focused_clip() {
    let mut panel = panel();
    let v1 = panel.row("V1");
    let clip = panel.in_row("Clip", v1);
    panel.fixture.click(&clip);
    panel.fixture.key(Key::Menu, Modifiers::NONE);
    panel.fixture.key(Key::F10, Modifiers::SHIFT);
    let menus: Vec<_> = panel
        .edits()
        .into_iter()
        .filter_map(|edit| match edit {
            TimelineEdit::ContextMenu { track, clip, anchor, pointer, .. } => Some((track, clip, anchor, pointer)),
            _ => None,
        })
        .collect();
    assert_eq!(menus.len(), 2, "{menus:?}");
    for (track, found, anchor, pointer) in &menus {
        assert_eq!((*track, *found, *pointer), (Some(V1), Some(ClipId(1)), None));
        assert!(
            (anchor.left() - clip.rect.left()).abs() < SLACK && (anchor.bottom() - clip.rect.bottom()).abs() < SLACK,
            "{anchor:?} at {:?}",
            clip.rect
        );
    }

    panel.sequence.borrow_mut().edits.clear();
    let at = clip.center();
    panel.fixture.frame(vec![pointer::moved(at)]);
    panel.fixture.frame(vec![pointer::secondary(at, true)]);
    panel.fixture.frame(vec![pointer::secondary(at, false)]);
    let click = panel.edits().into_iter().find_map(|edit| match edit {
        TimelineEdit::ContextMenu { pointer, .. } => Some(pointer),
        _ => None,
    });
    assert!(matches!(click, Some(Some(pos)) if (pos - at).length() < 1.0), "{click:?}");
}

/// The rects of the panel that are filled with a clip colour or the disabled fill, as one frame paints them.
fn clip_fills(disabled: Vec<ClipId>) -> Vec<(Rect, egui::Color32)> {
    let sequence = Rc::new(RefCell::new(Sequence {
        tracks: named(&[(V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]),
        master: None,
        view: TimeView::new(Tick::ZERO, PANEL_POINTS_PER_SECOND),
        playhead: Tick::ZERO,
        selection: Vec::new(),
        disabled,
        edits: Vec::new(),
    }));
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let mut sequence = sequence.borrow_mut();
        Timeline::new("Timeline", &mut *sequence).show(ui);
    };
    for _ in 0..SETTLE_FRAMES {
        frames.frame(Vec::new(), &mut show);
    }
    frames
        .frame_shapes(&mut show)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => Some((rect.rect, rect.fill)),
            _ => None,
        })
        .collect()
}

/// Frames the panel takes to lay out and paint its clips.
const SETTLE_FRAMES: usize = 6;

/// A disabled clip paints with the theme's disabled fill, not its label colour, while an enabled clip keeps its hue.
#[trial]
fn a_disabled_clip_paints_with_the_disabled_fill() {
    let hue = craft_timeline::ui::LabelPalette::default().0[0];
    let of_hue = |fills: &[(Rect, egui::Color32)]| fills.iter().filter(|(_, fill)| *fill == hue).map(|(rect, _)| *rect).collect::<Vec<_>>();
    let all_on = of_hue(&clip_fills(Vec::new()));
    assert_eq!(all_on.len(), 2, "both clips paint in their hue: {all_on:?}");
    let fills = clip_fills(vec![ClipId(1)]);
    let kept = of_hue(&fills);
    assert_eq!(kept.len(), 1, "the enabled neighbour keeps its hue: {kept:?}");
    let gone = all_on.iter().find(|rect| !kept.contains(rect)).expect("one clip left its hue");
    let disabled = fills.iter().find(|(rect, _)| rect == gone).map(|(_, fill)| *fill);
    assert!(disabled.is_some_and(|fill| fill != hue), "the disabled clip's body is filled with another colour: {disabled:?}");
}
