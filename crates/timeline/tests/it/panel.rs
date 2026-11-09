//! The time area and the track stack: keys, wheel, follow and the move rules.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::{EditEvent, SelectionChange};
use craft_timeline::ui::stack::{MoveTarget, RowGeom, Side, TrackSizes, resolve_move};
use craft_timeline::ui::view::TimelineView;
use craft_timeline::ui::{Timeline, TimelineEdit};
use craft_timeline::{ClipId, ClipInfo, TimeRange, TimelineModel, TrackId, TrackInfo};
use craft_timeline::{FollowMode, FrameRate, Tick, TimeFormat, TimeView, TrackKind};
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Rect, Vec2};

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

// The Timeline panel over a model of tracks, one clip in V1 and one in A1.

const V1: TrackId = TrackId(1);
const V2: TrackId = TrackId(2);
const A1: TrackId = TrackId(3);
const C1: TrackId = TrackId(9);
/// The panel's room.
const PANEL_ROOM: Vec2 = Vec2::new(900.0, 500.0);
/// The time the panel's view shows per second of room.
const PANEL_POINTS_PER_SECOND: f64 = 20.0;

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

impl Panel {}

/// A row resizes from its own height: video rows grow upward, so an edge dragged up adds height, and the top video row
/// has a top edge.
#[trial]
fn rows_resize_from_their_height_and_video_rows_grow_upward() {
    let mut panel = panel();
    let a1 = panel.row("A1");
    let edge = panel.in_row("Drag to change the height of A1", a1);
    panel.fixture.drag(&edge, Vec2::new(0.0, RESIZE_TRAVEL));
    let got = panel.heights();
    assert_eq!(got.last().map(|(track, _)| *track), Some(A1), "{got:?}");
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (a1.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", a1.height());

    let mut panel = self::panel();
    let v1 = panel.row("V1");
    let edge = panel.in_row("Drag to change the height of V1", v1);
    panel.fixture.drag(&edge, Vec2::new(0.0, -RESIZE_TRAVEL));
    let got = panel.heights();
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (v1.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", v1.height());

    let mut panel = self::panel();
    let v2 = panel.row("V2");
    let top = panel.in_row("Drag the top edge to change the height of V2", v2);
    panel.fixture.drag(&top, Vec2::new(0.0, -RESIZE_TRAVEL));
    let got = panel.heights();
    assert_eq!(got.last().map(|(track, _)| *track), Some(V2), "{got:?}");
    assert!((got.last().map_or(0.0, |(_, height)| *height) - (v2.height() + RESIZE_TRAVEL)).abs() < SLACK, "{got:?} from {}", v2.height());
}

/// The distance a height edge is dragged in the resize trial.
const RESIZE_TRAVEL: f32 = 10.0;

/// The rounding slack of a measured point.
const SLACK: f32 = 0.5;

/// Captions sit above the video rows; a model with a master volume shows the Mix row below the audio rows.
#[trial]
fn caption_rows_sit_above_video_with_a_mix_row() {
    let panel = panel_with(named(&[(C1, TrackKind::Caption, "C1"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]), Some(-3.0));
    let (c, v1, a1) = (panel.row("C1"), panel.row("V1"), panel.row("A1"));
    assert!(c.bottom() <= v1.top() + SLACK, "{c:?} {v1:?}");
    assert!(v1.bottom() < a1.top(), "{v1:?} {a1:?}");
    let tokens = craft_ui::testing::style::tokens(craft_theme::Mode::Light, craft_theme::Contrast::Standard);
    let caption = TrackSizes::of(&tokens).caption;
    assert!((c.height() - caption).abs() < SLACK, "the caption row is its own height, not the pane's: {c:?} for {caption}");
    let mix = panel.fixture.by_label("Mix").expect("a model with a master volume has a Mix row");
    assert!(mix.rect.top() >= a1.bottom() - SLACK, "the Mix row lies below the audio rows: {:?} {a1:?}", mix.rect);
}

/// The context menus a pass reported, as (track, clip, anchor, pointer).
fn reported_menus(panel: &Panel) -> Vec<(Option<TrackId>, Option<ClipId>, Rect, Option<egui::Pos2>)> {
    panel
        .edits()
        .into_iter()
        .filter_map(|edit| match edit {
            TimelineEdit::ContextMenu { track, clip, anchor, pointer, .. } => Some((track, clip, anchor, pointer)),
            _ => None,
        })
        .collect()
}

/// The Menu key and Shift+F10 on a clicked clip report a context menu anchored on that clip; a secondary click on it
/// reports its position; the menu still anchors on the clip.
#[trial]
fn the_menu_keys_report_a_context_menu_anchored_on_the_focused_clip() {
    let mut panel = panel();
    let v1 = panel.row("V1");
    let clip = panel.in_row("Clip", v1);
    panel.fixture.click(&clip);
    panel.fixture.key(Key::Menu, Modifiers::NONE);
    panel.fixture.key(Key::F10, Modifiers::SHIFT);
    let menus = reported_menus(&panel);
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
    // Off the clip's centre, so the pointer and the clip's own anchor differ.
    let at = clip.rect.min + clip.rect.size() * 0.25;
    panel.fixture.frame(vec![pointer::moved(at)]);
    panel.fixture.frame(vec![pointer::secondary(at, true)]);
    panel.fixture.frame(vec![pointer::secondary(at, false)]);
    let menus = reported_menus(&panel);
    assert_eq!(menus.len(), 1, "{menus:?}");
    let (track, found, anchor, pointer) = menus[0];
    assert_eq!((track, found), (Some(V1), Some(ClipId(1))));
    assert!(pointer.is_some_and(|pointer| (pointer - at).length() < SLACK), "the pointer is the click position: {pointer:?} for {at:?}");
    assert!(
        (anchor.left() - clip.rect.left()).abs() < SLACK && (anchor.bottom() - clip.rect.bottom()).abs() < SLACK,
        "the menu anchors on the clip, not the click: {anchor:?} at {:?}",
        clip.rect
    );
}

/// The Tab stop of the time area comes first: one Tab reaches it, and Menu and Shift+F10 then report a context menu
/// anchored on the selection's last clip.
#[trial]
fn the_menu_keys_report_a_context_menu_for_the_clip_reached_by_tab() {
    let mut panel = panel();
    panel.sequence.borrow_mut().selection = vec![ClipId(1)];
    panel.fixture.idle(2);
    let v1 = panel.row("V1");
    let clip = panel.in_row("Clip", v1);
    panel.fixture.key(Key::Tab, Modifiers::NONE);
    let focused = panel.fixture.focused().and_then(|id| panel.fixture.tree().name(id).map(str::to_owned));
    assert_eq!(focused.as_deref(), Some("Timeline"), "one Tab reaches the time area");
    panel.fixture.key(Key::Menu, Modifiers::NONE);
    panel.fixture.key(Key::F10, Modifiers::SHIFT);
    let menus = reported_menus(&panel);
    assert_eq!(menus.len(), 2, "{menus:?}");
    for (track, found, anchor, pointer) in &menus {
        assert_eq!((*track, *found, *pointer), (Some(V1), Some(ClipId(1)), None));
        assert!(clip.rect.expand(SLACK).contains(anchor.center()), "{anchor:?} lies on the clip {:?}", clip.rect);
    }
}

/// A switched-off clip is inactive, not disabled: its node stays enabled and a click on it still selects it (Premiere).
#[trial]
fn an_inactive_clip_stays_selectable() {
    let mut panel = panel();
    panel.sequence.borrow_mut().disabled = vec![ClipId(1)];
    panel.fixture.idle(2);
    let clip = panel.in_row("Clip", panel.row("V1"));
    assert!(!panel.fixture.tree().is_disabled_key(clip.node), "the clip's node is enabled");
    panel.fixture.click(&clip);
    let picked =
        panel.edits().iter().any(|edit| matches!(edit, TimelineEdit::Select { change: SelectionChange::Set(clips), .. } if clips == &[ClipId(1)]));
    assert!(picked, "a click selects the inactive clip: {:?}", panel.edits());
}
