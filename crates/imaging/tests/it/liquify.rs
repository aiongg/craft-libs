//! Tests of the Liquify workspace: its session, tools and keys, and the canvas gestures.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::edit_history::Step;
use craft_behavior::{CancelReason, Cursor, PaintedCursor, PixelSize};
use craft_host::target::Os;
use craft_imaging::ui::liquify::{
    BackdropLayering, BackdropMode, BackdropUse, LayeredImage, LiquifyBrush, LiquifyCommands, LiquifyEvent, LiquifyGlobal, LiquifyMesh, LiquifyOpen,
    LiquifySession, LiquifySettings, LiquifySource, LiquifyStep, LiquifyStroke, LiquifyTiming, LiquifyTool, LiquifyWorkspace, MeshSize, StrokeEvent,
    StrokeRecorder,
};
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::primitives::Revision;
use craft_ui::runtime::access::Role;
use craft_ui::sections::DialogRole;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Color32, ColorImage, Event, Key, Modifiers, Pos2, Vec2};

/// The document's size in pixels.
const DOCUMENT: [u32; 2] = [800, 600];

/// A solid picture.
struct Solid {
    size: PixelSize,
    colour: Color32,
}

impl RasterSource for Solid {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(self.colour);
    }
}

/// The app's side: a picture and two layers.
struct Source {
    image: Solid,
}

impl LiquifySource for Source {
    fn picture(&self) -> &dyn RasterSource {
        &self.image
    }

    fn freeze_mask(&self) -> Option<(&dyn RasterSource, u32)> {
        None
    }

    fn mesh(&self, _: MeshSize) -> Option<LiquifyMesh> {
        None
    }

    fn backdrop(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn layers(&self) -> Vec<(u64, String)> {
        vec![(7, "Sky".to_owned()), (9, "Lake".to_owned())]
    }
}

/// What the app keeps of the workspace and what it saw.
struct App {
    workspace: LiquifyWorkspace,
    source: Source,
    events: Vec<LiquifyEvent>,
    closed: Option<DialogRole>,
    changed: bool,
    timing: Option<LiquifyTiming>,
    cursor: Option<Cursor>,
}

/// The workspace over a solid picture, settled.
fn open() -> (Fixture, Rc<RefCell<App>>) {
    let size = PixelSize::new(DOCUMENT[0], DOCUMENT[1]);
    let app = Rc::new(RefCell::new(App {
        workspace: LiquifyWorkspace::new(LiquifyOpen::new(DOCUMENT, "Photo")),
        source: Source { image: Solid { size, colour: Color32::from_rgb(40, 120, 200) } },
        events: Vec::new(),
        closed: None,
        changed: false,
        timing: None,
        cursor: None,
    }));
    let shown = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { workspace, source, .. } = &mut *app;
        let out = workspace.show(ui, source);
        app.events.extend(out.events);
        app.closed = app.closed.or(out.closed);
        app.changed |= out.settings_changed;
        app.timing = out.timing;
        app.cursor = out.cursor;
    });
    fixture.room(by(1000.0, 900.0));
    fixture.settle();
    (fixture, app)
}

/// The offset `x` points right and `y` points down.
fn by(x: f32, y: f32) -> Vec2 {
    Vec2::X * x + Vec2::Y * y
}

/// The middle of the canvas, moved by `by`.
fn canvas_point(fixture: &Fixture, offset: Vec2) -> Pos2 {
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas");
    canvas.center() + offset
}

fn primary(at: Pos2, down: bool) -> Event {
    if down { pointer::press(at) } else { pointer::release(at) }
}

/// Press at `from` with `button`, move to `to` in two steps, release.
fn drag(fixture: &mut Fixture, button: fn(Pos2, bool) -> Event, from: Pos2, to: Pos2) {
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![button(from, true)]);
    fixture.frame(vec![pointer::moved(from + (to - from) * 0.5)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![button(to, false)]);
    fixture.frame(Vec::new());
}

fn strokes(events: &[LiquifyEvent]) -> usize {
    events.iter().filter(|event| matches!(event, LiquifyEvent::Stroke(_))).count()
}

fn commits(events: &[LiquifyEvent]) -> Vec<LiquifyStroke> {
    events
        .iter()
        .filter_map(|event| match event {
            LiquifyEvent::Stroke(StrokeEvent::Commit(stroke)) => Some(stroke.clone()),
            _ => None,
        })
        .collect()
}

fn stroke(tool: LiquifyTool, points: &[[f64; 3]]) -> LiquifyStroke {
    LiquifyStroke::new(tool, &LiquifyBrush::default(), points.to_vec())
}

/// Presses at `at` and moves 30 points away, leaving the stroke open.
fn open_stroke(fixture: &mut Fixture, at: Pos2) {
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::press(at)]);
    fixture.frame(vec![pointer::moved(at + by(30.0, 0.0))]);
}

#[trial]
fn liquify_alt_right_drag_resizes_size_and_density_and_paints_nothing() {
    let (mut fixture, app) = open();
    let start = app.borrow().workspace.settings().brush;
    let from = canvas_point(&fixture, Vec2::ZERO);
    fixture.hold(Modifiers::ALT);
    drag(&mut fixture, pointer::secondary, from, from + by(40.0, 30.0));
    let brush = app.borrow().workspace.settings().brush;
    assert!(brush.size > start.size, "right widens the brush");
    assert!(brush.density > start.density, "down raises the density");
    assert_eq!((strokes(&app.borrow().events), app.borrow().workspace.session().len()), (0, 0), "a resize paints nothing");

    // Control + Alt is the Mac chord (Control and Option); elsewhere Ctrl is the command key.
    fixture.os(Os::Mac);
    fixture.hold(Modifiers { alt: true, ctrl: true, ..Modifiers::NONE });
    drag(&mut fixture, primary, from, from + by(-20.0, -20.0));
    let narrower = app.borrow().workspace.settings().brush;
    assert!(narrower.size < brush.size && narrower.density < brush.density, "Control + Alt + drag resizes too");
    assert_eq!(strokes(&app.borrow().events), 0);
}

#[trial]
fn liquify_alt_primary_drag_paints_with_the_alternate_tool() {
    let (mut fixture, app) = open();
    fixture.key(Key::C, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().tool, LiquifyTool::TwirlClockwise);
    let from = canvas_point(&fixture, Vec2::ZERO);
    fixture.hold(Modifiers::ALT);
    drag(&mut fixture, primary, from, from + by(40.0, 30.0));
    let tools: Vec<_> = commits(&app.borrow().events).iter().map(|stroke| stroke.tool).collect();
    assert_eq!(tools, [LiquifyTool::TwirlCounterclockwise]);
}

#[trial]
fn liquify_backdrop_layers_follow_the_mode_and_opacity() {
    let half = 128;
    let layering = |shown, mode| BackdropLayering::of(shown, mode, 50.0);
    let plain = |layering: BackdropLayering| (layering.image, layering.behind, layering.front);
    assert_eq!(plain(layering(false, BackdropMode::Blend)), (u8::MAX, None, None));
    assert_eq!(plain(layering(true, BackdropMode::InFront)), (u8::MAX, None, Some(half)));
    assert_eq!(plain(layering(true, BackdropMode::Behind)), (half, Some(u8::MAX), None));
    assert_eq!(plain(layering(true, BackdropMode::Blend)), (half, None, Some(half)));
    let full = BackdropLayering::of(true, BackdropMode::Blend, 100.0);
    assert_eq!((full.image, full.front), (0, Some(u8::MAX)), "Blend at 100 % shows the backdrop alone");

    let one = PixelSize::new(1, 1);
    let image = Solid { size: one, colour: Color32::from_rgb(255, 0, 0) };
    let backdrop = Solid { size: one, colour: Color32::from_rgb(0, 0, 255) };
    let pixel = |mode| {
        let layered = LayeredImage::new(&image, &backdrop, BackdropLayering::of(true, mode, 50.0));
        let mut out = ColorImage::filled([1, 1], Color32::TRANSPARENT);
        layered.read(PixelRect::new(0, 0, 1, 1), 1, &mut out);
        out.pixels[0].to_array()
    };
    let near = |got: [u8; 4], want: [u8; 4]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 2);
    assert!(near(pixel(BackdropMode::InFront), [127, 0, 128, 255]), "{:?}", pixel(BackdropMode::InFront));
    assert!(near(pixel(BackdropMode::Behind), [128, 0, 127, 255]), "{:?}", pixel(BackdropMode::Behind));
    assert!(near(pixel(BackdropMode::Blend), [85, 0, 170, 192]), "{:?}", pixel(BackdropMode::Blend));
}

#[trial]
fn liquify_show_backdrop_asks_the_app_for_the_backdrop() {
    let (mut fixture, app) = open();
    assert!(!app.borrow().events.iter().any(|event| matches!(event, LiquifyEvent::BackdropWanted(_))), "nothing asked while it is off");

    let check = fixture.by_label("Show Backdrop").expect("the View Options show a Show Backdrop check box");
    fixture.click(&check);
    fixture.settle();
    assert!(app.borrow().workspace.settings().show_backdrop);
    let asked: Vec<_> = app.borrow().events.iter().filter(|event| matches!(event, LiquifyEvent::BackdropWanted(_))).cloned().collect();
    assert_eq!(asked, [LiquifyEvent::BackdropWanted(None)]);

    app.borrow_mut().events.clear();
    let used = fixture.by_label("Use").expect("Show Backdrop adds a Use select");
    fixture.click(&used);
    fixture.settle();
    let lake = fixture.by_label("Lake").expect("the Use select lists the layers");
    fixture.click(&lake);
    fixture.settle();
    assert_eq!(app.borrow().workspace.settings().backdrop, BackdropUse::Layer(9));
    assert_eq!(app.borrow().events, [LiquifyEvent::BackdropWanted(Some(9))]);
}

#[trial]
fn liquify_brush_outline_follows_the_pointer_except_while_resizing() {
    let (mut fixture, app) = open();
    let outline = |app: &Rc<RefCell<App>>| matches!(&app.borrow().cursor, Some(Cursor::Painted(painted)) if *painted != PaintedCursor::Hidden);
    let on = canvas_point(&fixture, Vec2::ZERO);
    fixture.frame(vec![pointer::moved(on)]);
    assert!(outline(&app), "the outline is the pointer over the canvas");
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas");
    fixture.frame(vec![pointer::moved(canvas.rect.min - by(1.0, 1.0))]);
    assert!(!outline(&app), "no outline off the canvas");

    let anchor = on + by(20.0, 20.0);
    fixture.frame(vec![pointer::moved(anchor)]);
    fixture.hold(Modifiers::ALT);
    fixture.frame(vec![pointer::secondary(anchor, true)]);
    fixture.frame(vec![pointer::moved(anchor + by(30.0, 0.0))]);
    assert_eq!(app.borrow().cursor, Some(Cursor::Painted(PaintedCursor::Hidden)), "the pointer hides while resizing");
    fixture.frame(vec![pointer::secondary(anchor + by(30.0, 0.0), false)]);
    fixture.hold(Modifiers::NONE);
    fixture.frame(vec![pointer::moved(anchor + by(30.0, 0.0))]);
    assert!(outline(&app), "the outline follows the pointer again");
}

#[trial]
fn liquify_escape_cancels_the_stroke_and_replays_then_the_next_cancels_the_dialog() {
    let (mut fixture, app) = open();
    let from = canvas_point(&fixture, Vec2::ZERO);
    open_stroke(&mut fixture, from);
    app.borrow_mut().events.clear();

    fixture.key(Key::Escape, Modifiers::NONE);
    let events = app.borrow().events.clone();
    assert!(
        matches!(events.as_slice(), [LiquifyEvent::Stroke(StrokeEvent::Cancel(_)), LiquifyEvent::Replay(steps)] if steps.is_empty()),
        "{events:?}"
    );
    assert!(app.borrow().workspace.session().is_empty(), "nothing was recorded");
    assert_eq!(app.borrow().closed, None, "the dialog stays open");
    fixture.frame(vec![pointer::release(from + by(30.0, 0.0))]);

    fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(app.borrow().closed, Some(DialogRole::Cancel));
}

#[trial]
fn liquify_freeze_lasso_commits_a_polygon_and_alt_thaws() {
    let (mut fixture, app) = open();
    fixture.key(Key::L, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().tool, LiquifyTool::FreezeLasso);
    let corners = [Vec2::ZERO, by(80.0, 0.0), by(80.0, 60.0), by(0.0, 60.0)].map(|by| canvas_point(&fixture, by));
    for modifiers in [Modifiers::NONE, Modifiers::ALT] {
        fixture.hold(modifiers);
        fixture.frame(vec![pointer::moved(corners[0])]);
        fixture.frame(vec![pointer::press(corners[0])]);
        for corner in &corners[1..] {
            fixture.frame(vec![pointer::moved(*corner)]);
        }
        fixture.frame(vec![pointer::release(corners[3])]);
        fixture.frame(Vec::new());
    }
    let committed: Vec<_> = commits(&app.borrow().events).iter().map(|stroke| (stroke.tool, stroke.amount, stroke.points.len() >= 4)).collect();
    assert_eq!(committed, [(LiquifyTool::FreezeLasso, Some(1.0), true), (LiquifyTool::FreezeLasso, Some(0.0), true)]);
    assert_eq!(strokes(&app.borrow().events), 2, "a lasso reports only its Commit");
    assert_eq!(app.borrow().workspace.session().len(), 2);
}

#[trial]
fn liquify_a_letter_mid_stroke_cancels_the_stroke_and_leaves_the_canvas_idle() {
    let (mut fixture, app) = open();
    let from = canvas_point(&fixture, Vec2::ZERO);
    open_stroke(&mut fixture, from);
    app.borrow_mut().events.clear();

    fixture.key(Key::R, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().tool, LiquifyTool::Reconstruct, "the letter picked the tool");
    let events = app.borrow().events.clone();
    assert!(
        matches!(events.as_slice(), [LiquifyEvent::Stroke(StrokeEvent::Cancel(CancelReason::Removed)), LiquifyEvent::Replay(steps)] if steps.is_empty()),
        "{events:?}"
    );
    fixture.frame(vec![pointer::release(from + by(30.0, 0.0))]);
    assert!(commits(&app.borrow().events).is_empty(), "the new tool got no release");
    assert!(app.borrow().workspace.session().is_empty());
}

#[trial]
fn liquify_undo_mid_stroke_cancels_the_stroke_then_steps() {
    let (mut fixture, app) = open();
    let left = canvas_point(&fixture, by(-40.0, 0.0));
    drag(&mut fixture, primary, left, left + by(80.0, 10.0));
    assert_eq!(app.borrow().workspace.session().len(), 1);
    let below = canvas_point(&fixture, by(0.0, 40.0));
    open_stroke(&mut fixture, below);
    app.borrow_mut().events.clear();

    assert!(app.borrow_mut().workspace.step(Step::Undo), "the step waits for the next pass");
    fixture.frame(Vec::new());
    let events = app.borrow().events.clone();
    assert!(
        matches!(
            events.as_slice(),
            [LiquifyEvent::Stroke(StrokeEvent::Cancel(CancelReason::UndoDuringGesture)), LiquifyEvent::Replay(steps)] if steps.is_empty()
        ),
        "{events:?}"
    );
    assert!(app.borrow().workspace.session().is_empty(), "the earlier stroke is undone");
}

#[trial]
fn liquify_step_from_the_menu_queues_the_replay_for_the_next_pass() {
    let (mut fixture, app) = open();
    let left = canvas_point(&fixture, by(-40.0, 0.0));
    drag(&mut fixture, primary, left, left + by(80.0, 10.0));
    app.borrow_mut().events.clear();

    assert!(app.borrow_mut().workspace.step(Step::Undo));
    assert!(app.borrow().workspace.session().is_empty());
    assert!(!app.borrow_mut().workspace.step(Step::Undo), "nothing left to undo");
    fixture.frame(Vec::new());
    let events = app.borrow().events.clone();
    assert!(matches!(events.as_slice(), [LiquifyEvent::Replay(steps)] if steps.is_empty()), "{events:?}");
}

#[trial]
fn liquify_output_reports_resize_changes_and_the_stroke_timing() {
    let (mut fixture, app) = open();
    assert!(!app.borrow().changed, "nothing changed yet");
    let from = canvas_point(&fixture, Vec2::ZERO);
    fixture.hold(Modifiers::ALT);
    drag(&mut fixture, pointer::secondary, from, from + by(40.0, 0.0));
    assert!(app.borrow().changed, "Alt + right-drag changed the brush");
    assert_eq!(app.borrow().timing, None, "a resize is not a stroke");

    fixture.hold(Modifiers::NONE);
    let left = canvas_point(&fixture, by(-40.0, 0.0));
    drag(&mut fixture, primary, left, left + by(80.0, 10.0));
    assert!(app.borrow().timing.is_some_and(|timing| timing.dabs >= 2), "{:?}", app.borrow().timing);
}

#[trial]
fn liquify_tool_letters_and_alt_swaps() {
    let (mut fixture, app) = open();
    let letters = [
        (Key::W, LiquifyTool::ForwardWarp),
        (Key::R, LiquifyTool::Reconstruct),
        (Key::E, LiquifyTool::Smooth),
        (Key::C, LiquifyTool::TwirlClockwise),
        (Key::S, LiquifyTool::Pucker),
        (Key::B, LiquifyTool::Bloat),
        (Key::O, LiquifyTool::PushLeft),
        (Key::F, LiquifyTool::Freeze),
        (Key::D, LiquifyTool::Thaw),
        (Key::L, LiquifyTool::FreezeLasso),
        (Key::H, LiquifyTool::Hand),
        (Key::Z, LiquifyTool::Zoom),
    ];
    for (key, tool) in letters {
        fixture.key(key, Modifiers::NONE);
        assert_eq!(app.borrow().workspace.settings().tool, tool, "{key:?}");
    }
    assert!(LiquifyTool::ALL.iter().all(|tool| LiquifyCommands::of(*tool).tool() == Some(*tool)));
    assert_eq!(LiquifyTool::TwirlClockwise.alternate(), LiquifyTool::TwirlCounterclockwise);
    assert_eq!(LiquifyTool::TwirlCounterclockwise.alternate(), LiquifyTool::TwirlClockwise);
    assert_eq!(LiquifyTool::Pucker.alternate(), LiquifyTool::Bloat);
    assert_eq!(LiquifyTool::Bloat.alternate(), LiquifyTool::Pucker);
    for tool in [LiquifyTool::ForwardWarp, LiquifyTool::Smooth, LiquifyTool::Freeze, LiquifyTool::FreezeLasso, LiquifyTool::Hand] {
        assert_eq!(tool.alternate(), tool);
    }
    let slots = LiquifyTool::slots();
    assert_eq!(slots.len(), LiquifyTool::ALL.len());
    let ends: Vec<LiquifyTool> = slots.iter().filter(|slot| slot.group_end).map(|slot| slot.tools[0].id).collect();
    assert_eq!(ends, [LiquifyTool::Smooth, LiquifyTool::PushLeft, LiquifyTool::Thaw, LiquifyTool::FreezeLasso]);
}

// A press, two drags and a release report Begin, Update, Update, Commit; each Update carries only its new point and
// the Commit the whole stroke; a press and release one pixel apart is recorded whole with both points.
#[trial]
fn liquify_stroke_events_begin_update_commit_and_short_strokes_are_recorded_whole() {
    let mut recorder = StrokeRecorder::default();
    let brush = LiquifyBrush::default();
    let StrokeEvent::Begin(first) = recorder.begin(LiquifyTool::Pucker, &brush, [10.0, 10.0, 1.0], true) else { panic!("Begin") };
    assert!(recorder.is_active());
    assert_eq!((first.tool, first.points.len()), (LiquifyTool::Bloat, 1), "Alt paints with the alternate tool");
    let Some(StrokeEvent::Update(two)) = recorder.extend([20.0, 10.0, 0.5]) else { panic!("Update") };
    assert_eq!(two.points, [[20.0, 10.0, 0.5]], "an Update carries the new point only");
    assert_eq!(two.tool, LiquifyTool::Bloat, "and the stroke's tool and brush");
    assert!(recorder.extend([20.0, 10.0, 0.9]).is_none(), "an equal point is skipped");
    let Some(StrokeEvent::Update(three)) = recorder.extend([30.0, 12.0, 0.5]) else { panic!("Update") };
    assert_eq!(three.points, [[30.0, 12.0, 0.5]]);
    for held in [4, 5] {
        let Some(StrokeEvent::Update(value)) = recorder.hold([30.0, 12.0, 0.5]) else { panic!("Update") };
        assert_eq!(value.points, [[30.0, 12.0, 0.5]], "a held dab {held} is its own point");
    }
    let Some(StrokeEvent::Commit(whole)) = recorder.end([40.0, 12.0]) else { panic!("Commit") };
    assert!(!recorder.is_active());
    assert_eq!(whole.points.last(), Some(&[40.0, 12.0, 0.5]), "the release point keeps the last pressure");
    assert_eq!(whole.points.len(), 6);

    recorder.begin(LiquifyTool::ForwardWarp, &brush, [5.0, 5.0, 1.0], false);
    let Some(StrokeEvent::Commit(short)) = recorder.end([6.0, 5.0]) else { panic!("Commit") };
    assert_eq!(short.points, [[5.0, 5.0, 1.0], [6.0, 5.0, 1.0]]);

    recorder.begin(LiquifyTool::ForwardWarp, &brush, [5.0, 5.0, 1.0], false);
    assert!(matches!(recorder.cancel(CancelReason::Escape), Some(StrokeEvent::Cancel(CancelReason::Escape))));
    assert!(recorder.end([1.0, 1.0]).is_none());
}

// Undo after two strokes gives the first; redo gives both, and each step asks the app to replay.
#[trial]
fn liquify_undo_redo_ask_the_app_to_replay_and_redo_is_bounded() {
    let (mut fixture, app) = open();
    for row in [-40.0, 40.0] {
        let from = canvas_point(&fixture, by(-40.0, row));
        drag(&mut fixture, primary, from, from + by(80.0, 0.0));
    }
    assert_eq!(app.borrow().workspace.session().len(), 2);
    assert!(app.borrow_mut().workspace.step(Step::Undo));
    assert_eq!(app.borrow().workspace.session().len(), 1);
    assert!(app.borrow_mut().workspace.step(Step::Redo));
    assert_eq!(app.borrow().workspace.session().len(), 2);
    fixture.frame(Vec::new());
    let replays = app.borrow().events.iter().filter(|event| matches!(event, LiquifyEvent::Replay(_))).count();
    assert_eq!(replays, 2, "each step asks the app to rebuild its field");
}

// A closed Freeze Lasso records one stroke with amount 1 (0 with Alt at the press) and the first point again at its
// end; fewer than 3 points record nothing.
#[trial]
fn liquify_freeze_lasso_records_a_stroke_and_alt_thaws() {
    let mut session = LiquifySession::default();
    let open = [[0.0, 0.0, 1.0], [10.0, 0.0, 1.0], [10.0, 10.0, 1.0]];
    let mut freeze = stroke(LiquifyTool::FreezeLasso, &open);
    freeze.amount = Some(1.0);
    session.record(freeze);
    let mut thaw = stroke(LiquifyTool::FreezeLasso, &[open[0], open[1], open[2], [0.2, 0.3, 1.0]]);
    thaw.amount = Some(0.0);
    session.record(thaw);
    session.record(stroke(LiquifyTool::FreezeLasso, &open[..2]));
    session.record(stroke(LiquifyTool::ForwardWarp, &[]));
    let [LiquifyStep::Stroke(first), LiquifyStep::Stroke(second)] = session.steps() else { panic!("two lassos recorded") };
    assert_eq!((first.amount, first.points.len(), first.points.last()), (Some(1.0), 4, Some(&open[0])));
    assert_eq!((second.amount, second.points.len()), (Some(0.0), 4), "a point within half a pixel closes it");
}

// Cmd+I and Cmd+D record the global step in the order pressed; Restore All empties the session.
#[trial]
fn liquify_global_buttons_and_keys_report_the_global_event() {
    let (mut fixture, app) = open();
    fixture.key(Key::D, Modifiers::COMMAND);
    fixture.key(Key::I, Modifiers::COMMAND);
    let globals: Vec<_> = app
        .borrow()
        .events
        .iter()
        .filter_map(|event| match event {
            LiquifyEvent::Global(op) => Some(*op),
            _ => None,
        })
        .collect();
    assert_eq!(globals, [LiquifyGlobal::MaskNone, LiquifyGlobal::InvertAll], "in the order pressed");
    assert_eq!(app.borrow().workspace.session().len(), 2);

    let mut session = LiquifySession::default();
    for op in [LiquifyGlobal::reconstruct(40.0), LiquifyGlobal::MaskAll, LiquifyGlobal::MaskNone, LiquifyGlobal::InvertAll] {
        session.record_global(op);
    }
    assert_eq!(session.len(), 4);
    assert_eq!(session.steps()[0], LiquifyStep::Global(LiquifyGlobal::reconstruct(40.0)));
    session.restore_all();
    assert!(session.is_empty());
}

// [ and ] change the size by at least one pixel within 1..=15000; Cmd+H toggles Show Mask.
#[trial]
fn liquify_brackets_scale_the_brush_and_command_h_toggles_the_mask() {
    let (mut fixture, app) = open();
    fixture.key(Key::CloseBracket, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().brush.size, 110.0);
    fixture.key(Key::OpenBracket, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().brush.size, 99.0);

    let at = |size| {
        let mut brush = LiquifyBrush::default();
        brush.size = size;
        brush
    };
    assert_eq!(at(2.0).smaller().size, 1.0);
    assert_eq!(at(1.0).smaller().size, 1.0);
    assert_eq!(at(3.0).larger().size, 4.0, "at least one pixel of change");
    assert_eq!(at(14999.0).larger().size, LiquifyBrush::SIZE_MAX);
    assert_eq!(at(LiquifyBrush::SIZE_MAX).larger().size, LiquifyBrush::SIZE_MAX);

    assert!(app.borrow().workspace.settings().show_mask);
    fixture.key(Key::H, Modifiers::COMMAND);
    assert!(!app.borrow().workspace.settings().show_mask);
}

// The workspace's keys belong to the dialog: with focus on a footer button a tool letter still picks its tool and a
// zoom chord still zooms.
#[trial]
fn liquify_keys_work_with_focus_on_the_footer_button() {
    let (mut fixture, app) = open();
    let ok = fixture.by_label("OK").expect("the footer has OK");
    fixture.access(AccessActionKind::Focus, &ok);
    fixture.idle(1);
    // The zoom is the last bracketed part of the dialog's title: "Liquify (Photo, 89.25%)".
    let zoom = |fixture: &Fixture| {
        let dialog = fixture.by_role(Role::DIALOG).into_iter().next()?;
        let title = craft_ui::testing::access::name(fixture.tree(), dialog.node)?;
        title.rsplit(", ").next()?.strip_suffix(")").filter(|zoom| zoom.ends_with('%')).map(str::to_owned)
    };
    let before = zoom(&fixture);
    fixture.key(Key::R, Modifiers::NONE);
    assert_eq!(app.borrow().workspace.settings().tool, LiquifyTool::Reconstruct);
    fixture.key(Key::Equals, Modifiers::COMMAND);
    fixture.idle(2);
    assert!(before.is_some() && zoom(&fixture) != before, "the zoom chord zooms: {before:?} then {:?}", zoom(&fixture));
}

// The settings round-trip through serde; a partial record fills the defaults; `clamped` puts out-of-range numbers back
// in range.
#[trial]
fn liquify_settings_round_trip_with_serde_defaults() {
    let mut settings = LiquifySettings::default();
    settings.tool = LiquifyTool::Bloat;
    settings.show_mesh = true;
    settings.backdrop_opacity = 20.0;
    let json = serde_json::to_string(&settings).expect("settings serialise");
    assert_eq!(serde_json::from_str::<LiquifySettings>(&json).ok(), Some(settings));
    let partial = serde_json::from_str::<LiquifySettings>(r#"{"showMesh":true,"brush":{"size":40}}"#).expect("a partial record loads");
    assert!(partial.show_mesh && partial.show_mask);
    assert_eq!((partial.brush.size, partial.brush.density), (40.0, 50.0));

    let mut wild = LiquifySettings::default();
    wild.brush.size = 1.0e9;
    wild.brush.density = -5.0;
    wild.brush.pressure = f64::NAN;
    wild.reconstruct_amount = 250.0;
    wild.backdrop_opacity = f64::INFINITY;
    // A non-finite number keeps the value in force, not the default.
    let mut old = LiquifySettings::default();
    old.brush.pressure = 70.0;
    old.backdrop_opacity = 20.0;
    let tame = wild.clamped(&old);
    assert_eq!((tame.brush.size, tame.brush.density, tame.brush.pressure), (LiquifyBrush::SIZE_MAX, 0.0, 70.0));
    assert_eq!((tame.reconstruct_amount, tame.backdrop_opacity), (100.0, 20.0));
}

// A stroke serialises with the engine's names (`twirlCw`, `lassoMask`, `points` as triples).
#[trial]
fn liquify_strokes_serialise_as_the_engine_reads_them() {
    let mut lasso = stroke(LiquifyTool::FreezeLasso, &[[1.0, 2.0, 1.0]]);
    lasso.amount = Some(0.0);
    let value = serde_json::to_value(&lasso).expect("serialise");
    assert_eq!(value["tool"], "lassoMask");
    assert_eq!(value["amount"], 0.0);
    assert_eq!(value["points"], serde_json::json!([[1.0, 2.0, 1.0]]));
    let twirl = serde_json::to_value(stroke(LiquifyTool::TwirlClockwise, &[])).expect("serialise");
    assert_eq!(twirl["tool"], "twirlCw");
    assert!(twirl.get("amount").is_none());
    assert_eq!(serde_json::to_value(LiquifyTool::TwirlCounterclockwise).expect("serialise"), "twirlCcw");
}
