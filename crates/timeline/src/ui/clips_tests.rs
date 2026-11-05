//! The clip lane.

use craft_ui::behavior::SelectionChange;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};
use egui::{Event, Key, Modifiers, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::clips::{ClipLane, MAX_THUMBNAIL_TILES, ThumbnailMode, TimelineTool, thumbnail_tiles};
use crate::clips::{ClipEdge, ClipEdit, SnapTargets, TrimKind};
use crate::model::{ClipId, ClipInfo, TrackId};
use crate::rates::FrameRate;
use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView};

const A: ClipId = ClipId(1);
const B: ClipId = ClipId(2);

fn seconds(s: i64) -> Tick {
    Tick(s * TICKS_PER_SECOND)
}

struct App {
    clips: Vec<ClipInfo<'static>>,
    view: TimeView,
    selection: Vec<ClipId>,
    tool: TimelineTool,
    locked: bool,
    snap: Option<SnapTargets>,
    rect: Rect,
    edits: Vec<EditEvent<ClipEdit>>,
    selects: Vec<(SelectionChange<ClipId>, bool)>,
    snapped: Option<Tick>,
}

fn clip(id: ClipId, name: &'static str, from: i64, to: i64) -> ClipInfo<'static> {
    ClipInfo { id, track: TrackId(1), range: TimeRange::from_bounds(seconds(from), seconds(to)), name, label: None, enabled: true, source: None }
}

fn show(ui: &mut Ui, app: &mut App) {
    app.rect = Rect::from_min_size(ui.max_rect().min, vec2(400.0, 40.0));
    let mut lane = ClipLane::new("lane", TrackId(1), &app.clips, &app.view, FrameRate::FPS_24)
        .tool(app.tool)
        .locked(app.locked)
        .snap(app.snap.as_ref())
        .selection(&app.selection);
    if app.tool == TimelineTool::Selection {
        lane = lane.no_modifier_trims(false);
    }
    let out = lane.show(ui, app.rect);
    app.edits.extend(out.edit);
    app.snapped = out.snapped_at.or(app.snapped);
    if let Some((change, unlinked)) = out.select {
        app.selection = change.selection.items.clone();
        app.selects.push((change, unlinked));
    }
}

/// Clips A (0 to 10 s) and B (12 to 20 s) at 10 points a second.
fn harness() -> Harness<'static, App> {
    let app = App {
        clips: vec![clip(A, "A", 0, 10), clip(B, "B", 12, 20)],
        view: TimeView::new(Tick::ZERO, 10.0),
        selection: Vec::new(),
        tool: TimelineTool::Selection,
        locked: false,
        snap: None,
        rect: Rect::NOTHING,
        edits: Vec::new(),
        selects: Vec::new(),
        snapped: None,
    };
    let mut harness =
        Harness::<App>::builder().with_size([480.0, 120.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(show, app);
    harness.run();
    harness
}

/// The point `x` lane points from the lane's left edge, in the middle of its height.
fn at(harness: &Harness<'_, App>, x: f32) -> Pos2 {
    let rect = harness.state().rect;
    pos2(rect.left() + x, rect.center().y)
}

fn press(harness: &mut Harness<'_, App>, x: f32) {
    let pos = at(harness, x);
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
}

fn drag_to(harness: &mut Harness<'_, App>, x: f32) {
    let to = at(harness, x);
    harness.hover_at(to);
    harness.step();
}

fn release(harness: &mut Harness<'_, App>, x: f32) {
    let pos = at(harness, x);
    harness.drop_at(pos);
    harness.step();
}

fn hold(harness: &mut Harness<'_, App>, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
}

fn drag(harness: &mut Harness<'_, App>, from: f32, to: f32) {
    press(harness, from);
    drag_to(harness, (from + to) / 2.0);
    drag_to(harness, to);
    release(harness, to);
}

/// The value of the last commit.
fn committed(harness: &Harness<'_, App>) -> Option<ClipEdit> {
    harness.state().edits.iter().rev().find_map(|event| match event {
        EditEvent::Commit { value, .. } => Some(value.clone()),
        _ => None,
    })
}

// Head, HeadAndTail and Continuous tiles, capped at 256, only visible ones.
#[test]
fn thumbnail_tiles_follow_the_mode() {
    let duration = seconds(10);
    let frame = Tick(TICKS_PER_SECOND / 24);
    let all = (0.0, 1000.0);
    assert!(thumbnail_tiles(ThumbnailMode::Off, 0.0, 100.0, 20.0, all, duration, frame).is_empty());
    assert_eq!(thumbnail_tiles(ThumbnailMode::Head, 0.0, 100.0, 20.0, all, duration, frame), vec![(0.0, Tick::ZERO)]);
    let ends = thumbnail_tiles(ThumbnailMode::HeadAndTail, 0.0, 100.0, 20.0, all, duration, frame);
    assert_eq!(ends, vec![(0.0, Tick::ZERO), (80.0, duration - frame)]);
    // too narrow for two tiles: the head alone
    assert_eq!(thumbnail_tiles(ThumbnailMode::HeadAndTail, 0.0, 30.0, 20.0, all, duration, frame).len(), 1);
    let tiles = thumbnail_tiles(ThumbnailMode::Continuous, 0.0, 100.0, 20.0, all, duration, frame);
    assert_eq!(tiles.iter().map(|(x, _)| *x).collect::<Vec<_>>(), [0.0, 20.0, 40.0, 60.0, 80.0]);
    assert_eq!(tiles[2].1, seconds(4));
    // only the tiles that meet the visible range
    let visible = thumbnail_tiles(ThumbnailMode::Continuous, 0.0, 100.0, 20.0, (45.0, 65.0), duration, frame);
    assert_eq!(visible.iter().map(|(x, _)| *x).collect::<Vec<_>>(), [40.0, 60.0]);
    let long = thumbnail_tiles(ThumbnailMode::Continuous, 0.0, 100_000.0, 20.0, (0.0, 100_000.0), duration, frame);
    assert_eq!(long.len(), MAX_THUMBNAIL_TILES);
}

// an edge drag reports Trim, Ripple or Roll by tool and modifiers.
#[test]
fn dragging_an_edge_trims_with_the_tools_kind() {
    let cases = [
        (TimelineTool::Selection, Modifiers::NONE, TrimKind::Trim),
        (TimelineTool::Selection, Modifiers::COMMAND, TrimKind::Ripple),
        (TimelineTool::Ripple, Modifiers::NONE, TrimKind::Ripple),
        (TimelineTool::RateStretch, Modifiers::NONE, TrimKind::Stretch),
    ];
    for (tool, modifiers, kind) in cases {
        let mut harness = harness();
        harness.state_mut().tool = tool;
        hold(&mut harness, modifiers);
        // 3 points inside A's end, 2 s to the right
        drag(&mut harness, 97.0, 117.0);
        let expected = ClipEdit::Trim { clip: A, edge: ClipEdge::Out, kind, delta: seconds(2), linked: true };
        assert_eq!(committed(&harness), Some(expected), "{tool:?} {modifiers:?}");
    }

    // B starts 2 s after A ends: pull B's start back across the gap to roll nothing; but with
    // the clips touching the Rolling tool rolls the cut.
    let mut harness = harness();
    harness.state_mut().clips[1] = clip(B, "B", 10, 20);
    harness.state_mut().tool = TimelineTool::Rolling;
    drag(&mut harness, 97.0, 77.0);
    assert_eq!(committed(&harness), Some(ClipEdit::Roll { left: A, right: B, delta: -seconds(2) }));
}

// a body drag reports Move with the snapped delta; Alt copies.
#[test]
fn dragging_a_body_moves_and_snaps() {
    let mut harness = harness();
    let mut targets = SnapTargets::new();
    targets.push(seconds(12));
    harness.state_mut().snap = Some(targets);
    // 11.7 s to the right is 11.708 s at 24 fps; B's start at 12 s is 2.9 points away
    drag(&mut harness, 50.0, 167.0);
    let moved = ClipEdit::Move { clips: vec![A], delta: seconds(12), track_delta: 0, new_track: false, copy: false };
    assert_eq!(committed(&harness), Some(moved));
    assert_eq!(harness.state().snapped, Some(seconds(12)));
    // the pressed clip was not selected: the drag selects it
    assert_eq!(harness.state().selection, vec![A]);

    let mut harness = self::harness();
    harness.state_mut().selection = vec![A, ClipId(9)];
    press(&mut harness, 50.0);
    hold(&mut harness, Modifiers::ALT);
    drag_to(&mut harness, 70.0);
    drag_to(&mut harness, 90.0);
    release(&mut harness, 90.0);
    let copied = ClipEdit::Move { clips: vec![A, ClipId(9)], delta: seconds(4), track_delta: 0, new_track: false, copy: true };
    assert_eq!(committed(&harness), Some(copied));
}

// Esc reports Cancel and the release after it does nothing.
#[test]
fn escape_cancels_a_drag_and_ignores_the_release() {
    let mut harness = harness();
    press(&mut harness, 50.0);
    drag_to(&mut harness, 70.0);
    drag_to(&mut harness, 90.0);
    harness.key_press(Key::Escape);
    harness.step();
    drag_to(&mut harness, 130.0);
    release(&mut harness, 130.0);
    let edits = &harness.state().edits;
    assert!(matches!(edits.first(), Some(EditEvent::Begin { .. })), "{edits:?}");
    assert!(matches!(edits.last(), Some(EditEvent::Cancel { .. })), "{edits:?}");
    assert!(edits.iter().all(|event| !matches!(event, EditEvent::Commit { .. })), "{edits:?}");
}

// a locked lane reports no edit.
#[test]
fn locked_tracks_start_no_gesture() {
    let mut harness = harness();
    harness.state_mut().locked = true;
    drag(&mut harness, 50.0, 150.0);
    drag(&mut harness, 97.0, 117.0);
    assert!(harness.state().edits.is_empty());
    assert!(harness.state().selects.is_empty());
}

// a click replaces the selection, Shift toggles, Cmd toggles without partners, empty space clears.
#[test]
fn clicks_pick_clips_and_clear_on_empty_space() {
    let mut harness = harness();
    let click = |harness: &mut Harness<'_, App>, x: f32, modifiers: Modifiers| {
        hold(harness, modifiers);
        press(harness, x);
        release(harness, x);
    };
    click(&mut harness, 50.0, Modifiers::NONE);
    assert_eq!(harness.state().selection, vec![A]);
    click(&mut harness, 150.0, Modifiers::SHIFT);
    assert_eq!(harness.state().selection, vec![A, B]);
    assert_eq!(harness.state().selects.last().map(|(_, unlinked)| *unlinked), Some(false));
    click(&mut harness, 50.0, Modifiers::COMMAND);
    assert_eq!(harness.state().selection, vec![B]);
    assert_eq!(harness.state().selects.last().map(|(_, unlinked)| *unlinked), Some(true));
    click(&mut harness, 110.0, Modifiers::NONE);
    assert!(harness.state().selection.is_empty());
    assert!(harness.state().edits.is_empty());

    // a reader's Click on a clip's button selects it
    harness.get_by_label("B").click();
    harness.run();
    assert_eq!(harness.state().selection, vec![B]);
}

// a track's colour label tints its whole lane by a tenth (FilmCraft `TRACK_LANE_TINT`).
#[test]
fn the_track_label_tints_the_lane() {
    use crate::markers::MarkerColor;
    use egui::epaint::Shape;
    use egui::{Context, RawInput};

    let palette = super::LabelPalette::default();
    let wanted = palette.color(MarkerColor(2)).gamma_multiply(0.10);
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 40.0));
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let clips = [clip(A, "A", 0, 10)];
    let view = TimeView::new(Tick::ZERO, 10.0);
    let tinted = |label: Option<MarkerColor>| {
        let mut output = ctx.run_ui(RawInput { screen_rect: Some(rect.expand(10.0)), ..RawInput::default() }, |ui| {
            ClipLane::new("lane", TrackId(1), &clips, &view, FrameRate::FPS_24).palette(&palette).track_label(label).show(ui, rect);
        });
        output.textures_delta.clear();
        output.shapes.iter().any(|clipped| matches!(&clipped.shape, Shape::Rect(shape) if shape.rect == rect && shape.fill == wanted))
    };
    assert!(tinted(Some(MarkerColor(2))));
    assert!(!tinted(None), "no label, no tint");
}
