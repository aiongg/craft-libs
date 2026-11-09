//! The Shape tools and the Create Shape dialog. The view maps one document unit to one screen point.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CancelReason, CanvasView, EditEvent, PointerButton};
use craft_draw::geometry::shape::ShapeGeometry;
use craft_draw::ui::shape_drag::{CreateShapeDialog, CreateShapeOptions, ShapeDragEvent, ShapeDragOptions, ShapeDragRules, ShapeDragTool, ShapeKind};
use craft_keymap::{Key as KeymapKey, Mods};
use craft_ui::canvas::{CanvasSpace, CanvasTool as _, ToolContext, ToolPointer};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Pos2, Vec2, pos2};

const CANVAS: Vec2 = Vec2::new(400.0, 300.0);
const SLOP: f32 = 6.0;

fn view(zoom: f64) -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(200.0, 150.0), zoom)
}

/// The view of a canvas filling `room`, with its middle at the document's middle.
fn room_view(room: egui::Rect) -> CanvasView {
    CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0)
}

fn cx<'a>(zoom: f64, down: &'a [KeymapKey]) -> ToolContext<'a> {
    ToolContext::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Some(CanvasSpace::new(view(zoom), 1.0))).slop(SLOP).down(down)
}

fn at(x: f32, y: f32) -> ToolPointer {
    ToolPointer::new(pos2(x, y), PointerButton::Primary, 0.0)
}

fn tool(kind: ShapeKind) -> ShapeDragTool {
    let mut tool = ShapeDragTool::new();
    tool.options(ShapeDragOptions::new(kind));
    tool
}

fn committed(events: &[ShapeDragEvent]) -> Option<&ShapeGeometry> {
    events.iter().find_map(|e| match e {
        ShapeDragEvent::Draw(EditEvent::Commit { after, .. }) => Some(after),
        _ => None,
    })
}

fn updated(events: &[ShapeDragEvent]) -> Option<&ShapeGeometry> {
    events.iter().rev().find_map(|e| match e {
        ShapeDragEvent::Draw(EditEvent::Update { value, .. }) => Some(value),
        _ => None,
    })
}

#[trial]
fn a_drag_under_one_pixel_commits_nothing() {
    let mut tool = tool(ShapeKind::Rectangle);
    tool.press(&at(100.0, 100.0), &mut cx(1.0, &[]));
    tool.drag(&at(100.4, 160.0), &mut cx(1.0, &[]));
    tool.release(&at(100.4, 160.0), &mut cx(1.0, &[]));
    let events = tool.take_events();
    assert!(committed(&events).is_none());
    assert!(
        events.iter().any(|e| matches!(e, ShapeDragEvent::Draw(EditEvent::Cancel { reason: CancelReason::Dismissed, .. }))),
        "the gesture is dismissed: {events:?}"
    );
}

#[trial]
fn a_click_inside_the_slop_reports_the_press_in_document_units_at_any_zoom() {
    let mut tool = tool(ShapeKind::Ellipse);
    tool.press(&at(300.0, 150.0), &mut cx(2.0, &[]));
    tool.release(&at(302.0, 150.0), &mut cx(2.0, &[]));
    let events = tool.take_events();
    let [ShapeDragEvent::Click { at, kind, .. }] = events.as_slice() else { panic!("one click in {events:?}") };
    assert_eq!(*kind, ShapeKind::Ellipse);
    assert_eq!(*at, Point::new(250.0, 150.0), "the press, in document units at zoom 2");
}

#[trial]
fn space_moves_the_shape_being_drawn_without_resizing_it() {
    let mut tool = tool(ShapeKind::Rectangle);
    tool.press(&at(100.0, 100.0), &mut cx(1.0, &[]));
    tool.drag(&at(150.0, 140.0), &mut cx(1.0, &[]));
    let held = [KeymapKey::Space];
    tool.drag(&at(150.0, 140.0), &mut cx(1.0, &held));
    tool.drag(&at(160.0, 150.0), &mut cx(1.0, &held));
    tool.release(&at(160.0, 150.0), &mut cx(1.0, &held));
    let events = tool.take_events();
    let Some(ShapeGeometry::Rect { rect, .. }) = committed(&events) else { panic!("a rectangle in {events:?}") };
    assert_eq!(*rect, Rect::new(110.0, 110.0, 160.0, 150.0), "the box keeps its 50 by 40 size and follows the pointer");
}

#[trial]
fn a_star_drag_holds_the_inner_radius_while_control_is_down() {
    let mut tool = ShapeDragTool::new();
    tool.rules(ShapeDragRules::ILLUSTRATOR).options(ShapeDragOptions::new(ShapeKind::Star).star_ratio(0.5));
    tool.press(&at(200.0, 200.0), &mut cx(1.0, &[]));
    tool.drag(&at(200.0, 100.0), &mut cx(1.0, &[]));
    tool.drag(&at(200.0, 50.0).mods(Mods::CTRL), &mut cx(1.0, &[]));
    let events = tool.take_events();
    let Some(ShapeGeometry::RadialPolygon { radius, star_ratio, .. }) = updated(&events) else { panic!("a radial star in {events:?}") };
    assert!((radius - 150.0).abs() < 1e-6, "the outer radius follows the pointer: {radius}");
    assert!((radius * star_ratio - 50.0).abs() < 1e-6, "the inner radius stays at 50: {}", radius * star_ratio);
}

#[trial]
fn the_shape_tool_owes_the_canvas_what_every_tool_owes() {
    ToolFixture::with_view("Shape", tool(ShapeKind::Rectangle), room_view, |_, _| {}).check();
}

fn opened(options: CreateShapeOptions) -> (Fixture, Rc<RefCell<DialogHost>>) {
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().open(CreateShapeDialog::new(options.recalled()));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.settle();
    (fixture, host)
}

fn answered(fixture: &mut Fixture, host: &Rc<RefCell<DialogHost>>) -> Option<craft_draw::ui::shape_drag::CreatedShape> {
    let ok = fixture.by_label("OK")?;
    fixture.click(&ok);
    match host.borrow_mut().take::<CreateShapeDialog>() {
        Some(DialogResult::Ok(shape)) => Some(shape),
        _ => None,
    }
}

#[trial]
fn a_size_out_of_range_shows_a_notice_and_disables_ok() {
    let mut options = CreateShapeOptions::initial(ShapeKind::Rectangle, Point::ZERO);
    options.size = (0.0, 100.0);
    let (mut fixture, host) = opened(options);
    assert!(fixture.texts().iter().any(|run| run.text.contains("must be between 1 and")), "the notice says why");
    assert!(answered(&mut fixture, &host).is_none(), "OK is disabled while a side is out of range");
}
