//! The Connector tool, its handles and its bar. The view maps one document unit to one screen point.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CancelReason, CanvasView, Cursor, CursorIcon, EditEvent, Mixed, PointerButton};
use craft_draw::geometry::connector::{ConnectionSide, ConnectorPath, RouteEnd, Routing, route_along};
use craft_draw::ui::connector::{
    ConnectionSite, ConnectionTarget, ConnectorBar, ConnectorDraft, ConnectorEnd, ConnectorEndKind, ConnectorEvent, ConnectorHandles, ConnectorItem,
    ConnectorLine, ConnectorTool,
};
use craft_draw::ui::stroke::{StrokeChoice, StrokeChoiceRow};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::{Canvas, CanvasSpace, CanvasTool as _, ToolContext, ToolPointer};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Key, Modifiers, Pos2, Vec2, pos2, vec2};

const CANVAS: Vec2 = Vec2::new(400.0, 300.0);
const SLOP: f32 = 6.0;
/// How far the bend test drags the middle run, in document units.
const BEND_DRAG: f64 = 20.0;
const TOLERANCE: f64 = 1e-6;

const A: Rect = Rect::new(20.0, 100.0, 100.0, 200.0);
const E: Rect = Rect::new(300.0, 20.0, 380.0, 80.0);

fn target(id: u64, rect: Rect) -> ConnectionTarget {
    ConnectionTarget::new(id, rect)
}

/// Three boxes: A (1) on the left, B (2) on the right at the same height, C (3) above A's right.
fn boxes() -> Vec<ConnectionTarget> {
    vec![target(1, A), target(2, Rect::new(250.0, 100.0, 330.0, 200.0)), target(3, Rect::new(150.0, 20.0, 200.0, 60.0))]
}

fn glued(target: u64, side: ConnectionSide) -> ConnectorEnd {
    ConnectorEnd::glued(target, side)
}

fn item(from: ConnectorEnd, to: ConnectorEnd, routing: Routing) -> ConnectorItem {
    ConnectorItem::new(9, from, to, routing)
}

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(200.0, 150.0), 1.0)
}

/// The view of a canvas filling `room`, with its middle at the document's middle.
fn room_view(room: egui::Rect) -> CanvasView {
    CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0)
}

fn cx<'a>() -> ToolContext<'a> {
    ToolContext::new(egui::Rect::from_min_size(Pos2::ZERO, CANVAS), Some(CanvasSpace::new(view(), 1.0))).slop(SLOP)
}

fn at(x: f32, y: f32) -> ToolPointer {
    ToolPointer::new(pos2(x, y), PointerButton::Primary, 0.0)
}

fn tool(targets: Vec<ConnectionTarget>) -> ConnectorTool {
    let mut tool = ConnectorTool::new();
    tool.targets(&targets);
    tool
}

/// A press, a move through the middle, a move to `b` and the release.
fn drag(tool: &mut ConnectorTool, a: (f32, f32), b: (f32, f32)) -> Vec<ConnectorEvent> {
    tool.press(&at(a.0, a.1), &mut cx());
    tool.drag(&at((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), &mut cx());
    tool.drag(&at(b.0, b.1), &mut cx());
    tool.release(&at(b.0, b.1), &mut cx());
    tool.take_events()
}

fn commit(events: &[ConnectorEvent]) -> Option<&ConnectorDraft> {
    events.iter().find_map(|e| match e {
        ConnectorEvent::Draw(EditEvent::Commit { after, .. }) => Some(after),
        _ => None,
    })
}

fn cancel_reason(events: &[ConnectorEvent]) -> Option<CancelReason> {
    events.iter().find_map(|e| match e {
        ConnectorEvent::Draw(EditEvent::Cancel { reason, .. }) => Some(*reason),
        _ => None,
    })
}

#[trial]
fn hovering_a_target_points_at_the_site_an_end_would_glue_to() {
    let mut tool = tool(boxes());
    tool.hover(&at(97.0, 150.0), &mut cx());
    assert_eq!(tool.cursor(&cx()), Cursor::System(CursorIcon::PointingHand), "over a side middle");
    tool.hover(&at(60.0, 150.0), &mut cx());
    assert_eq!(tool.cursor(&cx()), Cursor::System(CursorIcon::Crosshair), "the middle of the box chooses no site");
    tool.hover(&at(225.0, 250.0), &mut cx());
    assert_eq!(tool.cursor(&cx()), Cursor::System(CursorIcon::Crosshair), "over nothing");
}

#[trial]
fn a_drag_under_the_slop_or_onto_its_start_makes_nothing() {
    let mut tool = tool(boxes());
    assert!(drag(&mut tool, (60.0, 150.0), (60.0 + SLOP - 1.0, 150.0)).is_empty(), "a drag under the slop reports nothing");

    let events = drag(&mut tool, (60.0, 150.0), (50.0, 140.0));
    assert!(matches!(events.first(), Some(ConnectorEvent::Draw(EditEvent::Begin { .. }))));
    assert_eq!(cancel_reason(&events), Some(CancelReason::Dismissed), "ending on the start object makes nothing");
    assert!(commit(&events).is_none());

    let events = drag(&mut tool, (60.0, 150.0), (255.0, 150.0));
    let draft = commit(&events).expect("a commit");
    assert_eq!(draft.from, glued(1, ConnectionSide::Auto));
    assert_eq!(draft.to, glued(2, ConnectionSide::Left));
    assert!(draft.points.len() >= 2);
}

#[trial]
fn sites_accept_side_middles_and_numbered_sites() {
    let numbered = ConnectionTarget::new(4, Rect::new(150.0, 200.0, 250.0, 260.0)).sites(vec![
        ConnectionSite::new(Point::new(160.0, 210.0), ConnectionSide::Top, 7),
        ConnectionSite::new(Point::new(240.0, 250.0), ConnectionSide::Bottom, 9),
    ]);
    let mut targets = boxes();
    targets.push(numbered);
    let mut tool = tool(targets);

    let events = drag(&mut tool, (60.0, 150.0), (255.0, 150.0));
    assert_eq!(commit(&events).expect("a commit").to, glued(2, ConnectionSide::Left), "a side middle");

    let events = drag(&mut tool, (60.0, 150.0), (238.0, 248.0));
    assert_eq!(commit(&events).expect("a commit").to, ConnectorEnd::glued(4, ConnectionSide::Bottom).site(9));

    let events = drag(&mut tool, (60.0, 150.0), (200.0, 230.0));
    assert_eq!(commit(&events).expect("a commit").to, ConnectorEnd::Free(Point::new(200.0, 230.0)), "out of reach of every site: free");
}

#[trial]
fn a_release_the_geometry_refuses_cancels_as_refused() {
    let mut tool = tool(boxes());
    tool.press(&at(60.0, 250.0), &mut cx());
    tool.drag(&at(200.0, 250.0), &mut cx());
    tool.take_events();
    tool.release(&at(f32::NAN, f32::NAN), &mut cx());
    assert_eq!(cancel_reason(&tool.take_events()), Some(CancelReason::Refused));
}

#[trial]
fn the_connector_tool_owes_the_canvas_what_every_tool_owes() {
    ToolFixture::with_view("Connector", tool(boxes()), room_view, |_, _| {}).check();
}

/// What the app holds for the handles: the connector and the last operation reported.
#[derive(Default)]
struct App {
    item: Option<ConnectorItem>,
    events: Vec<ConnectorEvent>,
}

/// A canvas showing the handles of `item` over `targets`; the app applies what the handles edit.
fn handles(item: ConnectorItem, targets: Vec<ConnectionTarget>) -> (Fixture, Rc<RefCell<App>>) {
    let app = Rc::new(RefCell::new(App { item: Some(item), events: Vec::new() }));
    let shown = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        let mut held = shown.borrow().item.clone();
        let Some(item) = held.as_mut() else { return };
        let out = Canvas::new("Document")
            .view(view)
            .show(ui, |ui, body| ConnectorHandles::new("Connector", Bind::bound(&mut *item), &targets).show_at(ui, body.rect));
        let mut app = shown.borrow_mut();
        app.item = held;
        if let Some(event) = out.inner.and_then(|out| out.event) {
            app.events.push(event);
        }
    });
    fixture.room(CANVAS).settle();
    (fixture, app)
}

fn straight() -> ConnectorItem {
    item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight)
}

#[trial]
fn an_end_dragged_to_a_target_or_a_free_point_reattaches() {
    let (mut fixture, app) = handles(straight(), boxes());
    let end = fixture.by_label("End, attached to 2").expect("the end handle");
    fixture.drag(&end, vec2(60.0, -110.0));
    let held = app.borrow().item.clone().expect("a connector");
    assert!(matches!(held.to, ConnectorEnd::Free(_)), "dropped on empty canvas: free, got {:?}", held.to);
    assert!(app.borrow().events.iter().any(|e| matches!(e, ConnectorEvent::Reattach { end: ConnectorEndKind::End, .. })));

    let (mut fixture, app) = handles(straight(), boxes());
    let start = fixture.by_label("Start, attached to 1").expect("the start handle");
    fixture.drag(&start, vec2(75.0, -94.0));
    let held = app.borrow().item.clone().expect("a connector");
    assert_eq!(held.from, glued(3, ConnectionSide::Bottom), "dropped on the box above: glued at its bottom middle");

    let (mut fixture, app) = handles(straight(), boxes());
    let start = fixture.by_label("Start, attached to 1").expect("the start handle");
    fixture.drag(&start, vec2(190.0, 0.0));
    let held = app.borrow().item.clone().expect("a connector");
    assert_eq!(held.from, glued(1, ConnectionSide::Right), "dropped on the object the other end is on: unchanged");
}

fn middle_x(path: Option<&ConnectorPath>) -> f64 {
    let from = RouteEnd::Box(A, ConnectionSide::Right);
    let to = RouteEnd::Box(E, ConnectionSide::Left);
    route_along(from, to, Routing::Elbow, path).expect("a route")[1].x
}

#[trial]
fn a_bend_dragged_reshapes_the_elbow_from_the_path_the_drag_began_with() {
    let mut targets = boxes();
    targets.push(target(5, E));
    let elbow = item(glued(1, ConnectionSide::Right), glued(5, ConnectionSide::Left), Routing::Elbow);
    let (mut fixture, app) = handles(elbow, targets);
    let before = middle_x(None);
    let bend = fixture.by_label("Bend 1 of 1").expect("the middle run's handle");
    fixture.drag(&bend, vec2(-BEND_DRAG as f32, 0.0));
    let held = app.borrow().item.clone().expect("a connector");
    let path = held.path.expect("a reshaped path");
    assert!(matches!(path, ConnectorPath::Runs { .. }));
    assert!((before - middle_x(Some(&path)) - BEND_DRAG).abs() < TOLERANCE, "the middle run follows the drag");
}

#[trial]
fn the_arrows_move_a_picked_end_to_the_next_target_and_the_pick_follows_the_connector() {
    let (mut fixture, app) = handles(straight(), boxes());
    let end = fixture.by_label("End, attached to 2").expect("the end handle");
    fixture.click(&end);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    let held = app.borrow().item.clone().expect("a connector");
    assert_eq!(glued_to(&held.to), Some(3), "the next target that is not the start's object");
}

fn glued_to(end: &ConnectorEnd) -> Option<u64> {
    match end {
        ConnectorEnd::Glued { target, .. } => Some(*target),
        ConnectorEnd::Free(_) => None,
        _ => None,
    }
}

#[trial]
fn an_end_part_is_named_by_its_target_and_a_free_end_says_so() {
    let named = vec![ConnectionTarget::new(1, A).named("Rectangle 1"), target(2, Rect::new(250.0, 100.0, 330.0, 200.0))];
    let free = item(glued(1, ConnectionSide::Right), ConnectorEnd::Free(Point::new(300.0, 250.0)), Routing::Straight);
    let (fixture, _) = handles(free, named);
    assert!(fixture.by_label("Start, attached to Rectangle 1").is_some());
    assert!(fixture.by_label("End, free").is_some());

    let (fixture, _) = handles(straight(), boxes());
    assert!(fixture.by_label("End, attached to 2").is_some(), "a target without a name is read by its number");
}

#[trial]
fn the_bar_reports_routing_caps_and_line_style() {
    let choices = vec![StrokeChoice::new("none", "None"), StrokeChoice::new("arrow", "Arrow"), StrokeChoice::new("dot", "Dot")];
    let seen: Rc<RefCell<Vec<(Option<Routing>, Option<String>, Option<ConnectorLine>)>>> = Rc::default();
    let log = Rc::clone(&seen);
    let state = Rc::new(RefCell::new((Routing::Straight, ConnectorLine::default())));
    let held = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let (routing, line) = *held.borrow();
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            let start = StrokeChoiceRow::new("start", "Start", &choices).selected(Mixed::Same("none"));
            let end = StrokeChoiceRow::new("end", "End", &choices).selected(Mixed::Same("arrow"));
            let out = ConnectorBar::new(Mixed::Same(routing), Mixed::Same(line)).caps(start, end).show(ui);
            if let Some(picked) = out.routing {
                held.borrow_mut().0 = picked;
            }
            if let Some(picked) = out.line {
                held.borrow_mut().1 = picked;
            }
            if out.routing.is_some() || out.end_cap.is_some() || out.line.is_some() {
                log.borrow_mut().push((out.routing, out.end_cap, out.line));
            }
        });
    });
    fixture.room(vec2(600.0, 160.0)).settle();
    let elbow = fixture.by_label("Elbow").expect("the Elbow button");
    fixture.click(&elbow);
    for (menu, option) in [("End point", "Dot"), ("Stroke style", "Dash"), ("Stroke style", "Stroke weight 4")] {
        let menu = fixture.by_label(menu).expect("the menu");
        fixture.click(&menu);
        let option = fixture.by_label(option).expect("the option");
        fixture.click(&option);
    }
    let seen = seen.borrow();
    assert_eq!(seen.len(), 4);
    assert_eq!(seen[0].0, Some(Routing::Elbow));
    assert_eq!(seen[1].1.as_deref(), Some("dot"));
    assert_eq!(seen[2].2, Some(ConnectorLine::new(true, ConnectorLine::default().weight)));
    assert_eq!(seen[3].2, Some(ConnectorLine::new(true, ConnectorLine::WEIGHTS[2])));
}
