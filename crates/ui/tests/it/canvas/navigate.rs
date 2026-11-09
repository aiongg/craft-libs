//! Navigation rules (wheel, page keys, flick, autoscroll), the zoom motion and the momentary tool order.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CancelReason, CanvasView, PixelSize, Wheel, WheelPhase, WheelUnit};
use craft_keymap::Mods;
use craft_ui::canvas::{
    Canvas, CanvasTool, FlickPan, Momentary, MomentaryCommands, MomentaryTool, NavigationOptions, PixelRect, RasterSource, ToolAction, ToolContext,
    ToolHost, ToolPath, ToolPointer, TouchpadGlide, WheelAction, WheelInput, WheelNavigation, ZoomAnimation, ZoomDrag,
};
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::{FullWindowCanvas, FullWindowDialog, FullWindowView};
use craft_ui::primitives::{Draw, Revision};
use craft_ui::runtime::commands::FirePhase;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::focus::{egui_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::{Fixture, pointer, trial};
use craft_ui::value_drag::Slider;
use egui::{Color32, ColorImage, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Points of one wheel notch.
const NOTCH: f32 = 40.0;
const CANVAS: Vec2 = Vec2::new(800.0, 600.0);
const DOCUMENT: Size = Size::new(2000.0, 1500.0);

fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(1000.0, 750.0), 1.0)
}

fn input(scroll: Vec2, mods: Mods) -> WheelInput {
    WheelInput::of(&Wheel::new(scroll, mods, WheelUnit::Point, WheelPhase::Move))
}

#[trial]
fn the_wheel_scrolls_and_alt_wheel_zooms_unless_the_choice_swaps_them() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let scroll = Vec2::new(0.0, NOTCH);
    assert_eq!(wheel.classify(input(scroll, Mods::NONE), &options), Some(WheelAction::Pan(scroll)));
    let zoom = wheel.classify(input(scroll, Mods::ALT), &options);
    assert!(matches!(zoom, Some(WheelAction::Zoom(factor)) if (factor - WheelNavigation::NOTCH).abs() < 1e-9), "an Alt notch zooms: {zoom:?}");

    let mut swapped = NavigationOptions::default();
    swapped.zoom_with_wheel = true;
    assert!(matches!(wheel.classify(input(scroll, Mods::NONE), &swapped), Some(WheelAction::Zoom(_))));
    assert_eq!(wheel.classify(input(scroll, Mods::ALT), &swapped), Some(WheelAction::Pan(scroll)));
}

#[trial]
fn shift_wheel_scrolls_sideways_and_shift_alt_wheel_steps_the_ladder() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let scroll = Vec2::new(0.0, NOTCH);
    assert_eq!(wheel.classify(input(scroll, Mods::SHIFT), &options), Some(WheelAction::Pan(Vec2::new(NOTCH, 0.0))));
    let both = Mods { alt: true, shift: true, ..Mods::NONE };
    assert_eq!(wheel.classify(input(scroll, both), &options), Some(WheelAction::SnapZoom(1)));
}

#[trial]
fn a_part_of_a_ladder_notch_is_kept_until_it_adds_up() {
    let options = NavigationOptions::default();
    let mut wheel = WheelNavigation::default();
    let both = Mods { alt: true, shift: true, ..Mods::NONE };
    let quarter = Vec2::new(0.0, NOTCH / 4.0);
    let steps: Vec<_> = (0..4).map(|_| wheel.classify(input(quarter, both), &options)).collect();
    assert_eq!(steps, vec![None, None, None, Some(WheelAction::SnapZoom(1))]);
}

#[trial]
fn a_wheel_zoom_keeps_the_point_under_the_pointer_and_stops_at_the_limit() {
    let mut zoomed = view();
    let at = Pos2::new(600.0, 200.0);
    let doc = zoomed.to_doc(at);
    assert!(WheelNavigation::apply(&mut zoomed, WheelAction::Zoom(2.0), at, DOCUMENT, 1.0));
    let after = zoomed.to_doc(at);
    assert!((after.x - doc.x).abs() < 1e-6 && (after.y - doc.y).abs() < 1e-6, "the point under the pointer stays: {doc:?} -> {after:?}");

    let mut limit = view();
    limit.zoom = craft_behavior::ZoomLadder::MAX;
    assert!(!WheelNavigation::apply(&mut limit, WheelAction::Zoom(2.0), at, DOCUMENT, 1.0), "past the last rung the view stays put");
}

#[trial]
fn a_fast_hand_release_glides_and_slows_to_a_stop_and_a_slow_one_does_not() {
    const STEP: f64 = 0.016;
    let mut flick = FlickPan::default();
    let mut now = 0.0;
    for _ in 0..4 {
        now += STEP;
        flick.drag(now, Vec2::new(20.0, 0.0));
    }
    flick.release(now, true);
    assert!(flick.gliding(), "a release at speed glides");
    let mut travelled = 0.0;
    let mut passes = 0;
    while let Some(step) = flick.advance(now + STEP * f64::from(passes + 1)) {
        travelled += step.x;
        passes += 1;
        assert!(passes < 1000, "the glide stops");
    }
    assert!(travelled > 0.0 && !flick.gliding());

    let mut slow = FlickPan::default();
    now = 0.0;
    for _ in 0..4 {
        now += STEP;
        slow.drag(now, Vec2::new(0.1, 0.0));
    }
    slow.release(now, true);
    assert!(!slow.gliding(), "a slow release does not glide");

    let mut off = FlickPan::default();
    now = 0.0;
    for _ in 0..4 {
        now += STEP;
        off.drag(now, Vec2::new(20.0, 0.0));
    }
    off.release(now, false);
    assert!(!off.gliding(), "Flick Panning off never glides");
}

#[trial]
fn the_zoom_ease_runs_from_the_old_view_to_the_new_one() {
    let from = view();
    let mut target = from;
    target.zoom = 2.0;
    let mut animation = ZoomAnimation::default();
    animation.begin(&from, None, animation.target());
    assert!(animation.running());
    let start = animation.shown(&target, animation.target() - 1.0);
    assert_eq!(start.zoom, from.zoom, "the ease starts at the view drawn last");
    let middle = animation.shown(&target, animation.target() - 0.5);
    assert!(middle.zoom > from.zoom && middle.zoom < target.zoom);
    let end = animation.shown(&target, animation.target());
    assert_eq!(end.zoom, target.zoom);
    assert!(!animation.running(), "the ease ends at the target");
}

#[trial]
fn a_scrubby_drag_zooms_about_the_press_and_a_rectangle_zooms_to_fit() {
    let anchor = Pos2::new(400.0, 300.0);
    let mut scrubbed = view();
    let doc = scrubbed.to_doc(anchor);
    let mut scrub = ZoomDrag::begin(&scrubbed, anchor, true, 1.0);
    scrub.drag(&mut scrubbed, anchor + Vec2::new(ZoomDrag::DOUBLING, 0.0), DOCUMENT, 1.0);
    assert!((scrubbed.zoom - 2.0).abs() < 1e-9, "a drag of one doubling distance doubles the zoom");
    let after = scrubbed.to_doc(anchor);
    assert!((after.x - doc.x).abs() < 1e-6 && (after.y - doc.y).abs() < 1e-6);

    let mut ruled = view();
    let mut rectangle = ZoomDrag::begin(&ruled, anchor, false, 1.0);
    rectangle.drag(&mut ruled, anchor + Vec2::new(200.0, 150.0), DOCUMENT, 1.0);
    rectangle.release(&mut ruled, false, DOCUMENT, 1.0, 6.0);
    assert!(ruled.zoom > 1.0, "a rectangle zooms to fit it");

    let mut sliver = view();
    let mut thin = ZoomDrag::begin(&sliver, anchor, false, 1.0);
    thin.drag(&mut sliver, anchor + Vec2::new(200.0, 2.0), DOCUMENT, 1.0);
    thin.release(&mut sliver, false, DOCUMENT, 1.0, 6.0);
    assert_eq!(sliver.zoom, 1.0, "a rectangle too small to zoom to leaves the view");
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

#[trial]
fn holding_space_during_a_brush_pans_and_letting_go_returns_to_the_brush_without_losing_a_gesture() {
    let mut canvas = held(|_| {});
    canvas.focus(canvas.canvas.get());
    let from = canvas.area.get().center();
    let start = canvas.view.get().expect("the canvas showed a view").center;

    canvas.fixture.frame(vec![key_press(Key::Space, Modifiers::NONE)]);
    drag(&mut canvas.fixture, from, Vec2::new(60.0, 40.0));
    assert_eq!(canvas.brush.borrow().presses, 0, "the brush gets no press while Space is held");
    assert_ne!(canvas.view.get().map(|view| view.center), Some(start), "the Hand pans the view, and nothing but the keys fed it");

    canvas.fixture.frame(vec![key_release(Key::Space, Modifiers::NONE)]);
    drag(&mut canvas.fixture, from, Vec2::new(60.0, 40.0));
    let brush = &canvas.brush;
    assert_eq!((brush.borrow().presses, brush.borrow().releases), (1, 1), "the brush is back once Space is up");

    // Space pressed in the middle of a brush stroke does not take the stroke from the brush.
    canvas.fixture.frame(vec![pointer::moved(from)]);
    canvas.fixture.frame(vec![pointer::press(from)]);
    canvas.fixture.frame(vec![key_press(Key::Space, Modifiers::NONE)]);
    canvas.fixture.frame(vec![pointer::moved(from + Vec2::new(30.0, 0.0))]);
    canvas.fixture.frame(vec![pointer::moved(from + Vec2::new(60.0, 0.0))]);
    canvas.fixture.frame(vec![pointer::release(from + Vec2::new(60.0, 0.0))]);
    assert_eq!((brush.borrow().presses, brush.borrow().releases), (2, 2), "the stroke ends with the brush");
}

#[trial]
fn space_with_the_focus_in_a_slider_beside_the_canvas_pans_the_canvas() {
    let slider = Rc::new(Cell::new(Id::NULL));
    let (id, mut level) = (Rc::clone(&slider), 40.0);
    let mut canvas = held(move |ui| id.set(Slider::new(Bind::bound(&mut level), 0.0..=100.0).label("Opacity").show(ui).response.id));
    canvas.focus(slider.get());
    assert_eq!(egui_focused(canvas.fixture.ctx()), Some(slider.get()), "the slider holds the focus");
    let from = canvas.area.get().center();
    let start = canvas.view.get().expect("the canvas showed a view").center;

    canvas.fixture.frame(vec![key_press(Key::Space, Modifiers::NONE)]);
    assert_eq!(canvas.momentary.borrow_mut().tool(Mods::NONE, false), Some(MomentaryTool::Hand), "the canvas's host took the Hand press");
    drag(&mut canvas.fixture, from, Vec2::new(60.0, 40.0));
    assert_eq!(canvas.brush.borrow().presses, 0, "the canvas beside the focus took Space");
    assert_ne!(canvas.view.get().map(|view| view.center), Some(start), "the Hand pans the canvas");
}

/// An image of one grey, larger than a full-window dialog's canvas.
struct Grey;

impl RasterSource for Grey {
    fn size(&self) -> PixelSize {
        PixelSize::new(1600, 1200)
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_rgb(0x80, 0x80, 0x80));
    }
}

/// A full-window dialog's canvas is no `canvasView` instance: its host takes Space from the dialog around it.
#[trial]
fn space_in_a_full_window_dialog_pans_its_canvas() {
    let view = Rc::new(RefCell::new(FullWindowView::new()));
    let brush = Rc::new(RefCell::new(Brush::default()));
    let momentary = Rc::new(RefCell::new(Momentary::new(NavigationOptions::default())));
    let (shown, tool, keys) = (Rc::clone(&view), Rc::clone(&brush), Rc::clone(&momentary));
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        FullWindowDialog::new("Liquify").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            let (mut tool, mut keys) = (tool.borrow_mut(), keys.borrow_mut());
            FullWindowCanvas::new(Bind::bound(&mut shown.borrow_mut()), &Grey).tool(&mut *tool).momentary(&mut keys).show(ui);
        });
    });
    fixture.settle();
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the dialog shows its canvas");
    fixture.click(&canvas).settle();
    let (presses, start) = (brush.borrow().presses, view.borrow().view().center);

    fixture.frame(vec![key_press(Key::Space, Modifiers::NONE)]);
    drag(&mut fixture, canvas.center(), Vec2::new(60.0, 40.0));
    assert_eq!(brush.borrow().presses, presses, "the brush gets no press while Space is held");
    assert_ne!(view.borrow().view().center, start, "the Hand pans the dialog's view");
}
