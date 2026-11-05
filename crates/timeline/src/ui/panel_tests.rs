//! The timeline panel.

use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, TouchPhase, Ui, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::panel::{Timeline, TimelineEdit, TrackSwitch};
use crate::clips::ClipEdit;
use crate::clock::FollowMode;
use crate::model::{ClipId, ClipInfo, TimelineModel, TrackId, TrackInfo, TrackKind};
use crate::rates::FrameRate;
use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView};
use crate::timecode::TimeFormat;

fn seconds(s: i64) -> Tick {
    Tick(s * TICKS_PER_SECOND)
}

const V1: TrackId = TrackId(1);
const V2: TrackId = TrackId(2);
const A1: TrackId = TrackId(3);

struct Model {
    names: Vec<(TrackId, TrackKind, &'static str)>,
    clips: Vec<(ClipId, TrackId, TimeRange)>,
    view: TimeView,
    playhead: Tick,
    selection: Vec<ClipId>,
    master: Option<f32>,
}

impl TimelineModel for Model {
    fn duration(&self) -> Tick {
        seconds(60)
    }

    fn format(&self) -> TimeFormat {
        TimeFormat::timecode(FrameRate::FPS_24)
    }

    fn playhead(&self) -> Tick {
        self.playhead
    }

    fn view(&mut self) -> &mut TimeView {
        &mut self.view
    }

    fn tracks(&self) -> Vec<TrackInfo<'_>> {
        self.names
            .iter()
            .map(|&(id, kind, name)| TrackInfo {
                id,
                kind,
                name,
                height: None,
                depth: 0,
                label: None,
                locked: Some(false),
                targeted: None,
                sync_lock: None,
                enabled: None,
                muted: (kind == TrackKind::Audio).then_some(false),
                solo: (kind == TrackKind::Audio).then_some(false),
            })
            .collect()
    }

    fn clips(&self, track: TrackId, _visible: TimeRange) -> Vec<ClipInfo<'_>> {
        self.clips
            .iter()
            .filter(|(_, t, _)| *t == track)
            .map(|&(id, track, range)| ClipInfo { id, track, range, name: "clip", label: None, enabled: true, source: None })
            .collect()
    }

    fn selection(&self) -> &[ClipId] {
        &self.selection
    }

    fn master_volume_db(&self) -> Option<f32> {
        self.master
    }
}

struct App {
    model: Model,
    edits: Vec<TimelineEdit>,
    follow: FollowMode,
    playing: bool,
    time_area: Rect,
    painted: usize,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = Timeline::new("timeline", &mut app.model).follow(app.follow, app.playing).content(|_, _, _| app.painted += 1).show(ui);
    app.time_area = out.time_area;
    app.edits.extend(out.edits);
}

/// V2 and V1 over A1; a clip in V1 and one in A1 (2 s to 6 s) at 20 points a second.
fn harness() -> Harness<'static, App> {
    harness_with(vec![(V2, TrackKind::Video, "V2"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")], None)
}

fn harness_with(names: Vec<(TrackId, TrackKind, &'static str)>, master: Option<f32>) -> Harness<'static, App> {
    let model = Model {
        names,
        master,
        clips: vec![(ClipId(1), V1, TimeRange::from_bounds(seconds(2), seconds(6))), (ClipId(2), A1, TimeRange::from_bounds(seconds(2), seconds(6)))],
        view: TimeView::new(Tick::ZERO, 20.0),
        playhead: Tick::ZERO,
        selection: Vec::new(),
    };
    let app = App { model, edits: Vec::new(), follow: FollowMode::Page, playing: false, time_area: Rect::NOTHING, painted: 0 };
    let mut harness = Harness::<App>::builder()
        .with_size([900.0, 500.0])
        // Two clicks of a double-click must land within egui's double-click delay.
        .with_step_dt(0.05)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
        })
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn row(harness: &Harness<'_, App>, name: &str) -> Rect {
    harness.get_by_role_and_label(Role::Row, name).rect()
}

/// The point `seconds` into the view at height `y`.
fn point(harness: &Harness<'_, App>, seconds: f64, y: f32) -> Pos2 {
    Pos2::new(harness.state().time_area.left() + (seconds * 20.0) as f32, y)
}

fn drag(harness: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    harness.hover_at(from.lerp(to, 0.5));
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.step();
}

/// Drags the clip at 4 s from height `from` to height `to`.
fn drag_y(harness: &mut Harness<'_, App>, from: f32, to: f32) {
    let (a, b) = (point(harness, 4.0, from), point(harness, 4.0, to));
    drag(harness, a, b);
}

fn clip_events(harness: &Harness<'_, App>) -> Vec<EditEvent<ClipEdit>> {
    harness.state().edits.iter().filter_map(|edit| if let TimelineEdit::Clip(event) = edit { Some(event.clone()) } else { None }).collect()
}

fn committed(harness: &Harness<'_, App>) -> Option<ClipEdit> {
    clip_events(harness).into_iter().rev().find_map(|event| match event {
        EditEvent::Commit { value, .. } => Some(value),
        _ => None,
    })
}

// V1 sits just above the divider and V2 above it; audio goes down.
#[test]
fn video_tracks_stack_up_from_the_divider() {
    let harness = harness();
    let (v2, v1, a1) = (row(&harness, "V2"), row(&harness, "V1"), row(&harness, "A1"));
    assert!(v2.bottom() <= v1.top() + 0.5, "{v2:?} {v1:?}");
    assert!(v1.bottom() < a1.top(), "the divider lies between V1 and A1: {v1:?} {a1:?}");
    assert_eq!(v2.height(), v1.height());
}

// a clip drag in a row reports TimelineEdit::Clip events.
#[test]
fn clip_drags_come_out_as_timeline_edits() {
    let mut harness = harness();
    let y = row(&harness, "V1").center().y;
    let (from, to) = (point(&harness, 4.0, y), point(&harness, 7.0, y));
    drag(&mut harness, from, to);
    let events = clip_events(&harness);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    match committed(&harness) {
        Some(ClipEdit::Move { delta, track_delta: 0, new_track: false, .. }) => assert_eq!(delta, seconds(3)),
        other => panic!("{other:?}"),
    }
}

// The lane reports a move with `track_delta: 0`; the panel derives the tracks from the pointer and
// its rows: another row of the kind, or the empty space past the last one adds a track.
#[test]
fn a_move_gets_its_track_fields_from_the_pointer() {
    let mut harness = harness();
    let (v2, v1, a1) = (row(&harness, "V2"), row(&harness, "V1"), row(&harness, "A1"));

    // Straight up into V2: one track, with no time change.
    drag_y(&mut harness, v1.center().y, v2.center().y);
    assert!(matches!(committed(&harness), Some(ClipEdit::Move { track_delta: 1, new_track: false, .. })), "{:?}", committed(&harness));

    // Above V2: a new video track, one past V2.
    harness.state_mut().edits.clear();
    let above = harness.state().time_area.top() + (v2.top() - harness.state().time_area.top()) / 2.0;
    drag_y(&mut harness, v1.center().y, above);
    assert!(matches!(committed(&harness), Some(ClipEdit::Move { track_delta: 2, new_track: true, .. })), "{:?}", committed(&harness));

    // Into the audio rows: no change of track.
    harness.state_mut().edits.clear();
    drag_y(&mut harness, v1.center().y, a1.center().y);
    assert!(matches!(committed(&harness), Some(ClipEdit::Move { track_delta: 0, new_track: false, .. })), "{:?}", committed(&harness));

    // Below the last audio track: a new audio track.
    harness.state_mut().edits.clear();
    let below = a1.bottom() + (harness.state().time_area.bottom() - a1.bottom()).min(a1.height()) / 2.0;
    drag_y(&mut harness, a1.center().y, below);
    assert!(matches!(committed(&harness), Some(ClipEdit::Move { track_delta: 1, new_track: true, .. })), "{:?}", committed(&harness));
}

// clicking Lock reports Switch { Locked, true }; Alt-click on Solo is exclusive.
#[test]
fn header_switches_report_their_new_state() {
    let mut harness = harness();
    harness.get_by_label("Toggle Track Lock V1").click();
    harness.run();
    harness.get_by_label("Solo Track A1").click_modifiers(Modifiers::ALT);
    harness.run();
    let switches: Vec<_> = harness
        .state()
        .edits
        .iter()
        .filter_map(|edit| match edit {
            TimelineEdit::Switch { track, switch, value, exclusive } => Some((*track, *switch, *value, *exclusive)),
            _ => None,
        })
        .collect();
    assert_eq!(switches, [(V1, TrackSwitch::Locked, true, false), (A1, TrackSwitch::Solo, true, true)]);
}

// Alt+wheel zooms about the pointer, = and - about the playhead.
#[test]
fn wheel_and_keys_zoom_about_the_pointer_and_playhead() {
    let mut harness = harness();
    let y = row(&harness, "V1").center().y;
    let pointer = point(&harness, 10.0, y);
    harness.hover_at(pointer);
    harness.step();
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, 100.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::ALT,
        source: MouseWheelSource::Unknown,
    });
    harness.run_steps(3);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    let view = harness.state().model.view;
    assert!(view.points_per_second > 20.0, "{view:?}");
    let under = view.time_at(pointer.x, harness.state().time_area.left());
    assert!((under.seconds() - 10.0).abs() < 0.05, "the time under the pointer stays: {under:?}");

    harness.state_mut().model.view = TimeView::new(Tick::ZERO, 20.0);
    harness.state_mut().model.playhead = seconds(5);
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Group, "Timeline").focus();
    harness.run_steps(2);
    harness.press(egui::KeyboardShortcut::new(Modifiers::NONE, Key::Equals));
    harness.run_steps(2);
    let view = harness.state().model.view;
    assert!((view.points_per_second - 40.0).abs() < 0.01, "{view:?}");
    let x = view.x_of(seconds(5), 0.0);
    assert!((x - 100.0).abs() < 0.5, "the playhead keeps its x: {x}");
    harness.press(egui::KeyboardShortcut::new(Modifiers::NONE, Key::Minus));
    harness.run_steps(2);
    assert!((harness.state().model.view.points_per_second - 20.0).abs() < 0.01);
}

// with Page follow the view jumps when the playhead passes 95 %.
#[test]
fn view_follows_the_playhead_while_playing() {
    let mut harness = harness();
    let width = harness.state().time_area.width();
    harness.state_mut().playing = true;
    harness.state_mut().model.playhead = Tick::from_seconds_f64(f64::from(width) * 0.5 / 20.0);
    harness.run_steps(2);
    assert_eq!(harness.state().model.view.start, Tick::ZERO, "in view: no jump");
    let late = Tick::from_seconds_f64(f64::from(width) * 0.96 / 20.0);
    harness.state_mut().model.playhead = late;
    harness.run_steps(2);
    let view = harness.state().model.view;
    assert!(view.start > Tick::ZERO, "{view:?}");
    let x = view.x_of(late, 0.0);
    assert!((x - width * 0.05).abs() < 1.0, "the playhead lands 5 % from the left: {x}");
}

fn heights(harness: &Harness<'_, App>) -> Vec<(TrackId, f32)> {
    harness
        .state()
        .edits
        .iter()
        .filter_map(|edit| match edit {
            TimelineEdit::TrackHeight { track, height: EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } } => Some((*track, *value)),
            _ => None,
        })
        .collect()
}

/// A point in a row's header, clear of its switches.
fn header_point(row: Rect, y: f32) -> Pos2 {
    Pos2::new(row.left() + 40.0, y)
}

// A row at the style's default height resizes from that height, not from a collapsed one.
#[test]
fn dragging_a_default_height_rows_edge_starts_from_its_height() {
    let mut harness = harness();
    let a1 = row(&harness, "A1");
    let from = header_point(a1, a1.bottom());
    harness.drag(from, from + vec2(0.0, 10.0), Modifiers::NONE);
    let got = heights(&harness);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, A1);
    assert!((got[0].1 - (a1.height() + 10.0)).abs() < 0.5, "{} from {}", got[0].1, a1.height());
}

// Video rows grow upward: dragging an edge up adds height, and the top video row has a top edge.
#[test]
fn video_rows_resize_with_negated_travel_and_the_top_row_has_a_top_edge() {
    let mut harness = harness();
    let v1 = row(&harness, "V1");
    let from = header_point(v1, v1.bottom());
    harness.drag(from, from - vec2(0.0, 10.0), Modifiers::NONE);
    let got = heights(&harness);
    assert!(got.len() == 1 && got[0].0 == V1 && (got[0].1 - (v1.height() + 10.0)).abs() < 0.5, "{got:?}");

    harness.state_mut().edits.clear();
    let v2 = row(&harness, "V2");
    let from = header_point(v2, v2.top());
    harness.drag(from, from - vec2(0.0, 10.0), Modifiers::NONE);
    let got = heights(&harness);
    assert!(got.len() == 1 && got[0].0 == V2 && (got[0].1 - (v2.height() + 10.0)).abs() < 0.5, "{got:?}");
}

// A double-click on a header toggles the height between 64 and 30 points (FilmCraft).
#[test]
fn double_clicking_a_header_toggles_its_height() {
    let mut harness = harness();
    let a1 = row(&harness, "A1");
    let at = Pos2::new(a1.left() + 60.0, a1.top() + 3.0);
    for frame in events::click(at, PointerButton::Primary, 2, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
    let got = heights(&harness);
    let expected = if a1.height() < 50.0 { 64.0 } else { 30.0 };
    assert_eq!(got, [(A1, expected)]);
}

// The colour strip and the two edges take the pointer only: no Tab stop reaches them, and each
// edge has a name.
#[test]
fn tab_order_skips_the_pointer_only_parts() {
    let mut harness = harness();
    let mut seen = Vec::new();
    for _ in 0..60 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness.root().children_recursive().find(|node| node.accesskit_node().is_focused());
        if let Some(node) = focused {
            seen.push(format!("{:?}", node.accesskit_node().label()));
        }
    }
    assert!(seen.iter().any(|label| label.contains("Toggle Track Lock V1")), "{seen:?}");
    assert!(seen.iter().all(|label| !label.contains("Track Color") && !label.contains("Drag to change")), "{seen:?}");
    harness.get_by_label("Drag to change the height of V1");
    harness.get_by_label("Drag the top edge to change the height of V2");
    harness.get_by_label("Drag to change the width of the track headers");
}

// Timeline::content reaches every lane's clips.
#[test]
fn content_is_painted_for_the_lanes_clips() {
    let harness = harness();
    assert!(harness.state().painted >= 2, "{}", harness.state().painted);
}

// Captions have their own area above the video rows; a model with a master volume shows the Mix row.
#[test]
fn caption_rows_sit_above_video_with_a_mix_row() {
    let c1 = TrackId(9);
    let harness = harness_with(vec![(c1, TrackKind::Caption, "C1"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")], Some(-3.0));
    let (c, v1, a1) = (row(&harness, "C1"), row(&harness, "V1"), row(&harness, "A1"));
    assert!(c.bottom() < v1.top(), "{c:?} {v1:?}");
    assert!(v1.bottom() < a1.top() && a1.bottom() <= harness.state().time_area.bottom(), "{v1:?} {a1:?}");
}
