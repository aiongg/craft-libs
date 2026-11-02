//! Tests of the Connector tool (task SH-T10). The view maps one document unit to one screen point.

use egui::{CursorIcon, Key, Modifiers, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::{Point, Rect};

use super::{ConnectionSite, ConnectionTarget, ConnectorDraft, ConnectorEnd, ConnectorEndKind, ConnectorEvent, ConnectorItem, ConnectorTool};
use crate::geometry::connector::{ConnectionSide, ConnectorPath, RouteEnd, Routing, route_along};
use craft_ui::canvas::{CanvasId, CanvasTool as _, CanvasView, ToolHost, ToolInput};
use craft_ui::edit::CancelReason;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, current, install};

/// The app: the targets, the selected connector and what the tool reported. A committed reshape is
/// applied to the selected connector the way an app would.
struct Host {
    tool: ConnectorTool,
    targets: Vec<ConnectionTarget>,
    selected: Option<ConnectorItem>,
    view: CanvasView,
    canvas: Option<CanvasId>,
    cursor: CursorIcon,
    events: Vec<ConnectorEvent>,
    hovered: Option<(u64, Option<u32>)>,
}

fn rect(r: (f64, f64, f64, f64)) -> Rect {
    Rect::new(r.0, r.1, r.2, r.3)
}

fn target(id: u64, r: (f64, f64, f64, f64)) -> ConnectionTarget {
    ConnectionTarget::new(id, rect(r))
}

const A: (f64, f64, f64, f64) = (20.0, 100.0, 100.0, 200.0);
const E: (f64, f64, f64, f64) = (300.0, 20.0, 380.0, 80.0);

/// Three boxes: A (1) on the left, B (2) on the right at the same height, C (3) above A's right.
fn boxes() -> Vec<ConnectionTarget> {
    vec![target(1, A), target(2, (250.0, 100.0, 330.0, 200.0)), target(3, (150.0, 20.0, 200.0, 60.0))]
}

fn glued(target: u64, side: ConnectionSide) -> ConnectorEnd {
    ConnectorEnd::Glued { target, side, site: None, at: None }
}

fn item(from: ConnectorEnd, to: ConnectorEnd, routing: Routing) -> ConnectorItem {
    ConnectorItem { id: 9, from, to, routing, path: None }
}

fn harness(targets: Vec<ConnectionTarget>, selected: Option<ConnectorItem>) -> Harness<'static, Host> {
    let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0));
    let host = Host {
        tool: ConnectorTool::new(),
        targets,
        selected,
        view: CanvasView::new(bounds, Point::new(200.0, 150.0), 1.0),
        canvas: None,
        cursor: CursorIcon::Default,
        events: Vec::new(),
        hovered: None,
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "connector");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                host.tool.targets(&host.targets).selected(host.selected.as_ref());
                let r = ToolHost::new(id).show(ui, &host.view, &canvas, &mut host.tool);
                host.cursor = r.cursor;
                host.hovered = host.tool.hovered();
                for event in host.tool.take_events() {
                    if let ConnectorEvent::Reshape(EditEvent::Commit { value, .. }) = &event
                        && let Some(item) = host.selected.as_mut()
                    {
                        item.path = Some(value.clone());
                    }
                    host.events.push(event);
                }
            },
            host,
        );
    h.step();
    h
}

fn input(h: &mut Harness<'_, Host>, input: impl FnOnce(Modifiers) -> ToolInput) {
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input(Modifiers::NONE));
    h.step();
}

fn down(h: &mut Harness<'_, Host>, at: (f64, f64)) {
    input(h, |modifiers| ToolInput::Down {
        button: egui::PointerButton::Primary,
        doc: Point::new(at.0, at.1),
        pressure: 1.0,
        tilt: [0.0; 2],
        rotation: 0.0,
        modifiers,
    });
}

fn to(h: &mut Harness<'_, Host>, at: (f64, f64)) {
    input(h, |modifiers| ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Host>, at: (f64, f64)) {
    input(h, |modifiers| ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

/// A press, a move through the middle, a move to `b` and the release.
fn drag(h: &mut Harness<'_, Host>, a: (f64, f64), b: (f64, f64)) {
    down(h, a);
    to(h, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
    to(h, b);
    up(h, b);
}

fn drained(h: &mut Harness<'_, Host>) -> Vec<ConnectorEvent> {
    std::mem::take(&mut h.state_mut().events)
}

/// The draft and the `changed` flag of the draw commit among `events`.
fn draw_commit(events: &[ConnectorEvent]) -> Option<(&ConnectorDraft, bool)> {
    events.iter().find_map(|e| match e {
        ConnectorEvent::Draw(EditEvent::Commit { value, changed, .. }) => Some((value, *changed)),
        _ => None,
    })
}

/// The `Reattach` commit among `events`: the end, its value and `changed`.
fn reattach_commit(events: &[ConnectorEvent]) -> Option<(ConnectorEndKind, ConnectorEnd, bool)> {
    events.iter().find_map(|e| match e {
        ConnectorEvent::Reattach { end, event: EditEvent::Commit { value, changed, .. } } => Some((*end, *value, *changed)),
        _ => None,
    })
}

/// The x of the elbow's middle run between A's right side and E's left side, for `path`.
fn middle_x(path: Option<&ConnectorPath>) -> f64 {
    let from = RouteEnd::Box(rect(A), ConnectionSide::Right);
    let to = RouteEnd::Box(rect(E), ConnectionSide::Left);
    let Some(points) = route_along(from, to, Routing::Elbow, path) else { panic!("a route") };
    points[1].x
}

// Hovering a target paints its four side-middle sites; the site within reach is the active
// (larger) one.
#[test]
fn connector_hover_shows_attach_points_and_active_one_is_larger() {
    let mut h = harness(boxes(), None);
    let style = {
        let ctx = h.ctx.clone();
        ConnectorTool::style(&current(&ctx))
    };
    assert!(style.attach_active > style.attach, "the active site is the larger ring");

    // Near the right middle: that site is active (index 1 in Top, Right, Bottom, Left).
    to(&mut h, (97.0, 150.0));
    assert_eq!(h.state().hovered, Some((1, Some(1))));
    // Inside the bottom edge, near its middle.
    to(&mut h, (60.0, 192.0));
    assert_eq!(h.state().hovered, Some((1, Some(2))));
    // Far from every middle but towards the right: the side the point is towards is lit.
    to(&mut h, (80.0, 140.0));
    assert_eq!(h.state().hovered, Some((1, Some(1))));
    // The middle of the box chooses none; the sites still show.
    to(&mut h, (60.0, 150.0));
    assert_eq!(h.state().hovered, Some((1, None)));
    // Over nothing: no sites.
    to(&mut h, (225.0, 250.0));
    assert_eq!(h.state().hovered, None);
}

// A 3-point drag makes nothing; a drag from a box back onto it commits `changed: false`; a drag to
// another box commits a `ConnectorDraft` glued at both ends.
#[test]
fn connector_drag_under_four_points_or_onto_start_makes_nothing() {
    let mut h = harness(boxes(), None);
    drag(&mut h, (60.0, 150.0), (63.0, 150.0));
    assert!(drained(&mut h).is_empty(), "a drag under the click slop reports nothing");

    drag(&mut h, (60.0, 150.0), (50.0, 140.0));
    let events = drained(&mut h);
    assert!(matches!(events.first(), Some(ConnectorEvent::Draw(EditEvent::Begin { .. }))));
    let Some((_, changed)) = draw_commit(&events) else { panic!("a commit in {events:?}") };
    assert!(!changed, "ending on the start object makes nothing");

    drag(&mut h, (60.0, 150.0), (255.0, 150.0));
    let events = drained(&mut h);
    let Some((draft, changed)) = draw_commit(&events) else { panic!("a commit in {events:?}") };
    assert!(changed);
    assert_eq!(draft.from, glued(1, ConnectionSide::Auto));
    assert_eq!(draft.to, glued(2, ConnectionSide::Left));
    assert!(draft.points.len() >= 2);
}

// Dragging the selected connector's end onto a target reports `Reattach` glued there; onto empty
// canvas, free at the point.
#[test]
fn connector_reattach_to_target_or_free_point() {
    let selected = item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight);
    let mut h = harness(boxes(), Some(selected));

    drag(&mut h, (250.0, 150.0), (310.0, 40.0));
    let events = drained(&mut h);
    let Some((end, value, changed)) = reattach_commit(&events) else { panic!("a reattach commit in {events:?}") };
    assert_eq!((end, changed), (ConnectorEndKind::End, true));
    assert_eq!(value, ConnectorEnd::Free(Point::new(310.0, 40.0)));

    // The start end onto the box above: glued at its bottom middle.
    drag(&mut h, (100.0, 150.0), (175.0, 56.0));
    let events = drained(&mut h);
    let Some((end, value, _)) = reattach_commit(&events) else { panic!("a reattach commit in {events:?}") };
    assert_eq!(end, ConnectorEndKind::Start);
    assert_eq!(value, glued(3, ConnectionSide::Bottom));

    // Dropped on the object the other end is on: nothing changes.
    drag(&mut h, (100.0, 150.0), (290.0, 150.0));
    let events = drained(&mut h);
    let Some((_, _, changed)) = reattach_commit(&events) else { panic!("a reattach commit in {events:?}") };
    assert!(!changed);
}

// A target with no sites glues at side middles (FigCraft); one with numbered sites glues to the
// nearest site's index (DeckCraft).
#[test]
fn connector_sites_accept_side_middles_and_numbered_sites() {
    let numbered = ConnectionTarget {
        id: 4,
        name: None,
        bounds: Rect::new(150.0, 200.0, 250.0, 260.0),
        sites: vec![
            ConnectionSite { point: Point::new(160.0, 210.0), side: ConnectionSide::Top, index: 7 },
            ConnectionSite { point: Point::new(240.0, 250.0), side: ConnectionSide::Bottom, index: 9 },
        ],
    };
    let mut targets = boxes();
    targets.push(numbered);
    let mut h = harness(targets, None);

    drag(&mut h, (60.0, 150.0), (255.0, 150.0));
    let events = drained(&mut h);
    let Some((draft, _)) = draw_commit(&events) else { panic!("a commit in {events:?}") };
    assert_eq!(draft.to, glued(2, ConnectionSide::Left), "a side middle");

    drag(&mut h, (60.0, 150.0), (238.0, 248.0));
    let events = drained(&mut h);
    let Some((draft, _)) = draw_commit(&events) else { panic!("a commit in {events:?}") };
    assert_eq!(draft.to, ConnectorEnd::Glued { target: 4, side: ConnectionSide::Bottom, site: Some(9), at: None });

    // Inside the numbered target but out of reach of its sites: free.
    drag(&mut h, (60.0, 150.0), (200.0, 230.0));
    let events = drained(&mut h);
    let Some((draft, _)) = draw_commit(&events) else { panic!("a commit in {events:?}") };
    assert_eq!(draft.to, ConnectorEnd::Free(Point::new(200.0, 230.0)));
}

// Dragging an elbow's middle path handle reports `Reshape` with `ConnectorPath::Runs`.
#[test]
fn connector_path_handle_reshapes() {
    let mut targets = boxes();
    targets.push(target(5, E));
    let selected = item(glued(1, ConnectionSide::Right), glued(5, ConnectionSide::Left), Routing::Elbow);
    let mut h = harness(targets, Some(selected));

    // The elbow runs (100,150) -> (200,150) -> (200,50) -> (300,50); its middle run is at x = 200.
    assert_eq!(middle_x(None), 200.0);
    drag(&mut h, (200.0, 100.0), (180.0, 100.0));
    let events = drained(&mut h);
    let Some(ConnectorEvent::Reshape(EditEvent::Commit { value, changed, .. })) =
        events.iter().find(|e| matches!(e, ConnectorEvent::Reshape(EditEvent::Commit { .. })))
    else {
        panic!("a reshape commit in {events:?}")
    };
    assert!(changed);
    assert!(matches!(value, ConnectorPath::Runs { .. }));
    assert_eq!(middle_x(Some(value)), 180.0);

    // The path handles are found again on the reshaped route: the next drag grabs the middle run
    // where it now is.
    drag(&mut h, (180.0, 100.0), (160.0, 100.0));
    let events = drained(&mut h);
    let Some(ConnectorEvent::Reshape(EditEvent::Commit { value, .. })) =
        events.iter().find(|e| matches!(e, ConnectorEvent::Reshape(EditEvent::Commit { .. })))
    else {
        panic!("a second reshape commit in {events:?}")
    };
    assert_eq!(middle_x(Some(value)), 160.0);
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Reattach` and `Reshape`.
#[test]
fn connector_cancels_on_escape_and_on_tool_switch() {
    let mut targets = boxes();
    targets.push(target(5, E));
    let straight = item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight);
    let elbow = item(glued(1, ConnectionSide::Right), glued(5, ConnectionSide::Left), Routing::Elbow);
    let mut h = harness(targets, None);
    let cancels = |h: &mut Harness<'_, Host>| -> Vec<&'static str> {
        drained(h)
            .iter()
            .filter_map(|e| match e {
                ConnectorEvent::Draw(EditEvent::Cancel { .. }) => Some("draw"),
                ConnectorEvent::Reattach { event: EditEvent::Cancel { .. }, .. } => Some("reattach"),
                ConnectorEvent::Reshape(EditEvent::Cancel { .. }) => Some("reshape"),
                _ => None,
            })
            .collect()
    };
    for (selected, from, to_, want) in [
        (None, (300.0, 250.0), (350.0, 280.0), "draw"),
        (Some(straight), (250.0, 150.0), (310.0, 40.0), "reattach"),
        (Some(elbow.clone()), (200.0, 100.0), (180.0, 100.0), "reshape"),
    ] {
        h.state_mut().selected = selected;
        h.step();
        down(&mut h, from);
        to(&mut h, to_);
        h.key_press(Key::Escape);
        h.run();
        assert_eq!(cancels(&mut h), [want]);
        up(&mut h, to_);
        assert!(h.state().events.is_empty(), "{want}: the release after Esc reports nothing");
    }
    // The app switches tools mid-drag.
    h.state_mut().selected = Some(elbow);
    h.step();
    down(&mut h, (200.0, 100.0));
    to(&mut h, (180.0, 100.0));
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    let ctx = h.ctx.clone();
    ToolHost::cancel(&ctx, id, &mut h.state_mut().tool, CancelReason::Removed);
    h.step();
    assert_eq!(cancels(&mut h), ["reshape"]);
}

// The arrows move a picked part only while the parts group has the keyboard; the pick is
// forgotten when another connector is selected.
#[test]
fn connector_arrows_need_the_parts_focused_and_the_pick_follows_the_connector() {
    let straight = item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight);
    let mut h = harness(boxes(), Some(straight.clone()));
    // A press and release on the end handle picks it, with no drag.
    down(&mut h, (250.0, 150.0));
    up(&mut h, (250.0, 150.0));
    assert_eq!(h.state().tool.picked(), Some(1));
    drained(&mut h);
    h.key_press(Key::ArrowRight);
    h.run();
    assert!(drained(&mut h).is_empty(), "the canvas has the keyboard, not the parts group: arrows do nothing");

    let other = ConnectorItem { id: 10, ..straight };
    h.state_mut().selected = Some(other);
    h.step();
    assert_eq!(h.state().tool.picked(), None, "another connector: nothing is picked");
}

// A release the geometry refuses (an end the route can't be placed at) reports `Cancel` with
// `External`: no Esc was pressed.
#[test]
fn connector_refused_release_cancels_as_external() {
    let mut h = harness(boxes(), None);
    down(&mut h, (60.0, 250.0));
    to(&mut h, (200.0, 250.0));
    drained(&mut h);
    up(&mut h, (f64::NAN, f64::NAN));
    let events = drained(&mut h);
    assert!(
        events.iter().any(|e| matches!(e, ConnectorEvent::Draw(EditEvent::Cancel { reason: CancelReason::External, .. }))),
        "an External cancel in {events:?}"
    );
}

// Over an end handle of the selected connector the cursor is the move arrow, elsewhere the
// crosshair; read through real pointer events.
#[cfg(feature = "screen-testing")]
#[test]
fn connector_cursor_follows_the_pointer_through_the_rig() {
    use craft_ui::testing::canvas::move_to;
    let selected = item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight);
    let mut h = harness(boxes(), Some(selected));
    move_to(&mut h, egui::pos2(250.0, 150.0));
    assert_eq!(h.state().cursor, CursorIcon::Move);
    move_to(&mut h, egui::pos2(60.0, 270.0));
    assert_eq!(h.state().cursor, CursorIcon::Crosshair);
}

// A glued end is named by its target's name, else by its number.
#[test]
fn connector_end_part_uses_the_target_name() {
    let named = vec![ConnectionTarget::new(1, rect(A)).named("Rectangle 1"), target(2, (250.0, 100.0, 330.0, 200.0))];
    let selected = item(glued(1, ConnectionSide::Right), glued(2, ConnectionSide::Left), Routing::Straight);
    let h = harness(named, Some(selected));
    assert!(h.query_by_label("Start, attached to Rectangle 1").is_some());
    assert!(h.query_by_label("End, attached to 2").is_some());
}
