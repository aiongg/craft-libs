//! Navigation rules (wheel, page keys, flick, autoscroll), the zoom motion and the momentary tool order.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CancelReason, CanvasView, Wheel, WheelPhase, WheelUnit};
use craft_keymap::Mods;
use craft_ui::canvas::{
    Canvas, CanvasTool, Momentary, MomentaryCommands, MomentaryTool, NavigationOptions, ToolAction, ToolContext, ToolHost, ToolPath, ToolPointer,
    TouchpadGlide,
};
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::primitives::Draw;
use craft_ui::runtime::commands::FirePhase;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Id, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const CANVAS: Vec2 = Vec2::new(800.0, 600.0);

const DOCUMENT: Size = Size::new(2000.0, 1500.0);

fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(1000.0, 750.0), 1.0)
}

#[trial]
fn a_touchpad_glide_stops_in_the_pass_the_edge_clamp_leaves_the_view_where_it_was() {
    const STEP: f64 = 0.016;
    let mut glide = TouchpadGlide::new(true);
    let mut now = 0.0;
    glide.read(&Wheel::new(Vec2::new(30.0, 0.0), Mods::NONE, WheelUnit::Point, WheelPhase::Start), now, true);
    for _ in 0..4 {
        now += STEP;
        glide.read(&Wheel::new(Vec2::new(30.0, 0.0), Mods::NONE, WheelUnit::Point, WheelPhase::Move), now, true);
    }
    glide.read(&Wheel::new(Vec2::ZERO, Mods::NONE, WheelUnit::Point, WheelPhase::End), now, true);
    now += STEP;
    assert!(glide.step(now).x > 0.0, "a fast lift glides");
    glide.moved(true);
    now += STEP;
    assert!(glide.step(now).x > 0.0, "a glide whose view moved goes on");
    glide.moved(false);
    now += STEP;
    assert_eq!(glide.step(now), Vec2::ZERO, "a pass that left the view where it was ends the glide");
}

#[trial]
fn space_is_the_hand_and_cmd_or_alt_added_in_any_order_switch_to_zoom() {
    let mut momentary = Momentary::new(NavigationOptions::default());
    assert_eq!(momentary.tool(Mods::NONE, false), None);
    momentary.fired(MomentaryCommands::Hand, FirePhase::Press);
    assert_eq!(momentary.tool(Mods::NONE, false), Some(MomentaryTool::Hand));
    assert_eq!(momentary.tool(Mods::CMD, false), Some(MomentaryTool::ZoomIn));
    assert_eq!(momentary.tool(Mods::ALT, false), Some(MomentaryTool::ZoomOut));
    momentary.fired(MomentaryCommands::ZoomIn, FirePhase::Press);
    assert_eq!(momentary.tool(Mods::CMD, false), Some(MomentaryTool::ZoomIn));
    let both = Mods { cmd: true, alt: true, ..Mods::NONE };
    assert_eq!(momentary.tool(both, false), Some(MomentaryTool::ZoomOut), "Alt added to a held Zoom In zooms out");
    assert_eq!(momentary.tool(Mods::NONE, true), Some(MomentaryTool::ZoomOut), "a drag keeps the tool it began with");
    momentary.fired(MomentaryCommands::ZoomIn, FirePhase::Release);
    assert_eq!(momentary.tool(Mods::NONE, false), None);
}

/// A brush: it claims every press and counts what the host called.
#[derive(Default)]
struct Brush {
    presses: u32,
    releases: u32,
}

impl CanvasTool for Brush {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        self.presses += 1;
        ToolAction::Claimed
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.releases += 1;
    }

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Select Area... command")
    }
}

fn drag(fixture: &mut Fixture, from: Pos2, by: Vec2) {
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + by * (step as f32 / 4.0))]);
    }
    fixture.frame(vec![pointer::release(from + by)]);
    fixture.frame(Vec::new());
}

/// A canvas over a view at its middle, with a brush for its tool and the momentary set the host feeds from the
/// commands the keys fire.
struct Held {
    fixture: Fixture,
    view: Rc<Cell<Option<CanvasView>>>,
    canvas: Rc<Cell<Id>>,
    area: Rc<Cell<Rect>>,
    brush: Rc<RefCell<Brush>>,
    momentary: Rc<RefCell<Momentary>>,
}

/// The canvas under the host, with `beside` shown above it in the same room: a control that can hold the key focus
/// without being a canvas.
fn held(mut beside: impl FnMut(&mut Ui) + 'static) -> Held {
    let view = Rc::new(Cell::new(None::<CanvasView>));
    let canvas = Rc::new(Cell::new(Id::NULL));
    let area = Rc::new(Cell::new(Rect::NOTHING));
    let momentary = Rc::new(RefCell::new(Momentary::new(NavigationOptions::default())));
    let brush = Rc::new(RefCell::new(Brush::default()));
    let (seen, id, rect, keys, tool) = (Rc::clone(&view), Rc::clone(&canvas), Rc::clone(&area), Rc::clone(&momentary), Rc::clone(&brush));
    let mut fixture = Fixture::new(move |ui| {
        beside(ui);
        let room = ui.available_rect_before_wrap();
        let mut current = seen.get().unwrap_or_else(|| CanvasView::new(room, Point::new(1000.0, 750.0), 1.0));
        current.rect = room;
        let shown = Canvas::new("Document").view(current).document(DOCUMENT).show(ui, |_, _| {});
        id.set(shown.output.response.id);
        rect.set(shown.output.response.rect);
        let reply = ToolHost::new().momentary(&mut keys.borrow_mut()).run(ui, &shown.output, &mut *tool.borrow_mut());
        seen.set(Some(reply.view.unwrap_or(current)));
    });
    fixture.settle();
    Held { fixture, view, canvas, area, brush, momentary }
}

impl Held {
    fn focus(&mut self, id: Id) {
        request(self.fixture.ctx(), FocusTarget::Id(id));
        self.fixture.frame(Vec::new());
    }
}
