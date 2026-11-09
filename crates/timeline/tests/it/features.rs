//! The whole panel: clip drags across tracks, the marquee, the snap line, the divider and the label menu.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::{EditEvent, SelectionChange};
use craft_timeline::ui::{Timeline, TimelineEdit};
use craft_timeline::{
    ClipEdit, ClipId, ClipInfo, FrameRate, Marker, Tick, TimeFormat, TimeRange, TimeView, TimelineModel, TrackId, TrackInfo, TrackKind,
};
use craft_ui::contract::Role;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, vec2};

const POINTS_PER_SECOND: f64 = 40.0;
const V2: TrackId = TrackId(1);
const V1: TrackId = TrackId(2);
const A1: TrackId = TrackId(3);
const ALPHA: ClipId = ClipId(1);
const BETA: ClipId = ClipId(2);
const GAMMA: ClipId = ClipId(3);
/// The share of the room the video side has until the app keeps one.
const DEFAULT_SHARE: f32 = 0.5;
/// The name of the split area's divider.
const DIVIDER: &str = "Drag to change how the room is shared between video and audio tracks";

fn seconds(at: f64) -> Tick {
    Tick::from_seconds_f64(at)
}

fn span(from: f64, to: f64) -> TimeRange {
    TimeRange::from_bounds(seconds(from), seconds(to))
}

struct Model {
    clips: Vec<(ClipId, TrackId, TimeRange, &'static str)>,
    view: TimeView,
    selection: Vec<ClipId>,
    locked: Vec<TrackId>,
    markers: Vec<Marker>,
}

impl TimelineModel for Model {
    fn duration(&self) -> Tick {
        seconds(60.0)
    }

    fn format(&self) -> TimeFormat {
        TimeFormat::timecode(FrameRate::FPS_24)
    }

    fn playhead(&self) -> Tick {
        Tick::ZERO
    }

    fn view(&mut self) -> &mut TimeView {
        &mut self.view
    }

    fn tracks(&self) -> Vec<TrackInfo<'_>> {
        [(V2, TrackKind::Video, "V2"), (V1, TrackKind::Video, "V1"), (A1, TrackKind::Audio, "A1")]
            .into_iter()
            .map(|(id, kind, name)| TrackInfo {
                id,
                kind,
                name,
                height: None,
                depth: 0,
                label: None,
                locked: Some(self.locked.contains(&id)),
                targeted: None,
                sync_lock: None,
                enabled: None,
                muted: None,
                solo: None,
            })
            .collect()
    }

    fn clips(&self, track: TrackId, visible: TimeRange) -> Vec<ClipInfo<'_>> {
        self.clips
            .iter()
            .filter(|(_, on, range, _)| *on == track && range.overlaps(&visible))
            .map(|&(id, track, range, name)| ClipInfo { id, track, range, name, label: None, enabled: true, source: None })
            .collect()
    }

    fn selection(&self) -> &[ClipId] {
        &self.selection
    }

    fn markers(&self) -> &[Marker] {
        &self.markers
    }
}

#[derive(Default)]
struct Seen {
    edits: Vec<TimelineEdit>,
    divider: Option<f32>,
}

/// V2 over V1 over A1; alpha on V1 and beta on A1 (2 s to 6 s), gamma on V2 (9 s to 12 s).
fn panel(locked: Vec<TrackId>, selection: Vec<ClipId>) -> (Fixture, Rc<RefCell<Model>>, Rc<RefCell<Seen>>) {
    let model = Rc::new(RefCell::new(Model {
        clips: vec![(ALPHA, V1, span(2.0, 6.0), "alpha"), (BETA, A1, span(2.0, 6.0), "beta"), (GAMMA, V2, span(9.0, 12.0), "gamma")],
        view: TimeView::new(Tick::ZERO, POINTS_PER_SECOND),
        selection,
        locked,
        markers: Vec::new(),
    }));
    let seen = Rc::new(RefCell::new(Seen::default()));
    let (shown_model, shown_seen) = (model.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (mut model, mut seen) = (shown_model.borrow_mut(), shown_seen.borrow_mut());
        let share = seen.divider;
        let mut timeline = Timeline::new("Timeline", &mut *model);
        if let Some(share) = share {
            timeline = timeline.divider(share);
        }
        let out = timeline.show(ui);
        for edit in &out.edits {
            match edit {
                TimelineEdit::Select { change: SelectionChange::Set(clips), .. } => model.selection = clips.clone(),
                TimelineEdit::Select { change: SelectionChange::Clear, .. } => model.selection.clear(),
                TimelineEdit::Divider(event) => seen.divider = event.value().copied().or(seen.divider),
                _ => {}
            }
        }
        seen.edits.extend(out.edits);
    });
    fixture.room(vec2(900.0, 700.0));
    fixture.settle();
    (fixture, model, seen)
}

/// The rects of the track rows by name: V2, V1, A1.
fn rows(fixture: &Fixture) -> Vec<Rect> {
    let tree = fixture.tree();
    ["V2", "V1", "A1"]
        .into_iter()
        .filter_map(|name| fixture.by_role(Role::ROW).into_iter().find(|row| tree.name_of(row.node) == Some(name)).map(|row| row.rect))
        .collect()
}

fn clip_rect(fixture: &Fixture, name: &str) -> Rect {
    fixture.by_label(name).unwrap_or_else(|| panic!("clip {name} is in view")).rect
}

fn clip_events(seen: &Seen) -> Vec<EditEvent<ClipEdit>> {
    seen.edits.iter().filter_map(|edit| if let TimelineEdit::Clip(event) = edit { Some(event.clone()) } else { None }).collect()
}

/// The track fields of a move edit.
fn track_fields(edit: &ClipEdit) -> Option<(i32, bool)> {
    match edit {
        ClipEdit::Move { track_delta, new_track, .. } => Some((*track_delta, *new_track)),
        _ => None,
    }
}

fn committed(seen: &Seen) -> Option<(i32, bool)> {
    clip_events(seen).iter().rev().find_map(|event| match event {
        EditEvent::Commit { after, .. } => track_fields(after),
        _ => None,
    })
}

/// A clip dragged to another track of its kind moves there, and the move shows the change while it runs. Over the other
/// kind it stays, and past the last track of its kind a drop adds one.
#[trial]
fn a_clip_dragged_to_another_track_of_its_kind_moves_there() {
    let (mut fixture, _, seen) = panel(Vec::new(), Vec::new());
    let rows = rows(&fixture);
    assert_eq!(rows.len(), 3, "V2, V1 and A1 are rows: {rows:?}");
    let alpha = clip_rect(&fixture, "alpha");

    let to = |y: f32| alpha.center() + vec2(0.0, y - alpha.center().y);
    fixture.drag_at(alpha.center(), to(rows[0].center().y) - alpha.center());
    assert_eq!(committed(&seen.borrow()), Some((1, false)), "V1 to V2 is one row away from the divider");
    let previewed =
        clip_events(&seen.borrow()).iter().any(|event| matches!(event, EditEvent::Update { value, .. } if track_fields(value) == Some((1, false))));
    assert!(previewed, "the track change shows before the drop");

    seen.borrow_mut().edits.clear();
    let alpha = clip_rect(&fixture, "alpha");
    fixture.drag_at(alpha.center(), to(rows[2].center().y) - alpha.center());
    assert_eq!(committed(&seen.borrow()), Some((0, false)), "a video clip over an audio row stays");

    seen.borrow_mut().edits.clear();
    let beta = clip_rect(&fixture, "beta");
    fixture.drag_at(beta.center(), vec2(0.0, rows[2].bottom() + 6.0 - beta.center().y));
    assert_eq!(committed(&seen.borrow()), Some((0, true)), "below the last audio track a drop adds one");
}

/// A move onto a locked track is refused and stays on its track.
#[trial]
fn a_move_onto_a_locked_track_is_refused() {
    let (mut fixture, _, seen) = panel(vec![V2], Vec::new());
    let rows = rows(&fixture);
    let alpha = clip_rect(&fixture, "alpha");
    fixture.drag_at(alpha.center(), vec2(0.0, rows[0].center().y - alpha.center().y));
    assert_eq!(committed(&seen.borrow()), Some((0, false)));
}

/// A drag from empty lane space sweeps a marquee over the clips it touches on every track. Shift adds them to the
/// selection; Escape restores the selection the drag started from.
#[trial]
fn a_marquee_selects_the_clips_it_touches_across_tracks() {
    let (mut fixture, model, _) = panel(Vec::new(), Vec::new());
    let rows = rows(&fixture);
    let (alpha, beta) = (clip_rect(&fixture, "alpha"), clip_rect(&fixture, "beta"));
    let from = Pos2::new(alpha.left() - 30.0, rows[0].center().y);
    let to = Pos2::new(beta.right() - 20.0, rows[2].center().y);
    fixture.drag_at(from, to - from);
    let mut selected = model.borrow().selection.clone();
    selected.sort();
    assert_eq!(selected, vec![ALPHA, BETA], "V1 and A1 clips are swept, the empty V2 lane adds none");

    let (mut fixture, model, _) = panel(Vec::new(), vec![GAMMA]);
    fixture.hold(Modifiers::SHIFT);
    fixture.drag_at(from, to - from);
    let mut selected = model.borrow().selection.clone();
    selected.sort();
    assert_eq!(selected, vec![ALPHA, BETA, GAMMA], "Shift keeps the selection the drag started from");

    let (mut fixture, model, _) = panel(Vec::new(), vec![GAMMA]);
    fixture.drag_at_pausing(from, to - from, |fixture| {
        assert_eq!(model.borrow().selection.len(), 2, "the selection follows the marquee while it runs");
        fixture.key(Key::Escape, Modifiers::NONE);
    });
    assert_eq!(model.borrow().selection, vec![GAMMA], "Escape restores the starting selection");
}

/// A drag that snaps shows its target as a line across the tracks while it runs, and the line goes with the drag.
#[trial]
fn the_snap_target_shows_as_a_line_while_dragging() {
    let (mut fixture, _, _) = panel(Vec::new(), Vec::new());
    let alpha = clip_rect(&fixture, "alpha");
    let at = |time: f64| alpha.left() + ((time - 2.0) * POINTS_PER_SECOND) as f32;
    // Alpha's end meets the start of gamma (9 s) a point short of it.
    let by = vec2(((9.0 - 6.0) * POINTS_PER_SECOND) as f32 - 1.0, 0.0);
    let lines = |fixture: &Fixture| {
        let tall = alpha.height() * 2.0;
        fixture
            .shape_bounds()
            .iter()
            .filter(|bounds| bounds.width() < 4.0 && bounds.height() > tall && (bounds.center().x - at(9.0)).abs() < 1.5)
            .count()
    };
    fixture.drag_at_pausing(alpha.center(), by, |fixture| {
        assert!(lines(fixture) > 0, "a line stands at the snap target");
    });
    assert_eq!(lines(&fixture), 0, "the line is gone with the drag");
}

/// Dragging the divider changes the video side's share, and the share the app keeps lays the sides out.
#[trial]
fn the_divider_changes_and_keeps_the_share() {
    let (mut fixture, _, seen) = panel(Vec::new(), Vec::new());
    let before = rows(&fixture)[2].top();
    let divider = fixture.by_label(DIVIDER).expect("the divider is named");
    fixture.drag(&divider, vec2(0.0, -divider.rect.height()));
    fixture.idle(2);
    let share = seen.borrow().divider;
    let share = share.unwrap_or_else(|| panic!("the drag reports a share: the divider is at {:?}, the rows at {:?}", divider.rect, rows(&fixture)));
    assert!(share < DEFAULT_SHARE, "dragging up gives the video side less: {share}");
    let after = rows(&fixture)[2].top();
    assert!(after < before, "the audio side moved up with the divider: {before} -> {after}");
}
