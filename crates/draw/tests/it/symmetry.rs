//! Painting symmetry: the model's geometry, the menu, the handles of Transform Symmetry and the tool that carries its
//! keys.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, EditEvent, PartKey};
use craft_draw::ui::symmetry::{Symmetry, SymmetryEvent, SymmetryHandles, SymmetryMode, SymmetryTool, SymmetryTransform};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The page, in document units.
const PAGE: Size = Size::new(400.0, 300.0);

/// The parts of the handles.
const SYMMETRY: PartKey = PartKey::new("symmetry");

/// A vertical symmetry at (200, 150): the rotation handle sits at (200, 255).
fn start() -> Symmetry {
    Symmetry::new(SymmetryMode::Vertical, Point::new(200.0, 150.0), 0.0).expect("a valid symmetry")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// Every mode on 100 x 100 and 101 x 53 gives segments inside the page; a horizontal axis at y -5 gives none.
#[trial]
fn segments_clip_to_the_page_and_vanish_when_the_centre_line_misses_it() {
    for size in [Size::new(100.0, 100.0), Size::new(101.0, 53.0)] {
        for mode in SymmetryMode::ALL {
            let s = Symmetry::new(mode, Point::new(size.width / 2.0, size.height / 2.0), 0.0).expect("a valid symmetry");
            let segments = s.segments(size);
            assert_eq!(segments.len(), if mode == SymmetryMode::Dual { 2 } else { 1 });
            for line in segments {
                for p in [line.p0, line.p1] {
                    assert!(p.x >= -1e-9 && p.x <= size.width + 1e-9 && p.y >= -1e-9 && p.y <= size.height + 1e-9);
                }
            }
        }
    }
    let off = Symmetry::new(SymmetryMode::Horizontal, Point::new(50.0, -5.0), 0.0).expect("a valid symmetry");
    assert!(off.segments(Size::new(100.0, 100.0)).is_empty());
}

/// The session, the view and the events the handles reported.
struct Host {
    session: SymmetryTransform,
    view: CanvasView,
    id: egui::Id,
    events: Vec<EditEvent<Symmetry>>,
}

struct Harness {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
}

fn harness() -> Harness {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, by(400.0, 300.0));
    let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
    let host = Rc::new(RefCell::new(Host { session: SymmetryTransform::begin(start()), view, id: egui::Id::NULL, events: Vec::new() }));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        let preview = {
            let mut host = shown.borrow_mut();
            host.view = view;
            host.session.preview()
        };
        let canvas = Canvas::new("Document")
            .view(view)
            .show(ui, |ui, body| SymmetryHandles::new("Symmetry", Bind::given(preview), PAGE).show_at(ui, body.rect));
        if let Some(output) = canvas.inner {
            let mut host = shown.borrow_mut();
            host.id = output.edit.response.id;
            for event in output.edit.events {
                host.session.apply(&event);
                host.events.push(event);
            }
        }
    });
    fixture.settle();
    Harness { fixture, host }
}

impl Harness {
    fn preview(&self) -> Symmetry {
        self.host.borrow().session.preview()
    }

    fn screen(&self, p: Point) -> Pos2 {
        self.host.borrow().view.to_screen(p)
    }

    fn centre(&self) -> Pos2 {
        self.screen(self.preview().center)
    }

    fn handle(&self) -> Pos2 {
        self.screen(self.preview().rotation_handle(PAGE, 0.35))
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        self.fixture.frame(events);
    }

    /// Presses at `from` and moves through the middle to `to`, the button held.
    fn hold(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![pointer::moved(from)]);
        self.frame(vec![pointer::press(from)]);
        self.frame(vec![pointer::moved(from.lerp(to, 0.5))]);
        self.frame(vec![pointer::moved(to)]);
    }

    /// A whole drag with `mods` held.
    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.fixture.hold(mods);
        self.hold(from, to);
        self.frame(vec![pointer::release(to)]);
        self.fixture.hold(Modifiers::NONE);
        self.frame(Vec::new());
    }

    /// A click on the part, which gives the parts group the keyboard.
    fn pick(&mut self, at: Pos2) {
        self.fixture.click_at(at);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(2);
    }
}

fn by(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// A drag from the centre moves it, from the handle turns the axes (Shift in absolute 15 degree steps), from
/// elsewhere changes nothing; each drag is one gesture.
#[trial]
fn centre_moves_handle_turns_with_shift_steps_elsewhere_does_nothing() {
    let mut h = harness();
    let (centre, id) = (h.centre(), h.host.borrow().id);
    assert!(h.fixture.part(id, SYMMETRY.at(0)).is_some() && h.fixture.part(id, SYMMETRY.at(1)).is_some());
    h.drag(centre, centre + by(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.preview().center, Point::new(230.0, 170.0));
    let events = h.host.borrow().events.clone();
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { .. })), "{events:?}");

    // The handle is now at (230, 275): a quarter turn counter-clockwise puts it east of the centre.
    let handle = h.handle();
    h.drag(handle, h.screen(Point::new(335.0, 170.0)), Modifiers::NONE);
    assert!(near(h.preview().rotation, 270.0), "{:?}", h.preview());

    // The handle is now at (335, 170); 53 degrees toward the south turns to 323, which rounds to 330 in 15 degree steps.
    let handle = h.handle();
    let turned = 53.0_f64.to_radians();
    let to = h.screen(Point::new(230.0 + 105.0 * turned.cos(), 170.0 + 105.0 * turned.sin()));
    h.drag(handle, to, Modifiers::SHIFT);
    let rotation = h.preview().rotation;
    assert!(near(rotation.rem_euclid(15.0), 0.0) && near(rotation, 330.0), "{rotation}");

    let (before, count) = (h.preview(), h.host.borrow().events.len());
    h.drag(Pos2::new(50.0, 50.0), Pos2::new(60.0, 70.0), Modifiers::NONE);
    assert_eq!(h.preview(), before);
    assert_eq!(h.host.borrow().events.len(), count, "a press away from the handles is not theirs");
}

/// The arrows on the picked centre move it 1 document unit, Shift 10; on the picked handle Right turns 1 degree
/// clockwise, Shift 15, Left the other way, and Up and Down do not turn.
#[trial]
fn keys_move_the_center_and_turn_the_handle() {
    let mut h = harness();
    let centre = h.centre();
    h.pick(centre);
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.preview().center, Point::new(201.0, 160.0));

    let handle = h.handle();
    h.pick(handle);
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert!(near(h.preview().rotation, 1.0), "{:?}", h.preview());
    h.key(Key::ArrowRight, Modifiers::SHIFT);
    assert!(near(h.preview().rotation, 16.0), "{:?}", h.preview());
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert!(near(h.preview().rotation, 15.0), "{:?}", h.preview());
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert!(near(h.preview().rotation, 15.0), "Up and Down do not turn: {:?}", h.preview());
}

/// Escape during a drag ends the drag only: the symmetry goes back to where the press found it.
#[trial]
fn escape_during_a_drag_cancels_the_drag_not_the_session() {
    let mut h = harness();
    let centre = h.centre();
    h.hold(centre, centre + by(30.0, 0.0));
    h.key(Key::Escape, Modifiers::NONE);
    h.frame(vec![pointer::release(centre + by(30.0, 0.0))]);
    h.frame(Vec::new());
    assert_eq!(h.preview(), start());
}

/// The session opens from a menu, so the canvas has no focus yet: Enter and Escape still reach the tool.
#[trial]
fn enter_and_escape_work_before_the_canvas_is_focused() {
    let session: &'static SymmetryTransform = Box::leak(Box::new(SymmetryTransform::begin(start())));
    let mut canvas = craft_ui::testing::ToolFixture::new("Document", SymmetryTool::new(session, true));
    canvas.fixture().settle();
    canvas.fixture().key(Key::Enter, Modifiers::NONE);
    assert_eq!(canvas.tool_mut().take_events(), [SymmetryEvent::Apply(start())]);
    canvas.fixture().key(Key::Escape, Modifiers::NONE);
    assert_eq!(canvas.tool_mut().take_events(), [SymmetryEvent::Discard]);
}
