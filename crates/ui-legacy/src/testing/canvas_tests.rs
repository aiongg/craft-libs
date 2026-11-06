//! Behaviour tests of `testing::canvas`: the rig's events reach a tool through the host's OS path.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Key, Modifiers, Painter, PointerButton, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::canvas;
use crate::canvas::{CanvasId, CanvasTool, CanvasView, ToolAction, ToolContext, ToolHost, ToolKey, ToolPointer};
use crate::edit::CancelReason;
use crate::theme::{Base, install};

type Log = Rc<RefCell<Vec<String>>>;

/// Takes the primary button, and the secondary one while Alt is held; logs each call.
struct Chord {
    log: Log,
}

impl CanvasTool for Chord {
    fn takes(&self, button: PointerButton, modifiers: Modifiers) -> bool {
        button == PointerButton::Primary || (button == PointerButton::Secondary && modifiers.alt)
    }

    fn press(&mut self, p: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.log.borrow_mut().push(format!("press {:?} clicks {} at {:.0},{:.0}", p.button, p.clicks, p.doc.x, p.doc.y));
        ToolAction::Claimed
    }

    fn drag(&mut self, _p: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.log.borrow_mut().push("drag".into());
    }

    fn release(&mut self, p: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.log.borrow_mut().push(format!("release at {:.0},{:.0}", p.doc.x, p.doc.y));
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.log.borrow_mut().push(format!("cancel {reason:?}"));
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        self.log.borrow_mut().push(format!("key {:?} pressed {}", key.key, key.pressed));
        true
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}
}

struct State {
    tool: Chord,
    view: CanvasView,
    focus: bool,
}

/// A harness whose UI is one canvas under a host, the view mapping screen points to the same
/// document points.
fn harness(log: &Log) -> Harness<'static, State> {
    let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, state: &mut State| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let response = canvas_id.interact(ui, state.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(&mut state.focus) {
                response.request_focus();
            }
            let _ = ToolHost::new(canvas_id).show(ui, &state.view, &response, &mut state.tool);
        },
        State { tool: Chord { log: log.clone() }, view, focus: true },
    );
    h.step();
    h
}

fn calls(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.borrow_mut())
}

// A drag arrives as one press, moves and one release in document units; a secondary press the tool
// takes only with Alt starts nothing without it and runs with it; a key reaches the focused tool.
#[test]
fn the_rig_drives_a_tool_through_takes_and_keys() {
    let log = Log::default();
    let mut h = harness(&log);
    canvas::drag(&mut h, pos2(100.0, 80.0), pos2(180.0, 120.0), Modifiers::NONE);
    let drag = calls(&log);
    assert_eq!(drag.first().map(String::as_str), Some("press Primary clicks 1 at 100,80"), "{drag:?}");
    assert!(drag.iter().filter(|c| *c == "drag").count() >= 1, "{drag:?}");
    assert_eq!(drag.last().map(String::as_str), Some("release at 180,120"), "{drag:?}");

    canvas::press_button(&mut h, pos2(50.0, 50.0), PointerButton::Secondary, Modifiers::NONE);
    canvas::release_button(&mut h, pos2(50.0, 50.0), PointerButton::Secondary, Modifiers::NONE);
    assert!(calls(&log).is_empty(), "the tool takes no plain secondary press");

    canvas::press_button(&mut h, pos2(50.0, 50.0), PointerButton::Secondary, Modifiers::ALT);
    canvas::release_button(&mut h, pos2(50.0, 50.0), PointerButton::Secondary, Modifiers::ALT);
    assert_eq!(calls(&log), ["press Secondary clicks 1 at 50,50", "release at 50,50"]);

    canvas::key(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(calls(&log), ["key ArrowLeft pressed true", "key ArrowLeft pressed false"]);
}

// A button that goes up in a pass the host does not see ends the gesture as a lost pointer.
#[test]
fn a_release_the_host_missed_cancels_with_focus_lost() {
    let log = Log::default();
    let mut h = harness(&log);
    canvas::press(&mut h, pos2(100.0, 80.0), Modifiers::NONE);
    canvas::move_to(&mut h, pos2(120.0, 90.0));
    canvas::lose_pointer(&mut h, pos2(120.0, 90.0), PointerButton::Primary);
    let all = calls(&log);
    assert_eq!(all.last().map(String::as_str), Some("cancel FocusLost"), "{all:?}");
    assert!(!all.iter().any(|c| c.starts_with("release")), "{all:?}");
}
