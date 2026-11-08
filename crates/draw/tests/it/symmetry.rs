//! Painting symmetry: the model's geometry, the menu, the handles of Transform Symmetry and the tool that carries its
//! keys.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, EditEvent, Mixed, PartKey};
use craft_draw::ui::symmetry::{
    MAX_COORDINATE, Symmetry, SymmetryChoice, SymmetryCommand, SymmetryEvent, SymmetryHandles, SymmetryMenu, SymmetryMode, SymmetryPathRow,
    SymmetryTool, SymmetryTransform,
};
use craft_keymap::Mods;
use craft_ui::canvas::{Canvas, CanvasSpace, CanvasTool as _, ToolContext, ToolKey, ToolPath};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, pointer, trial};
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

fn sym(mode: SymmetryMode, rotation: f64) -> Symmetry {
    Symmetry::new(mode, Point::new(20.0, 30.0), rotation).expect("a valid symmetry")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// Vertical and Dual start at 90 degrees, Horizontal at 0, Diagonal at 45, plus the rotation clockwise; Dual has two
/// perpendicular axes.
#[trial]
fn axes_start_at_90_0_45_degrees_plus_rotation_and_dual_adds_the_perpendicular() {
    let v = sym(SymmetryMode::Vertical, 0.0).axes();
    assert!(v.len() == 1 && near(v[0].x, 0.0) && near(v[0].y, 1.0));
    let h = sym(SymmetryMode::Horizontal, 0.0).axes();
    assert!(near(h[0].x, 1.0) && near(h[0].y, 0.0));
    let d = sym(SymmetryMode::Diagonal, 0.0).axes();
    assert!(near(d[0].x, d[0].y));
    let turned = sym(SymmetryMode::Horizontal, 90.0).axes();
    assert!(near(turned[0].x, 0.0) && near(turned[0].y, 1.0));
    let dual = sym(SymmetryMode::Dual, 0.0).axes();
    assert_eq!(dual.len(), 2);
    assert!(near(dual[0].dot(dual[1]), 0.0));
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

/// `Symmetry::new` refuses NaN, infinity and a centre past the limit, and wraps 370 to 10 and -90 to 270.
#[trial]
fn new_rejects_non_finite_and_wraps_rotation() {
    let o = Point::ORIGIN;
    assert!(Symmetry::new(SymmetryMode::Dual, Point::new(f64::NAN, 0.0), 0.0).is_none());
    assert!(Symmetry::new(SymmetryMode::Dual, o, f64::INFINITY).is_none());
    assert!(Symmetry::new(SymmetryMode::Dual, Point::new(0.0, MAX_COORDINATE + 1.0), 0.0).is_none());
    assert!(near(sym(SymmetryMode::Dual, 370.0).rotation, 10.0));
    assert!(near(sym(SymmetryMode::Dual, -90.0).rotation, 270.0));
}

/// The rotation handle sits at a share of the shorter side along the first axis, at least 1 unit from the centre.
#[trial]
fn the_rotation_handle_sits_at_a_share_of_the_shorter_side() {
    let s = sym(SymmetryMode::Horizontal, 0.0);
    let h = s.rotation_handle(Size::new(200.0, 100.0), 0.35);
    assert!(near(h.x, 55.0) && near(h.y, 30.0));
    let tiny = s.rotation_handle(Size::new(2.0, 2.0), 0.35);
    assert!(near(tiny.x, 21.0));
}

/// What the menu host keeps.
struct MenuHost {
    choice: SymmetryChoice,
    can_transform: bool,
    commands: Vec<SymmetryCommand>,
}

fn menu(choice: SymmetryChoice, can_transform: bool) -> (Fixture, Rc<RefCell<MenuHost>>) {
    let host = Rc::new(RefCell::new(MenuHost { choice, can_transform, commands: Vec::new() }));
    let shown = Rc::clone(&host);
    let paths = vec![SymmetryPathRow::new("work", "Work Path")];
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let output = SymmetryMenu::new("Symmetry", &host.choice).paths(&paths).can_transform(host.can_transform).show(ui);
        host.commands.extend(output.command);
    });
    fixture.settle();
    (fixture, host)
}

fn open(fixture: &mut Fixture) {
    let button = fixture.by_label("Set painting symmetry options").expect("the menu button is shown");
    fixture.click(&button);
    fixture.idle(3);
}

fn pick(fixture: &mut Fixture, row: &str) {
    let target: Target = fixture.by_label(row).unwrap_or_else(|| panic!("the menu has a row {row}"));
    fixture.click(&target);
    fixture.idle(3);
}

/// A row runs its command and closes the menu; Symmetry Off is disabled while off and Transform Symmetry without
/// `can_transform`.
#[trial]
fn rows_check_the_current_choice_and_run_commands() {
    let (mut fixture, host) = menu(SymmetryChoice::Mode(SymmetryMode::Dual), false);
    open(&mut fixture);
    let checked = |fixture: &Fixture, row: &str| fixture.by_label(row).and_then(|target| fixture.tree().toggled_of(target.node));
    assert_eq!(checked(&fixture, "Dual Axis"), Some(Mixed::Same(true)));
    assert_eq!(checked(&fixture, "Vertical"), Some(Mixed::Same(false)));
    pick(&mut fixture, "Transform Symmetry");
    assert!(host.borrow().commands.is_empty(), "no transform without can_transform");
    pick(&mut fixture, "Horizontal");
    assert_eq!(host.borrow().commands, [SymmetryCommand::Mode(SymmetryMode::Horizontal)]);
    assert!(fixture.by_label("Horizontal").is_none(), "the menu closed");

    let (mut fixture, host) = menu(SymmetryChoice::Off, false);
    open(&mut fixture);
    pick(&mut fixture, "Symmetry Off");
    assert!(host.borrow().commands.is_empty(), "Symmetry Off is disabled while off");
    pick(&mut fixture, "Work Path");
    assert_eq!(host.borrow().commands, [SymmetryCommand::FromPath("work".to_owned())]);

    let (mut fixture, host) = menu(SymmetryChoice::Path("work".to_owned()), true);
    open(&mut fixture);
    pick(&mut fixture, "Transform Symmetry");
    assert_eq!(host.borrow().commands, [SymmetryCommand::Transform]);
}

/// The button is pressed and describes the mode while a symmetry is on, and is neither while it is off.
#[trial]
fn the_button_is_pressed_while_a_symmetry_is_on() {
    let (mut fixture, _) = menu(SymmetryChoice::Mode(SymmetryMode::Diagonal), false);
    let button = fixture.by_label("Set painting symmetry options").expect("the button is shown");
    let tree = fixture.tree();
    assert_eq!(tree.selected_of(button.node), Some(true));
    assert_eq!(tree.description_of(button.node), Some("Diagonal"));

    let (fixture, _) = menu(SymmetryChoice::Off, false);
    let button = fixture.by_label("Set painting symmetry options").expect("the button is shown");
    let tree = fixture.tree();
    assert_ne!(tree.selected_of(button.node), Some(true));
    assert_eq!(tree.description_of(button.node), None);
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
            if let Some(event) = output.edit.event {
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

fn tool_context() -> ToolContext<'static> {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, by(400.0, 300.0));
    let space = CanvasSpace::new(CanvasView::new(rect, Point::new(200.0, 150.0), 1.0), 1.0);
    ToolContext::new(rect, Some(space))
}

/// Enter gives Apply of the preview, Escape gives Discard, an invalid session gives Discard in its first pass and
/// takes no key.
#[trial]
fn enter_applies_escape_discards_invalid_discards() {
    let session = SymmetryTransform::begin(start());
    let mut tool = SymmetryTool::new(&session, true);
    let mut cx = tool_context();
    assert!(tool.key(&ToolKey::Submit, &mut cx));
    assert_eq!(tool.take_events(), [SymmetryEvent::Apply(start())]);
    assert!(tool.key(&ToolKey::Cancel, &mut cx));
    assert_eq!(tool.take_events(), [SymmetryEvent::Discard]);
    assert!(!tool.key(&ToolKey::Kept(craft_keymap::Chord::plain(craft_keymap::Key::F1)), &mut cx), "other keys stay the app's");

    let mut invalid = SymmetryTool::new(&session, false);
    invalid.each_pass(&mut cx);
    assert_eq!(invalid.take_events(), [SymmetryEvent::Discard]);
    assert!(!invalid.key(&ToolKey::Submit, &mut cx));
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

/// With the canvas focused, the arrows ask the tool to move the symmetry's centre 1 document unit, Shift 10, each as a
/// `Set` of the preview.
#[trial]
fn the_tool_nudges_the_centre_with_the_arrows() {
    let session: &'static SymmetryTransform = Box::leak(Box::new(SymmetryTransform::begin(start())));
    let mut canvas = craft_ui::testing::ToolFixture::with_view(
        "Document",
        SymmetryTool::new(session, true),
        |room| CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0),
        |_, _| {},
    );
    canvas.fixture().key(Key::Tab, Modifiers::NONE).settle();
    canvas.fixture().key(Key::ArrowRight, Modifiers::NONE).settle();
    canvas.fixture().key(Key::ArrowDown, Modifiers::SHIFT).settle();
    let centres: Vec<Point> = canvas
        .tool_mut()
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SymmetryEvent::Preview(EditEvent::Set { after, .. }) => Some(after.center),
            _ => None,
        })
        .collect();
    assert_eq!(centres, [Point::new(201.0, 150.0), Point::new(200.0, 160.0)]);
}
