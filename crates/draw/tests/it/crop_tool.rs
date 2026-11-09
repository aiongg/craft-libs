//! The Crop tool and its session through the public API: the pass, the keys the overlay leaves to the tool, the
//! session's box history, and one drag through the overlay on a canvas.

use craft_behavior::geometry::CropBox;
use craft_behavior::kurbo::{Point, Rect, Size};
use craft_behavior::{CanvasView, EditEvent, GestureId, Mixed};
use craft_draw::ui::crop::{CropDocKey, CropDocument, CropEvent, CropInput, CropRatio, CropSession, CropSettings, CropTool, CropToolPass};
use craft_keymap::Mods;
use craft_ui::canvas::{Canvas, CanvasSpace, CanvasTool as _, ToolContext, ToolHost};
use craft_ui::contract::ShowContainer as _;
use craft_ui::runtime::access::Role;
use craft_ui::runtime::env::{Env, Units};
use craft_ui::testing::{Fixture, Frames, pointer, trial};
use egui::{Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const SIZE: Size = Size::new(400.0, 300.0);

/// The units of the default scope (PostScript points).
fn units() -> Units {
    let mut frames = Frames::new();
    let mut units = None;
    frames.frame(vec![], |ui| units = Some(Env::of(ui).units()));
    units.expect("the frame ran")
}

fn document(key: u64) -> CropDocument {
    CropDocument::new(CropDocKey::new(key), SIZE, 72.0)
}

fn upright(x0: f64, y0: f64, x1: f64, y1: f64) -> CropBox {
    CropBox::upright(Rect::new(x0, y0, x1, y1))
}

fn classic() -> CropSettings {
    let mut settings = CropSettings::default();
    settings.shield.classic_mode = true;
    settings
}

/// A tool with its settings and document, driven pass by pass through a `ToolContext` without a canvas.
struct Run {
    tool: CropTool,
    settings: CropSettings,
    document: CropDocument,
    space: Option<CanvasSpace>,
    pause: Option<f64>,
    ctx: egui::Context,
}

impl Run {
    fn new(settings: CropSettings) -> Self {
        Run { tool: CropTool::new(), settings, document: document(1), space: None, pause: None, ctx: egui::Context::default() }
    }

    /// One pass at `time`: `each_pass`, then `act`; returns what `act` returns and the pass's events. `Settings`
    /// events are stored.
    fn pass<R>(&mut self, time: f64, mods: Mods, act: impl FnOnce(&mut CropToolPass<'_>, &mut ToolContext<'_>) -> R) -> (R, Vec<CropEvent>) {
        let mut cx = ToolContext::new(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0)), self.space).time(time).mods(mods);
        let mut pass = self.tool.pass(CropInput::new(&self.ctx, &self.settings, &self.document).nudge_pause(self.pause));
        pass.each_pass(&mut cx);
        let result = act(&mut pass, &mut cx);
        let events = pass.finish().events;
        for event in &events {
            if let CropEvent::Settings(next) = event {
                self.settings = next.clone();
            }
        }
        (result, events)
    }

    fn settle(&mut self) {
        self.pass(0.0, Mods::NONE, |_, _| ());
    }

    fn frame(&self) -> Option<CropBox> {
        self.tool.frame()
    }
}

#[trial]
fn image_mode_requests_the_turned_view() {
    let room = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
    let upright_view = CanvasView::new(room, Point::new(200.0, 150.0), 1.0);
    let turned = CropBox::new(Rect::new(100.0, 80.0, 300.0, 220.0), -30.0);

    let mut run = Run::new(CropSettings::default());
    run.space = Some(CanvasSpace::new(upright_view, 1.0));
    run.settle();
    run.tool.crop_to(turned);
    let (view, _) = run.pass(0.0, Mods::NONE, |_, cx| cx.requested_view());
    let view = view.expect("image mode turns the view for a frame changed outside a gesture");
    assert!((view.rotation - 30.0).abs() < 1e-6, "the view turns against the frame: {}", view.rotation);

    let mut run = Run::new(classic());
    run.space = Some(CanvasSpace::new(upright_view, 1.0));
    run.settle();
    run.tool.crop_to(turned);
    let (view, _) = run.pass(0.0, Mods::NONE, |_, cx| cx.requested_view());
    assert_eq!(view.map(|v| v.rotation), None, "Classic Mode turns the frame over an upright view and asks for nothing more");

    let mut run = Run::new(classic());
    run.space = Some(CanvasSpace::new(upright_view.rotation(20.0), 1.0));
    run.settle();
    run.tool.crop_to(turned);
    let (view, _) = run.pass(0.0, Mods::NONE, |_, cx| cx.requested_view());
    assert_eq!(view.map(|v| v.rotation), Some(0.0), "Classic Mode turns a turned view upright once");
}

#[trial]
fn editing_a_ratio_refits_from_the_frame_the_edit_began_with() {
    let mut run = Run::new(classic());
    run.settle();
    run.tool.crop_to(upright(50.0, 50.0, 350.0, 250.0));
    let document = run.document;
    let mut settings = run.settings.clone();
    let mut refit = |tool: &mut CropTool, width: f64, height: f64, editing: bool| {
        settings.ratio = CropRatio::sides(width, height);
        let Some(CropEvent::Frame(event)) = tool.refit(&settings, &document, &units(), editing) else { return None };
        event.value().copied()
    };

    let narrow = refit(&mut run.tool, 1.0, 3.0, true).expect("1:3 fits inside the frame");
    assert!((narrow.rect.width() - 200.0 / 3.0).abs() < 1e-9 && (narrow.rect.height() - 200.0).abs() < 1e-9, "{narrow:?}");
    assert_eq!(narrow.rect.center(), Point::new(200.0, 150.0), "about the centre");
    let wide = refit(&mut run.tool, 16.0, 3.0, true).expect("16:3 fits inside the first frame");
    assert!((wide.rect.width() - 300.0).abs() < 1e-9 && (wide.rect.height() - 56.25).abs() < 1e-9, "{wide:?}");
    // The run ends: the next change fits from the frame as it is.
    assert_eq!(refit(&mut run.tool, 16.0, 3.0, false), None);
    let square = refit(&mut run.tool, 1.0, 1.0, false).expect("1:1 fits inside the wide frame");
    assert!((square.rect.width() - 56.25).abs() < 1e-9 && (square.rect.height() - 56.25).abs() < 1e-9, "{square:?}");
}

#[trial]
fn leaving_applies_a_drawn_frame_and_a_refused_commit_restores_it() {
    let mut run = Run::new(classic());
    run.settle();
    assert_eq!(run.tool.leave(&run.settings, &run.document, &units(), false), None, "the default frame is dropped");
    assert_eq!(run.frame(), None);
    run.settle();

    let drawn = upright(100.0, 80.0, 300.0, 220.0);
    run.tool.crop_to(drawn);
    assert_eq!(run.tool.leave(&run.settings, &run.document, &units(), true), None, "a kept frame stays");
    let Some(CropEvent::Apply(commit)) = run.tool.leave(&run.settings, &run.document, &units(), false) else { panic!("a drawn frame applies") };
    assert_eq!(commit.frame, drawn);
    // A refusal of the Apply from leaving clears the session: the next tool does not retry it.
    run.tool.applied(false);
    assert_eq!(run.frame(), None);
    run.settle();

    run.tool.crop_to(drawn);
    let Some(CropEvent::Apply(_)) = run.tool.apply(&run.settings, &run.document, &units()) else { panic!("a drawn frame applies") };
    // The app's command changed the box history; the refusal gives it back.
    run.tool.undo();
    run.tool.applied(false);
    assert_eq!(run.frame(), Some(drawn));
    assert!(run.tool.session().can_undo());
}

#[trial]
fn a_document_switch_drops_the_kept_frame() {
    let mut run = Run::new(classic());
    run.settle();
    run.tool.crop_to(upright(100.0, 80.0, 300.0, 220.0));
    let Some(CropEvent::Apply(_)) = run.tool.apply(&run.settings, &run.document, &units()) else { panic!("a drawn frame applies") };
    run.document = document(2);
    run.settle();
    run.tool.applied(false);
    assert_eq!(run.frame(), Some(upright(0.0, 0.0, 400.0, 300.0)), "the refusal of another document's crop brings nothing back");
    assert!(!run.tool.session().pending());
}

// The session cases.

fn rect_of(session: &CropSession) -> Option<Rect> {
    session.frame().map(|f| f.rect)
}

fn sized(key: u64, width: f64, height: f64) -> CropDocument {
    CropDocument::new(CropDocKey::new(key), Size::new(width, height), 72.0)
}

#[trial]
fn default_frame_is_the_selection_bounds_or_the_canvas() {
    let mut session = CropSession::new();
    assert!(session.ensure_frame(&sized(1, 200.0, 100.0), false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 200.0, 100.0)));
    assert!(session.is_default() && !session.pending());
    assert!(!session.ensure_frame(&sized(1, 200.0, 100.0), false), "a frame exists");

    let mut session = CropSession::new();
    let selected = sized(1, 200.0, 100.0).selection(Some(Rect::new(150.0, 20.0, 300.0, 60.0)));
    assert!(session.ensure_frame(&selected, false));
    assert_eq!(rect_of(&session), Some(Rect::new(150.0, 20.0, 200.0, 60.0)));

    let mut session = CropSession::new();
    let empty = sized(1, 200.0, 100.0).selection(Some(Rect::new(10.0, 10.0, 10.0, 50.0)));
    assert!(session.ensure_frame(&empty, false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 200.0, 100.0)));

    let mut session = CropSession::new();
    assert!(!session.ensure_frame(&sized(1, 0.0, 100.0), false));
    assert!(session.frame().is_none());
    assert!(!session.ensure_frame(&sized(1, 50.0, 50.0), true), "a gesture runs");
}

#[trial]
fn another_document_or_size_gets_its_own_default_frame() {
    let mut session = CropSession::new();
    session.ensure_frame(&sized(1, 200.0, 100.0), false);
    assert!(session.ensure_frame(&sized(1, 300.0, 100.0), false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 300.0, 100.0)));
    assert!(session.ensure_frame(&sized(2, 300.0, 100.0), false));
    assert!(session.is_default());

    assert!(!session.for_document(CropDocKey::new(2)));
    assert!(session.frame().is_some());
    session.set(upright(10.0, 10.0, 50.0, 50.0), false, "Crop");
    assert!(session.for_document(CropDocKey::new(3)), "a drawn frame was dropped");
    assert!(session.frame().is_none() && !session.can_undo());

    session.ensure_frame(&sized(3, 10.0, 10.0), false);
    assert!(!session.for_document(CropDocKey::new(4)), "an untouched default frame is no discard");
    assert!(session.frame().is_none());
}

#[trial]
fn box_undo_redo_steps_committed_gestures_only() {
    let mut session = CropSession::new();
    let ctx = egui::Context::default();
    session.ensure_frame(&sized(1, 200.0, 100.0), false);
    let start = session.frame();
    assert!(!session.can_undo());

    session.nudge(&ctx, upright(10.0, 0.0, 100.0, 100.0), "Crop");
    session.nudge(&ctx, upright(20.0, 0.0, 100.0, 100.0), "Crop");
    session.nudge(&ctx, upright(30.0, 0.0, 100.0, 100.0), "Crop");
    assert!(session.can_undo(), "an open run counts, so Undo never falls through to the document");
    session.settle();
    assert!(session.pending());
    session.set(upright(31.0, 0.0, 100.0, 100.0), false, "Crop");

    assert!(session.undo());
    assert_eq!(session.frame(), Some(upright(30.0, 0.0, 100.0, 100.0)), "the run of nudges is one step");
    assert!(session.undo());
    assert_eq!(session.frame(), start);
    assert!(session.is_default(), "the default mark comes back with the step");
    assert!(!session.can_undo() && !session.undo());
    assert!(session.can_redo() && session.redo());
    assert_eq!(session.frame(), Some(upright(30.0, 0.0, 100.0, 100.0)));
    assert!(!session.is_default());

    session.set(upright(5.0, 5.0, 60.0, 60.0), false, "Crop");
    assert!(!session.can_redo() && !session.redo(), "a new step drops the redo side");
}

#[trial]
fn a_cancelled_drag_goes_back_to_the_latest_step() {
    let mut session = CropSession::new();
    session.ensure_frame(&sized(1, 200.0, 100.0), false);
    let start = session.frame();
    let id = GestureId::new(900);
    session.mirror(&EditEvent::begin(id, Mixed::Same(start.expect("a default frame")), "Crop".into(), None));
    assert!(!session.is_default() && session.editing(), "a drag ends the default mark");
    session.mirror(&EditEvent::cancel(id, craft_behavior::CancelReason::Escape, None));
    assert_eq!(session.frame(), start);
    assert!(session.is_default() && !session.can_undo(), "the cancelled drag leaves the untouched default frame and no step");

    let drawn = upright(10.0, 10.0, 80.0, 60.0);
    session.mirror(&EditEvent::begin(id, Mixed::Same(start.expect("a default frame")), "Crop".into(), None));
    session.mirror(&EditEvent::commit(id, Mixed::Same(start.expect("a default frame")), drawn));
    assert_eq!(session.frame(), Some(drawn));
    assert!(session.pending() && session.can_undo(), "a committed drag is one box step");
}

// One drag through the overlay on a canvas.

struct Host {
    tool: CropTool,
    settings: CropSettings,
    document: CropDocument,
    ctx: egui::Context,
    events: Vec<CropEvent>,
}

struct Rig {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
}

impl Rig {
    fn new(settings: CropSettings) -> Self {
        let host =
            Rc::new(RefCell::new(Host { tool: CropTool::new(), settings, document: document(1), ctx: egui::Context::default(), events: Vec::new() }));
        let shown = Rc::clone(&host);
        let mut fixture = Fixture::new(move |ui| {
            let mut guard = shown.borrow_mut();
            let host = &mut *guard;
            let room = ui.available_rect_before_wrap();
            let view = CanvasView::new(room, Point::new(200.0, 150.0), 1.0);
            let mut pass = host.tool.pass(CropInput::new(&host.ctx, &host.settings, &host.document));
            let canvas = Canvas::new("Document").document(SIZE).view(view).show(ui, |ui, body| pass.show_frame(ui, body.rect));
            let _ = ToolHost::new().run(ui, &canvas.output, &mut pass);
            let events = pass.finish().events;
            for event in &events {
                if let CropEvent::Settings(next) = event {
                    host.settings = next.clone();
                }
            }
            host.events.extend(events);
        });
        fixture.settle();
        Rig { fixture, host }
    }

    /// The screen point of document point `(x, y)`: the view shows (200, 150) at the canvas's centre.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        let canvas = self.fixture.by_role(Role::CANVAS).into_iter().next().expect("a canvas");
        canvas.center() + Vec2::new(x as f32 - 200.0, y as f32 - 150.0)
    }

    fn drag(&mut self, from: (f64, f64), to: (f64, f64)) {
        let (from, to) = (self.at(from.0, from.1), self.at(to.0, to.1));
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![pointer::press(from)]);
        for step in 1..=4 {
            self.fixture.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 4.0))]);
        }
        self.fixture.frame(vec![pointer::release(to)]);
        self.fixture.frame(Vec::new());
    }
}

#[trial]
fn leaving_during_a_drag_waits_for_the_release() {
    let mut rig = Rig::new(classic());
    rig.drag((100.0, 80.0), (300.0, 220.0));
    let from = rig.at(200.0, 150.0);
    rig.fixture.frame(vec![pointer::moved(from)]);
    rig.fixture.frame(vec![pointer::press(from)]);
    rig.fixture.frame(vec![pointer::moved(from + Vec2::new(30.0, 10.0))]);
    rig.fixture.frame(vec![pointer::moved(from + Vec2::new(40.0, 20.0))]);
    {
        let mut host = rig.host.borrow_mut();
        let Host { tool, settings, document, .. } = &mut *host;
        assert!(tool.dragging());
        assert_eq!(tool.leave(settings, document, &units(), false), None, "a drag keeps the frame pending");
    }
    rig.fixture.frame(vec![pointer::release(from + Vec2::new(40.0, 20.0))]);
    rig.fixture.frame(Vec::new());
    let mut host = rig.host.borrow_mut();
    let Host { tool, settings, document, .. } = &mut *host;
    assert!(matches!(tool.leave(settings, document, &units(), false), Some(CropEvent::Apply(_))), "after the release the drawn frame applies");
}
