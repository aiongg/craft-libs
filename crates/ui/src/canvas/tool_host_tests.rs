//! The tool host: one tool's presses, drags, releases, keys and paint.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::{Event, Key, KeyboardShortcut, Modifiers, Painter, PointerButton, Pos2, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Size};

use super::claim::PrimaryPress;
use super::navigate::{HandTool, NavigationOptions, ZoomTool};
use super::zoom::{self, ZoomAnimation, ZoomLadder};
use super::{
    CanvasId, CanvasTool, CanvasView, Claim, SnapGesture, SnapKind, SnapTarget, SnapTargets, ToolAction, ToolContext, ToolGesture, ToolHost,
};
use super::{Part, ToolHostResponse, ToolInput, ToolKey, ToolNode, ToolParts, ToolPointer};
use crate::behavior::KeyClaim;
use crate::edit::CancelReason;
use crate::theme::{Base, install};

type Log = Rc<RefCell<Vec<String>>>;

/// A tool that logs what the host calls, with the knobs the tests turn.
struct Probe {
    log: Log,
    action: ToolAction,
    snap: Option<SnapGesture>,
    autoscrolls: bool,
    takes_key: bool,
    /// The reason of each `cancel` call.
    reasons: Rc<RefCell<Vec<CancelReason>>>,
}

impl Probe {
    fn new(log: &Log) -> Self {
        Self { log: log.clone(), action: ToolAction::Claimed, snap: None, autoscrolls: true, takes_key: false, reasons: Rc::default() }
    }

    fn note(&self, what: &str, p: Option<&ToolPointer>) {
        let at = p.map(|p| format!(" doc {:.2},{:.2}{}", p.doc.x, p.doc.y, if p.snapped { " snapped" } else { "" })).unwrap_or_default();
        self.log.borrow_mut().push(format!("{what}{at}"));
    }
}

impl CanvasTool for Probe {
    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.note("press", Some(pointer));
        self.action
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.note("drag", Some(pointer));
    }

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.note("release", Some(pointer));
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.reasons.borrow_mut().push(reason);
        self.note("cancel", None);
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        self.note(&format!("key {:?}", key.key), None);
        self.takes_key
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn snap_gesture(&self) -> Option<SnapGesture> {
        self.snap
    }

    fn autoscrolls(&self) -> bool {
        self.autoscrolls
    }
}

struct Rig {
    view: CanvasView,
    tool: Box<dyn CanvasTool>,
    targets: Option<SnapTargets>,
    document: Size,
    /// An overlay that takes every primary press before the host.
    overlay: bool,
    /// The click count the overlay saw at each press it took.
    overlay_clicks: Vec<u32>,
    /// A text field next to the canvas, and whether it holds the focus.
    text: Option<String>,
    focus_text: bool,
    focus_canvas: bool,
    id: Option<CanvasId>,
    navigation: Option<NavigationOptions>,
    responses: Vec<ToolHostResponse>,
}

impl Rig {
    fn new(tool: impl CanvasTool + 'static) -> Self {
        Self {
            view: CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0),
            tool: Box::new(tool),
            targets: None,
            document: Size::ZERO,
            overlay: false,
            overlay_clicks: Vec::new(),
            text: None,
            focus_text: false,
            focus_canvas: false,
            id: None,
            navigation: None,
            responses: Vec::new(),
        }
    }

    fn last(&self) -> &ToolHostResponse {
        self.responses.last().expect("a pass ran")
    }

    fn gestures(&self) -> Vec<ToolGesture> {
        self.responses.iter().filter_map(|r| r.gesture).collect()
    }
}

fn harness(rig: Rig, step_dt: f32) -> Harness<'static, Rig> {
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(step_dt)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, rig: &mut Rig| {
                let rect = rig.view.rect;
                let canvas_id = CanvasId::new(ui, "canvas");
                let canvas = canvas_id.interact(ui, rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                if std::mem::take(&mut rig.focus_canvas) {
                    canvas.request_focus();
                }
                if let Some(text) = rig.text.as_mut() {
                    let edit = ui.put(Rect::from_min_size(pos2(5.0, 5.0), vec2(80.0, 20.0)), egui::TextEdit::singleline(text));
                    edit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Name"));
                    if std::mem::take(&mut rig.focus_text) {
                        edit.request_focus();
                    }
                }
                if rig.overlay && ui.input(|i| i.pointer.primary_pressed()) {
                    Claim::take(ui.ctx(), canvas.id, canvas.id.with("overlay"));
                    if let Some(press) = PrimaryPress::current(ui.ctx()) {
                        rig.overlay_clicks.push(press.clicks(ui.ctx()));
                    }
                }
                let mut host = ToolHost::new(canvas_id).document(rig.document);
                if let Some(prefs) = rig.navigation {
                    host = host.navigation(prefs);
                }
                if let Some(targets) = rig.targets.as_ref() {
                    host = host.snap(targets);
                }
                let r = host.show(ui, &rig.view, &canvas, &mut *rig.tool);
                rig.id = Some(r.id);
                if let Some(view) = r.view {
                    rig.view = view;
                }
                rig.responses.push(r);
            },
            rig,
        );
    h.step();
    h
}

fn button(at: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers }
}

fn move_to(h: &mut Harness<'_, Rig>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
}

fn press(h: &mut Harness<'_, Rig>, at: Pos2, modifiers: Modifiers) {
    move_to(h, at);
    h.event(button(at, true, modifiers));
    h.step();
}

fn release(h: &mut Harness<'_, Rig>, at: Pos2, modifiers: Modifiers) {
    h.event(button(at, false, modifiers));
    h.step();
}

fn key(h: &mut Harness<'_, Rig>, key: Key) {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.step();
}

fn log(log: &Log) -> Vec<String> {
    log.borrow().clone()
}

fn focused(h: &mut Harness<'_, Rig>) {
    h.state_mut().focus_canvas = true;
    h.step();
}

// A drag in a zoomed, turned view arrives as the mapped document points, Began/Moved/Ended.
#[test]
fn the_tool_gets_press_drag_release_in_document_units() {
    let calls = Log::default();
    let mut rig = Rig::new(Probe::new(&calls));
    rig.view.zoom = 2.0;
    rig.view.rotation = 90.0;
    let view = rig.view;
    let mut h = harness(rig, 0.05);
    let (a, b) = (pos2(150.0, 100.0), pos2(250.0, 180.0));
    press(&mut h, a, Modifiers::NONE);
    move_to(&mut h, b);
    release(&mut h, b, Modifiers::NONE);
    let at = |p: Pos2| {
        let d = view.to_doc(p);
        format!("{:.2},{:.2}", d.x, d.y)
    };
    let calls = log(&calls);
    assert_eq!(calls.first().map(String::as_str), Some(format!("press doc {}", at(a)).as_str()));
    assert!(calls.contains(&format!("drag doc {}", at(b))), "{calls:?}");
    assert_eq!(calls.last().map(String::as_str), Some(format!("release doc {}", at(b)).as_str()));
    let gestures = h.state().gestures();
    assert_eq!(gestures.first(), Some(&ToolGesture::Begin));
    assert!(gestures.contains(&ToolGesture::Update));
    assert_eq!(gestures.last(), Some(&ToolGesture::Commit));
    assert!(!ToolHost::in_gesture(&h.ctx, h.state().id.expect("a pass ran")));
}

// A press a guide claimed never reaches press.
#[test]
fn an_overlay_claim_keeps_the_press_from_the_tool() {
    let calls = Log::default();
    let mut rig = Rig::new(Probe::new(&calls));
    rig.overlay = true;
    let mut h = harness(rig, 0.05);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    move_to(&mut h, pos2(140.0, 100.0));
    release(&mut h, pos2(140.0, 100.0), Modifiers::NONE);
    assert!(log(&calls).is_empty(), "{:?}", log(&calls));
    assert!(h.state().gestures().is_empty());
}

// Esc calls cancel, gesture Cancelled, no release.
#[test]
fn esc_during_a_drag_cancels_the_gesture() {
    let calls = Log::default();
    let mut h = harness(Rig::new(Probe::new(&calls)), 0.05);
    focused(&mut h);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    move_to(&mut h, pos2(140.0, 120.0));
    key(&mut h, Key::Escape);
    assert_eq!(h.state().last().gesture, Some(ToolGesture::Cancel));
    assert_eq!(h.state().last().took_key.map(|k| k.key), Some(Key::Escape));
    release(&mut h, pos2(140.0, 120.0), Modifiers::NONE);
    let calls = log(&calls);
    assert_eq!(calls.iter().filter(|c| *c == "cancel").count(), 1, "{calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("release") || c.starts_with("key")), "{calls:?}");
}

fn targets() -> SnapTargets {
    let mut t = SnapTargets::new();
    t.add(SnapTarget { vertical: true, position: 100.0, kind: SnapKind::Guide, span: None });
    t
}

// A point within 8 / zoom of a target snaps; with Ctrl it stays raw.
#[test]
fn ctrl_turns_snapping_off_during_a_drag() {
    let calls = Log::default();
    let mut rig = Rig::new(Probe { snap: Some(SnapGesture::Point), ..Probe::new(&calls) });
    rig.targets = Some(targets());
    let mut h = harness(rig, 0.05);
    // The view maps screen x to document x directly (centre 200 at 200).
    press(&mut h, pos2(50.0, 100.0), Modifiers::NONE);
    move_to(&mut h, pos2(103.0, 100.0));
    h.event(Event::ModifiersChanged(Modifiers::CTRL));
    move_to(&mut h, pos2(104.0, 100.0));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    release(&mut h, pos2(104.0, 100.0), Modifiers::NONE);
    let drags: Vec<String> = log(&calls).into_iter().filter(|c| c.starts_with("drag")).collect();
    assert!(drags.contains(&"drag doc 100.00,100.00 snapped".to_owned()), "{drags:?}");
    assert!(drags.contains(&"drag doc 104.00,100.00".to_owned()), "{drags:?}");
    assert_eq!(ToolHost::snap_tolerance(&CanvasView::new(Rect::ZERO, Point::ZERO, 2.0), Modifiers::NONE, false), Some(4.0));
    assert_eq!(ToolHost::snap_tolerance(&CanvasView::new(Rect::ZERO, Point::ZERO, 2.0), Modifiers::CTRL, false), None);
    assert_eq!(ToolHost::snap_tolerance(&CanvasView::new(Rect::ZERO, Point::ZERO, 2.0), Modifiers::CTRL, true), Some(4.0));
}

// scrolled_by is Some for a marquee-like tool, None when autoscrolls is false.
#[test]
fn a_drag_past_the_edge_autoscrolls_for_tools_that_ask() {
    for autoscrolls in [true, false] {
        let calls = Log::default();
        let mut rig = Rig::new(Probe { autoscrolls, ..Probe::new(&calls) });
        rig.document = Size::new(1000.0, 1000.0);
        let start = rig.view;
        let mut h = harness(rig, 0.05);
        press(&mut h, pos2(300.0, 150.0), Modifiers::NONE);
        move_to(&mut h, pos2(480.0, 150.0));
        move_to(&mut h, pos2(480.0, 150.0));
        let scrolled: Vec<_> = h.state().responses.iter().filter_map(|r| r.scrolled_by).collect();
        if autoscrolls {
            assert!(scrolled.iter().all(|s| s.x > 0.0), "{scrolled:?}");
            assert!(!scrolled.is_empty());
            // The scroll is already in the view the response hands back.
            assert!(h.state().view.center.x > start.center.x);
            assert_eq!(h.state().last().view.map(|v| v.center), Some(h.state().view.center));
        } else {
            assert!(scrolled.is_empty());
            assert_eq!(h.state().view, start);
        }
    }
}

// inject Down/Move/Up gives the same calls as the OS pointer.
#[test]
fn injected_input_drives_the_tool_like_the_pointer() {
    let os = Log::default();
    let mut h = harness(Rig::new(Probe::new(&os)), 0.05);
    let (a, b) = (pos2(120.0, 90.0), pos2(220.0, 140.0));
    press(&mut h, a, Modifiers::NONE);
    move_to(&mut h, b);
    release(&mut h, b, Modifiers::NONE);

    let injected = Log::default();
    let mut h = harness(Rig::new(Probe::new(&injected)), 0.05);
    let id = h.state().id.expect("a pass ran");
    let view = h.state().view;
    let (da, db) = (view.to_doc(a), view.to_doc(b));
    let m = Modifiers::NONE;
    ToolHost::inject(&h.ctx, id, ToolInput::Down { doc: da, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers: m });
    h.step();
    assert!(ToolHost::in_gesture(&h.ctx, id));
    ToolHost::inject(&h.ctx, id, ToolInput::Move { doc: db, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers: m });
    h.step();
    ToolHost::inject(&h.ctx, id, ToolInput::Up { doc: db, modifiers: m });
    h.step();
    assert_eq!(log(&os), log(&injected));
    assert!(!ToolHost::in_gesture(&h.ctx, id));
}

// ToolHost::cancel runs the tool's cancel at once; nothing is left for the next pass.
#[test]
fn cancel_from_the_app_runs_the_tools_cancel() {
    let calls = Log::default();
    let mut h = harness(Rig::new(Probe::new(&calls)), 0.05);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    let (ctx, id) = (h.ctx.clone(), h.state().id.expect("a pass ran"));
    assert!(ToolHost::in_gesture(&ctx, id));
    ToolHost::cancel(&ctx, id, &mut *h.state_mut().tool, CancelReason::Removed);
    assert_eq!(log(&calls).iter().filter(|c| *c == "cancel").count(), 1, "{:?}", log(&calls));
    assert!(!ToolHost::in_gesture(&ctx, id));
    h.step();
    assert_eq!(h.state().last().gesture, None, "the cancel was the app's, the pass reports nothing");
    release(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    let calls = log(&calls);
    assert_eq!(calls.iter().filter(|c| *c == "cancel").count(), 1, "{calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("release")), "{calls:?}");
}

// The app switches tools in the middle of a drag: the tool that has the gesture is cancelled and
// the new tool never sees the old gesture.
#[test]
fn a_tool_switch_mid_drag_cancels_the_old_tool_not_the_new_one() {
    let (old, new) = (Log::default(), Log::default());
    let mut h = harness(Rig::new(Probe::new(&old)), 0.05);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    move_to(&mut h, pos2(120.0, 110.0));
    let (ctx, id) = (h.ctx.clone(), h.state().id.expect("a pass ran"));
    ToolHost::cancel(&ctx, id, &mut *h.state_mut().tool, CancelReason::Removed);
    h.state_mut().tool = Box::new(Probe::new(&new));
    move_to(&mut h, pos2(140.0, 120.0));
    release(&mut h, pos2(140.0, 120.0), Modifiers::NONE);
    let (old, new) = (log(&old), log(&new));
    assert_eq!(old.last().map(String::as_str), Some("cancel"), "{old:?}");
    assert!(!old.iter().any(|c| c.starts_with("release")), "{old:?}");
    assert!(new.is_empty(), "the new tool gets no call of the old gesture: {new:?}");
}

// The Hand puts the view back when the app cancels its drag, and the host hands that view over.
#[test]
fn cancelling_a_hand_drag_returns_the_view_it_started_from() {
    let mut h = harness(Rig::new(HandTool::new(DOC, NavigationOptions::default())), 0.05);
    let start = h.state().view;
    press(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    move_to(&mut h, pos2(260.0, 190.0));
    assert!((h.state().view.center - start.center).hypot() > 1.0, "the drag pans");
    let (ctx, id) = (h.ctx.clone(), h.state().id.expect("a pass ran"));
    let back = ToolHost::cancel(&ctx, id, &mut *h.state_mut().tool, CancelReason::Removed).expect("the Hand asks for its press view");
    assert_eq!(back.center, start.center);
    assert_eq!(ToolHost::cancel(&ctx, id, &mut *h.state_mut().tool, CancelReason::Removed), None, "no gesture is left");
}

// A flick released with the Hand glides on after the app passes another tool (Space came up).
#[test]
fn a_flick_keeps_gliding_whatever_the_tool() {
    let prefs = NavigationOptions { overscroll: true, ..NavigationOptions::default() };
    let mut rig = Rig::new(HandTool::new(DOC, prefs));
    rig.navigation = Some(prefs);
    rig.document = DOC;
    let mut h = harness(rig, 0.05);
    press(&mut h, pos2(100.0, 150.0), Modifiers::NONE);
    for x in [120.0, 140.0, 160.0, 180.0] {
        move_to(&mut h, pos2(x, 150.0));
    }
    release(&mut h, pos2(180.0, 150.0), Modifiers::NONE);
    h.state_mut().tool = Box::new(Probe::new(&Log::default()));
    let mut centres = vec![h.state().view.center];
    for _ in 0..3 {
        h.step();
        centres.push(h.state().view.center);
    }
    for pair in centres.windows(2) {
        assert!((pair[1] - pair[0]).hypot() > 0.1, "the view keeps moving: {centres:?}");
    }
}

/// Records the click count of each press; presses are whole clicks.
struct Counter(Rc<RefCell<Vec<u32>>>);

impl CanvasTool for Counter {
    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.0.borrow_mut().push(pointer.clicks);
        ToolAction::Done
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}
}

// Presses within egui's double-click delay and distance count up; a slow or distant one starts again.
#[test]
fn quick_presses_in_one_spot_count_their_clicks() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Rig::new(Counter(seen.clone())), 0.05);
    let at = pos2(100.0, 100.0);
    for _ in 0..3 {
        press(&mut h, at, Modifiers::NONE);
        release(&mut h, at, Modifiers::NONE);
    }
    for _ in 0..10 {
        h.step();
    }
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    press(&mut h, pos2(200.0, 200.0), Modifiers::NONE);
    release(&mut h, pos2(200.0, 200.0), Modifiers::NONE);
    assert_eq!(*seen.borrow(), vec![1, 2, 3, 1, 1]);
}

/// Keeps the canvas's keys for itself, as a text editor on the canvas does.
struct Typist;

impl CanvasTool for Typist {
    fn press(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}

    fn key(&mut self, _key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        cx.keep_keys();
        true
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}
}

// The key router and the app's shortcuts ask KeyClaim: typed keys are the tool's, chords stay the app's.
#[test]
fn a_tool_that_keeps_the_keys_claims_them_from_the_shortcuts() {
    let mut h = harness(Rig::new(Typist), 0.05);
    focused(&mut h);
    key(&mut h, Key::A);
    let claimed = |h: &Harness<'_, Rig>, modifiers, key| KeyClaim::claimed(&h.ctx, &KeyboardShortcut::new(modifiers, key));
    assert!(claimed(&h, Modifiers::NONE, Key::Space), "Space types");
    assert!(claimed(&h, Modifiers::SHIFT, Key::A), "so do shifted letters");
    assert!(!claimed(&h, Modifiers::COMMAND, Key::Z), "Cmd+Z stays the app's");
}

// A key with a TextEdit focused never reaches key; took_key reports a taken key.
#[test]
fn keys_reach_the_tool_only_while_the_canvas_has_the_keyboard() {
    let calls = Log::default();
    let mut rig = Rig::new(Probe { takes_key: true, ..Probe::new(&calls) });
    rig.text = Some(String::new());
    let mut h = harness(rig, 0.05);
    focused(&mut h);
    key(&mut h, Key::Delete);
    assert_eq!(log(&calls), ["key Delete"]);
    assert_eq!(h.state().last().took_key.map(|k| k.key), Some(Key::Delete));

    h.state_mut().focus_text = true;
    h.step();
    h.step();
    key(&mut h, Key::Backspace);
    assert_eq!(log(&calls), ["key Delete"]);
    assert_eq!(h.state().last().took_key, None);
}

// ToolAction::Ignored fills ignored_press.
#[test]
fn an_ignored_press_goes_back_to_the_app() {
    let calls = Log::default();
    let mut h = harness(Rig::new(Probe { action: ToolAction::Ignored, ..Probe::new(&calls) }), 0.05);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    let ignored = h.state().responses.iter().find_map(|r| r.ignored_press).expect("the press comes back");
    assert_eq!(ignored.screen, pos2(100.0, 100.0));
    assert!(h.state().gestures().is_empty());
    assert!(!ToolHost::in_gesture(&h.ctx, h.state().id.expect("a pass ran")));
}

fn nav(animated: bool, scrubby: bool) -> NavigationOptions {
    NavigationOptions { animated_zoom: animated, scrubby_zoom: scrubby, ..NavigationOptions::default() }
}

const DOC: Size = Size::new(1000.0, 800.0);

// The Zoom tool's click steps the ladder about the clicked point; Alt-click steps out.
#[test]
fn the_zoom_tool_clicks_in_and_alt_clicks_out() {
    for (modifiers, dir) in [(Modifiers::NONE, 1), (Modifiers::ALT, -1)] {
        let mut rig = Rig::new(ZoomTool::new(DOC, nav(false, true)));
        rig.view.zoom = 0.5;
        let before = rig.view;
        let mut h = harness(rig, 0.05);
        let at = pos2(120.0, 90.0);
        h.event(Event::ModifiersChanged(modifiers));
        press(&mut h, at, modifiers);
        release(&mut h, at, modifiers);
        let after = h.state().view;
        let expect = ZoomLadder::step(zoom::device_zoom(&before, 1.0), dir, DOC);
        assert!((zoom::device_zoom(&after, 1.0) - expect).abs() < 1e-9, "{dir}: {} vs {expect}", zoom::device_zoom(&after, 1.0));
        let (a, b) = (before.to_doc(at), after.to_doc(at));
        assert!((a - b).hypot() < 1e-6, "the clicked point stays under the pointer");
    }
}

// A zoom rectangle (Scrubby Zoom off) zooms to fit it, centred on it.
#[test]
fn the_zoom_tool_zooms_to_a_dragged_rectangle() {
    let mut h = harness(Rig::new(ZoomTool::new(DOC, nav(false, false))), 0.05);
    let before = h.state().view;
    let (a, b) = (pos2(100.0, 50.0), pos2(300.0, 200.0));
    press(&mut h, a, Modifiers::NONE);
    move_to(&mut h, b);
    release(&mut h, b, Modifiers::NONE);
    let after = h.state().view;
    assert!((after.zoom / before.zoom - 2.0).abs() < 1e-9, "{}", after.zoom);
    let centre = before.to_doc(pos2(200.0, 125.0));
    assert!((after.center - centre).hypot() < 1e-6, "{:?} vs {centre:?}", after.center);
}

// Esc cancels a Hand drag back to the view at the press.
#[test]
fn esc_puts_a_hand_drag_back_to_its_press_view() {
    let mut h = harness(Rig::new(HandTool::new(DOC, NavigationOptions::default())), 0.05);
    focused(&mut h);
    let start = h.state().view;
    press(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    move_to(&mut h, pos2(260.0, 190.0));
    assert!((h.state().view.center - start.center).hypot() > 1.0, "the drag pans");
    key(&mut h, Key::Escape);
    assert_eq!(h.state().view, start);
    assert_eq!(h.state().last().gesture, Some(ToolGesture::Cancel));
}

// The Zoom tool's eased click runs under the id the app made before the canvas existed.
#[test]
fn a_zoom_click_eases_under_the_canvas_id_the_app_holds() {
    let mut h = harness(Rig::new(ZoomTool::new(DOC, nav(true, false))), 0.05);
    let at = pos2(120.0, 90.0);
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    assert!(ZoomAnimation::running(&h.ctx, h.state().id.expect("a pass ran")));
}

// A press held still past the delay zooms continuously about the press point; the release is no click.
#[test]
fn the_zoom_tool_zooms_while_the_press_is_held() {
    let mut h = harness(Rig::new(ZoomTool::new(DOC, nav(true, true))), 0.1);
    let before = h.state().view;
    let at = pos2(120.0, 90.0);
    press(&mut h, at, Modifiers::NONE);
    for _ in 0..8 {
        h.step();
    }
    let held = h.state().view;
    assert!(held.zoom > before.zoom * 1.2, "{} vs {}", held.zoom, before.zoom);
    assert!((before.to_doc(at) - held.to_doc(at)).hypot() < 1e-6, "the press point stays put");
    release(&mut h, at, Modifiers::NONE);
    assert_eq!(h.state().view.zoom, held.zoom, "the release of a hold steps nothing");
}

/// A tool that takes the secondary button while Alt is held, as well as the primary (the brush
/// resize chord and Alt + click).
struct AltResize {
    log: Log,
    overridden: bool,
}

impl AltResize {
    fn note(&self, what: &str, p: &ToolPointer) {
        self.log.borrow_mut().push(format!("{what} {:?} x{}", p.button, p.clicks));
    }
}

impl CanvasTool for AltResize {
    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.note("press", pointer);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.note("drag", pointer);
    }

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.note("release", pointer);
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn takes(&self, button: PointerButton, modifiers: Modifiers) -> bool {
        button == self.button() || (self.overridden && button == PointerButton::Secondary && modifiers.alt)
    }
}

fn button_of(at: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button, pressed, modifiers }
}

// A brush-type tool gets both the Alt + right press (the resize chord, `Secondary`) and the Alt +
// primary press (`Primary`); without Alt the secondary press is not the tool's.
#[test]
fn takes_gives_a_brush_type_tool_both_buttons_while_alt_is_held() {
    let calls = Log::default();
    let mut h = harness(Rig::new(AltResize { log: calls.clone(), overridden: true }), 0.05);
    let at = pos2(100.0, 100.0);
    move_to(&mut h, at);
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(button_of(at, PointerButton::Secondary, true, Modifiers::ALT));
    h.step();
    assert_eq!(log(&calls), ["press Secondary x1"]);
    h.event(button_of(at, PointerButton::Secondary, false, Modifiers::ALT));
    h.step();
    assert_eq!(log(&calls).last().map(String::as_str), Some("release Secondary x1"));
    h.event(button_of(at, PointerButton::Primary, true, Modifiers::ALT));
    h.step();
    assert!(log(&calls).last().is_some_and(|c| c.starts_with("press Primary")), "{:?}", log(&calls));
    h.event(button_of(at, PointerButton::Primary, false, Modifiers::ALT));
    h.step();
    // Without Alt the secondary press is not the tool's.
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    let before = log(&calls).len();
    h.event(button_of(at, PointerButton::Secondary, true, Modifiers::NONE));
    h.step();
    assert_eq!(log(&calls).len(), before, "{:?}", log(&calls));
}

// A tool that doesn't override `takes` gets only its `button()`, with or without Alt.
#[test]
fn takes_default_is_the_tools_button() {
    let calls = Log::default();
    let mut h = harness(Rig::new(AltResize { log: calls.clone(), overridden: false }), 0.05);
    let at = pos2(100.0, 100.0);
    move_to(&mut h, at);
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(button_of(at, PointerButton::Secondary, true, Modifiers::ALT));
    h.step();
    assert!(log(&calls).is_empty(), "the secondary press is not the tool's");
    h.event(button_of(at, PointerButton::Secondary, false, Modifiers::ALT));
    h.step();
    h.event(button_of(at, PointerButton::Primary, true, Modifiers::ALT));
    h.step();
    assert_eq!(log(&calls), ["press Primary x1"]);
}

// Alt released mid-gesture: the gesture still ends on the secondary button's release, not the
// primary's, and drags keep arriving meanwhile.
#[test]
fn a_gesture_keeps_its_button_after_alt_is_released() {
    let calls = Log::default();
    let mut h = harness(Rig::new(AltResize { log: calls.clone(), overridden: true }), 0.05);
    let at = pos2(100.0, 100.0);
    move_to(&mut h, at);
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(button_of(at, PointerButton::Secondary, true, Modifiers::ALT));
    h.step();
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    move_to(&mut h, pos2(140.0, 100.0));
    move_to(&mut h, pos2(160.0, 100.0));
    let drags = log(&calls).iter().filter(|c| c.starts_with("drag Secondary")).count();
    assert!(drags >= 2, "{:?}", log(&calls));
    assert!(!log(&calls).iter().any(|c| c.starts_with("release")), "still running");
    h.event(button_of(pos2(160.0, 100.0), PointerButton::Secondary, false, Modifiers::NONE));
    h.step();
    assert_eq!(log(&calls).last().map(String::as_str), Some("release Secondary x1"));
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

const PART_KEYS: [u64; 3] = [1, 2, 3];

/// A tool made of three parts along the top of the canvas; Page keys pick, the arrows log.
struct Parted {
    picked: Option<u64>,
    parts: bool,
    node: Option<ToolNode>,
    keys: Log,
    /// `ToolContext::parts_focused` at each `key` call.
    focus: Rc<RefCell<Vec<bool>>>,
}

impl Parted {
    fn new(parts: bool, keys: &Log) -> Self {
        Self { picked: None, parts, node: None, keys: keys.clone(), focus: Rc::default() }
    }
}

impl CanvasTool for Parted {
    fn press(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        self.focus.borrow_mut().push(cx.parts_focused);
        self.keys.borrow_mut().push(format!("{:?} on {:?}", key.key, self.picked));
        true
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn parts(&self, _cx: &ToolContext<'_>) -> Option<ToolParts> {
        self.parts.then(|| {
            let parts = PART_KEYS
                .iter()
                .map(|&key| Part {
                    key,
                    name: format!("Handle {key}"),
                    value: Some(format!("{key} px")),
                    rect: Rect::from_center_size(pos2(40.0 + key as f32 * 40.0, 40.0), vec2(12.0, 12.0)),
                    selected: false,
                })
                .collect();
            ToolParts::new("Frame", parts).value("3 handles")
        })
    }

    fn picked(&self) -> Option<u64> {
        self.picked
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.picked = key;
    }

    fn node(&self, _cx: &ToolContext<'_>) -> Option<ToolNode> {
        self.node.clone()
    }
}

// A tool with parts is one Tab stop: Tab focuses the group, Page Down picks through `pick`, the
// arrows then reach the tool's `key` and the host draws the focus ring round the picked part.
#[test]
fn a_tool_with_parts_is_one_tab_stop_whose_page_keys_pick() {
    let keys = Log::default();
    let mut h = harness(Rig::new(Parted::new(true, &keys)), 0.05);
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(log(&keys).last().map(String::as_str), Some("ArrowRight on Some(2)"), "Tab picked the first part, Page Down the second");
}

// The keys the group used to pick a part do not reach the tool a second time, and
// `ToolContext::parts_focused` is true only while the group has the keyboard.
#[test]
fn page_keys_that_pick_a_part_do_not_reach_the_tool_and_parts_focused_follows_the_group() {
    let keys = Log::default();
    let parted = Parted::new(true, &keys);
    let focus = parted.focus.clone();
    let mut h = harness(Rig::new(parted), 0.05);
    focused(&mut h);
    key(&mut h, Key::ArrowRight);
    assert_eq!(*focus.borrow(), [false], "the canvas has the focus, the group does not: {:?}", log(&keys));
    h.key_press(Key::Tab);
    h.run();
    key(&mut h, Key::PageDown);
    key(&mut h, Key::Home);
    key(&mut h, Key::ArrowRight);
    let keys = log(&keys);
    assert!(!keys.iter().any(|k| k.starts_with("PageDown") || k.starts_with("Home")), "{keys:?}");
    assert_eq!(keys.last().map(String::as_str), Some("ArrowRight on Some(1)"), "{keys:?}");
    assert_eq!(focus.borrow().last(), Some(&true), "{:?}", focus.borrow());
}

// Esc reaches the tool's `cancel` as `Escape`, and the app's own reason through `ToolHost::cancel`
// as given. (A lost pointer, `FocusLost`, cannot be produced with kittest's input.)
#[test]
fn each_cancel_reason_reaches_the_tool() {
    let one = |act: &dyn Fn(&mut Harness<'static, Rig>)| {
        let probe = Probe::new(&Log::default());
        let reasons = probe.reasons.clone();
        let mut h = harness(Rig::new(probe), 0.05);
        focused(&mut h);
        press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
        move_to(&mut h, pos2(140.0, 120.0));
        act(&mut h);
        reasons.borrow().clone()
    };
    assert_eq!(one(&|h| key(h, Key::Escape)), [CancelReason::Escape]);
    assert_eq!(
        one(&|h| {
            let (ctx, id) = (h.ctx.clone(), h.state().id.expect("a pass ran"));
            ToolHost::cancel(&ctx, id, &mut *h.state_mut().tool, CancelReason::UndoDuringGesture);
        }),
        [CancelReason::UndoDuringGesture]
    );
}

// The group and each part are in the AccessKit tree with the tool's names and values; the picked
// part is the group's active descendant.
#[test]
fn tool_parts_reach_the_accesskit_tree() {
    let keys = Log::default();
    let mut parted = Parted::new(true, &keys);
    parted.picked = Some(3);
    let mut h = harness(Rig::new(parted), 0.05);
    h.run();
    let group = h.get_by_label("Frame");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("3 handles"));
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default()))
        .collect();
    let want: Vec<_> = PART_KEYS.iter().map(|k| (format!("Handle {k}"), format!("{k} px"))).collect();
    assert_eq!(objects, want);
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Handle 3"));
}

// A claimed press released on a part gives the group focus; a tool without parts adds no Tab stop.
#[test]
fn a_press_on_a_part_focuses_the_group_and_tools_without_parts_add_no_stop() {
    struct Claims(Parted);
    impl CanvasTool for Claims {
        fn press(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
            ToolAction::Claimed
        }
        fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}
        fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}
        fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}
        fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
            self.0.key(key, cx)
        }
        fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}
        fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
            self.0.parts(cx)
        }
    }
    let keys = Log::default();
    let mut h = harness(Rig::new(Claims(Parted::new(true, &keys))), 0.05);
    let on_part = pos2(80.0, 40.0);
    press(&mut h, on_part, Modifiers::NONE);
    release(&mut h, on_part, Modifiers::NONE);
    key(&mut h, Key::ArrowLeft);
    assert_eq!(keys.borrow().len(), 1, "the arrow reached the tool: the group has the focus");

    // Without parts the canvas is the only stop and the group is not in the tree.
    let keys = Log::default();
    let mut h = harness(Rig::new(Parted::new(false, &keys)), 0.05);
    h.run();
    assert!(h.query_by_label("Frame").is_none());
}

// A press the host took and one an overlay took share one click run: `ToolPointer::clicks` and
// `PrimaryPress::clicks` agree for a double click (after K8's `click_count`).
#[test]
fn the_host_and_an_overlay_count_one_click_run() {
    let calls = Log::default();
    let mut rig = Rig::new(AltResize { log: calls.clone(), overridden: false });
    rig.overlay = true;
    let mut h = harness(rig, 0.02);
    let at = pos2(100.0, 100.0);
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    assert_eq!(h.state().overlay_clicks, [1]);
    h.state_mut().overlay = false;
    press(&mut h, at, Modifiers::NONE);
    assert_eq!(log(&calls), ["press Primary x2"], "the host continues the overlay's run");
    release(&mut h, at, Modifiers::NONE);
}

// A tool's `node` is one GraphicsObject under the canvas node with its name, value and screen
// bounds; a tool returning `None` adds no node.
#[test]
fn a_tools_node_is_a_graphics_object_under_the_canvas() {
    let keys = Log::default();
    let mut tool = Parted::new(false, &keys);
    tool.node = Some(ToolNode::new("Outline").value("5 points").bounds(kurbo::Rect::new(100.0, 50.0, 180.0, 90.0)));
    let mut h = harness(Rig::new(tool), 0.05);
    h.run();
    let node = h.get_by_label("Outline");
    assert_eq!(node.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("5 points"));
    let bounds = node.accesskit_node().raw_bounds().expect("bounds");
    assert!((bounds.x0 - 100.0).abs() < 1.0 && (bounds.x1 - 180.0).abs() < 1.0, "{bounds:?}");

    let mut h = harness(Rig::new(Parted::new(false, &keys)), 0.05);
    h.run();
    assert!(h.query_by_role(Role::GraphicsObject).is_none());
}
