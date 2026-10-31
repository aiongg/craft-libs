//! The tool host: one tool's presses, drags, releases, keys and paint.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Event, Id, Key, Modifiers, Painter, PointerButton, Pos2, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Size};

use super::navigate::{HandTool, NavigationPrefs, ZoomTool};
use super::zoom::{self, ZoomLadder};
use super::{CanvasTool, CanvasView, Claim, SnapGesture, SnapKind, SnapTarget, SnapTargets, ToolAction, ToolContext, ToolGesture, ToolHost};
use super::{ToolHostResponse, ToolInput, ToolKey, ToolPointer};
use crate::theme::{Base, install};

type Log = Rc<RefCell<Vec<String>>>;

/// A tool that logs what the host calls, with the knobs the tests turn.
struct Probe {
    log: Log,
    action: ToolAction,
    snap: Option<SnapGesture>,
    autoscrolls: bool,
    takes_key: bool,
}

impl Probe {
    fn new(log: &Log) -> Self {
        Self { log: log.clone(), action: ToolAction::Claimed, snap: None, autoscrolls: true, takes_key: false }
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

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
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
    /// A text field next to the canvas, and whether it holds the focus.
    text: Option<String>,
    focus_text: bool,
    focus_canvas: bool,
    id: Id,
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
            text: None,
            focus_text: false,
            focus_canvas: false,
            id: Id::NULL,
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
                let canvas = ui.interact(rect, ui.scope_id().with("canvas"), Sense::click_and_drag() | Sense::FOCUSABLE);
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
                }
                let mut host = ToolHost::new("doc").document(rig.document);
                if let Some(targets) = rig.targets.as_ref() {
                    host = host.snap(targets);
                }
                let r = host.show(ui, &rig.view, &canvas, &mut *rig.tool);
                rig.id = r.id;
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
    assert_eq!(gestures.first(), Some(&ToolGesture::Began));
    assert!(gestures.contains(&ToolGesture::Moved));
    assert_eq!(gestures.last(), Some(&ToolGesture::Ended));
    assert!(!ToolHost::in_gesture(&h.ctx, h.state().id));
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
    assert_eq!(h.state().last().gesture, Some(ToolGesture::Cancelled));
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
    let id = h.state().id;
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

// ToolHost::cancel then show calls cancel once.
#[test]
fn cancel_from_the_app_runs_the_tools_cancel() {
    let calls = Log::default();
    let mut h = harness(Rig::new(Probe::new(&calls)), 0.05);
    press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    let id = h.state().id;
    assert!(ToolHost::in_gesture(&h.ctx, id));
    ToolHost::cancel(&h.ctx, id);
    h.step();
    assert_eq!(h.state().last().gesture, Some(ToolGesture::Cancelled));
    assert!(!ToolHost::in_gesture(&h.ctx, id));
    release(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
    let calls = log(&calls);
    assert_eq!(calls.iter().filter(|c| *c == "cancel").count(), 1, "{calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("release")), "{calls:?}");
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
    assert!(!ToolHost::in_gesture(&h.ctx, h.state().id));
}

fn nav(animated: bool, scrubby: bool) -> NavigationPrefs {
    NavigationPrefs { animated_zoom: animated, scrubby_zoom: scrubby, ..NavigationPrefs::default() }
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
    let mut h = harness(Rig::new(HandTool::new(DOC, NavigationPrefs::default())), 0.05);
    focused(&mut h);
    let start = h.state().view;
    press(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    move_to(&mut h, pos2(260.0, 190.0));
    assert!((h.state().view.center - start.center).hypot() > 1.0, "the drag pans");
    key(&mut h, Key::Escape);
    assert_eq!(h.state().view, start);
    assert_eq!(h.state().last().gesture, Some(ToolGesture::Cancelled));
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
