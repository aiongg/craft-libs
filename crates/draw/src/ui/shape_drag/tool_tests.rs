//! Tests of the Shape tools on the tool host (task SH-T3).

use egui::accesskit::Role;
use egui::{Key, Modifiers, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{ShapeDragEvent, ShapeDragOptions, ShapeDragRules, ShapeDragTool, ShapeKind};
use crate::geometry::shape::ShapeGeometry;
use craft_ui::canvas::{CanvasId, CanvasTool as _, CanvasView, SnapGesture, ToolHost, ToolInput};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::theme::{Base, install};

struct Host {
    tool: ShapeDragTool,
    view: CanvasView,
    canvas: Option<CanvasId>,
    events: Vec<ShapeDragEvent>,
}

fn harness(zoom: f64, options: ShapeDragOptions, rules: ShapeDragRules) -> Harness<'static, Host> {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0));
    let mut tool = ShapeDragTool::new();
    tool.rules(rules);
    let host = Host { tool, view: CanvasView::new(rect, Point::new(200.0, 150.0), zoom), canvas: None, events: Vec::new() };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "shape");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                let _ = ToolHost::new(id).show(ui, &host.view, &canvas, &mut host.tool);
                let events = host.tool.take_events();
                host.events.extend(events);
            },
            host,
        );
    h.state_mut().tool.options(options);
    h.step();
    h
}

fn opts(kind: ShapeKind) -> ShapeDragOptions {
    ShapeDragOptions { kind, ..ShapeDragOptions::default() }
}

fn input(h: &mut Harness<'_, Host>, input: ToolInput) {
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input);
    h.step();
}

fn down(h: &mut Harness<'_, Host>, at: (f64, f64)) {
    input(
        h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: Point::new(at.0, at.1),
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers: Modifiers::NONE,
        },
    );
}

fn to(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

fn key(h: &mut Harness<'_, Host>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

/// Space going down or up alone: `key` presses and releases in one frame.
fn space(h: &mut Harness<'_, Host>, pressed: bool) {
    h.event(egui::Event::Key { key: Key::Space, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
    h.run();
}

/// Tab gives the canvas the keyboard (it is the first Tab stop).
fn focus(h: &mut Harness<'_, Host>) {
    key(h, Key::Tab, Modifiers::NONE);
}

fn last_update(h: &Harness<'_, Host>) -> Option<ShapeGeometry> {
    h.state().events.iter().rev().find_map(|e| match e {
        ShapeDragEvent::Gesture(EditEvent::Update { value, .. }) => Some(value.clone()),
        _ => None,
    })
}

fn committed(h: &Harness<'_, Host>) -> Option<(ShapeGeometry, bool)> {
    h.state().events.iter().find_map(|e| match e {
        ShapeDragEvent::Gesture(EditEvent::Commit { value, changed, .. }) => Some((value.clone(), *changed)),
        _ => None,
    })
}

fn radial(shape: Option<ShapeGeometry>) -> (f64, u32, f64) {
    let Some(ShapeGeometry::RadialPolygon { radius, sides, star_ratio, .. }) = shape else { panic!("a radial polygon, got {shape:?}") };
    (radius, sides, star_ratio)
}

// A drag of 0.4 px reports `Commit { changed: false }` after `Begin` (past the slop at zoom 8)
// and no shape.
#[test]
fn shape_drag_under_one_pixel_commits_nothing() {
    let mut h = harness(8.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (100.4, 100.4), Modifiers::NONE);
    up(&mut h, (100.4, 100.4), Modifiers::NONE);
    let events = &h.state().events;
    assert!(matches!(events.first(), Some(ShapeDragEvent::Gesture(EditEvent::Begin { .. }))), "{events:?}");
    assert!(last_update(&h).is_none(), "{events:?}");
    assert!(matches!(committed(&h), Some((_, false))), "{events:?}");
}

// At zoom 0.5 a 5 px move is a click and at zoom 4 a 1 px move is a drag: the slop is 3 screen
// points over the zoom (PhotoCraft `click_slop_scales_with_zoom`, `dialogs/shape_dialog/tests.rs`).
#[test]
fn shape_drag_click_reports_click_scaled_with_zoom() {
    let mut h = harness(0.5, opts(ShapeKind::Ellipse), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (105.0, 100.0), Modifiers::NONE);
    up(&mut h, (105.0, 100.0), Modifiers::NONE);
    assert!(
        matches!(h.state().events.as_slice(), [ShapeDragEvent::Click { at, kind: ShapeKind::Ellipse, .. }] if *at == Point::new(100.0, 100.0)),
        "{:?}",
        h.state().events
    );

    let mut h = harness(4.0, opts(ShapeKind::Ellipse), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (101.0, 100.0), Modifiers::NONE);
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    up(&mut h, (160.0, 140.0), Modifiers::NONE);
    assert!(matches!(committed(&h), Some((ShapeGeometry::Ellipse { .. }, true))), "{:?}", h.state().events);
}

// Space held mid-drag moves the box by the pointer's move and keeps its size; released, the drag
// sizes again from the moved press.
#[test]
fn space_moves_the_shape_being_drawn() {
    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    focus(&mut h);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    space(&mut h, true);
    to(&mut h, (200.0, 150.0), Modifiers::NONE);
    let Some(ShapeGeometry::Rect { rect, .. }) = last_update(&h) else { panic!("a rectangle") };
    assert_eq!((rect.x0, rect.y0, rect.width(), rect.height()), (140.0, 110.0, 60.0, 40.0));

    space(&mut h, false);
    to(&mut h, (230.0, 150.0), Modifiers::NONE);
    let Some(ShapeGeometry::Rect { rect, .. }) = last_update(&h) else { panic!("a rectangle") };
    assert_eq!((rect.x0, rect.y0, rect.width(), rect.height()), (140.0, 110.0, 90.0, 40.0));
}

// Illustrator rules: ↑ during a Polygon drag adds a side up to 100, ↓ removes one down to 3; the
// next `Update` carries the count; Photoshop rules leave the arrows alone.
#[test]
fn radial_polygon_arrow_keys_change_sides_between_3_and_100() {
    let mut h = harness(1.0, opts(ShapeKind::Polygon), ShapeDragRules::ILLUSTRATOR);
    focus(&mut h);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (100.0, 50.0), Modifiers::NONE);
    assert_eq!(radial(last_update(&h)), (50.0, 5, 1.0));
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    assert_eq!(radial(last_update(&h)).1, 6);
    for _ in 0..4 {
        key(&mut h, Key::ArrowDown, Modifiers::NONE);
    }
    key(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(radial(last_update(&h)).1, 3);
    for _ in 0..100 {
        key(&mut h, Key::ArrowUp, Modifiers::NONE);
    }
    assert_eq!(radial(last_update(&h)).1, 100);

    let mut h = harness(1.0, opts(ShapeKind::Polygon), ShapeDragRules::PHOTOSHOP);
    focus(&mut h);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    let Some(ShapeGeometry::Polygon { sides, .. }) = last_update(&h) else { panic!("a box polygon") };
    assert_eq!(sides, 5);
}

// Illustrator rules: with Ctrl/Cmd held a Star's inner radius stays while the outer follows; ⌥
// with five points gives the straight-shoulder ratio.
#[test]
fn star_ctrl_holds_inner_radius_while_outer_follows() {
    let options = ShapeDragOptions { star_ratio: 0.5, ..opts(ShapeKind::Star) };
    let mut h = harness(1.0, options, ShapeDragRules::ILLUSTRATOR);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (100.0, 50.0), Modifiers::NONE);
    assert_eq!(radial(last_update(&h)), (50.0, 5, 0.5));
    to(&mut h, (100.0, 0.0), Modifiers::COMMAND);
    let (radius, _, ratio) = radial(last_update(&h));
    assert!((radius - 100.0).abs() < 1e-9 && (ratio - 0.25).abs() < 1e-9, "{radius} {ratio}");

    to(&mut h, (100.0, 0.0), Modifiers::ALT);
    let (_, _, ratio) = radial(last_update(&h));
    let Some(straight) = crate::geometry::shape::RadialFrame::straight_shoulders(5) else { panic!("five points") };
    assert!((ratio - straight).abs() < 1e-9, "{ratio}");
}

// Enter with the canvas focused and no drag reports `Click` at the view centre.
#[test]
fn enter_with_canvas_focus_opens_create_shape_at_view_centre() {
    let mut h = harness(2.0, opts(ShapeKind::Star), ShapeDragRules::PHOTOSHOP);
    focus(&mut h);
    key(&mut h, Key::Enter, Modifiers::NONE);
    let events = &h.state().events;
    assert!(
        matches!(events.as_slice(), [ShapeDragEvent::Click { at, kind: ShapeKind::Star, .. }] if at.distance(Point::new(200.0, 150.0)) < 1e-6),
        "{events:?}"
    );
}

// Esc mid-drag and `ToolHost::cancel` both report `Cancel`; a press inside the slop then Esc
// reports nothing.
#[test]
fn shape_drag_cancels_on_escape_and_on_tool_switch() {
    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(matches!(h.state().events.last(), Some(ShapeDragEvent::Gesture(EditEvent::Cancel { reason: CancelReason::Escape, .. }))));

    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    let ctx = h.ctx.clone();
    let _ = ToolHost::cancel(&ctx, id, &mut h.state_mut().tool, CancelReason::Removed);
    let events = h.state_mut().tool.take_events();
    assert!(matches!(events.last(), Some(ShapeDragEvent::Gesture(EditEvent::Cancel { .. }))), "{events:?}");

    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (101.0, 100.0), Modifiers::NONE);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(h.state().events.is_empty(), "{:?}", h.state().events);
}

// The tool asks the host to snap the pointer while sizing and the moving box while Space is held:
// the box as it was when Space went down, shifted back by the pointer's travel since the press.
#[test]
fn space_changes_the_snap_from_the_point_to_the_moving_box() {
    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    focus(&mut h);
    down(&mut h, (100.0, 100.0));
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    assert_eq!(h.state().tool.snap_gesture(), Some(SnapGesture::Point));
    space(&mut h, true);
    assert_eq!(h.state().tool.snap_gesture(), Some(SnapGesture::Rect { at_press: Rect::new(40.0, 60.0, 100.0, 100.0) }));
    space(&mut h, false);
    assert_eq!(h.state().tool.snap_gesture(), Some(SnapGesture::Point));
}

// A begun drag adds one AccessKit object named for its size, with the hint as its value; it goes
// when the drag ends.
#[test]
fn shape_drag_node_names_the_size_while_drawing() {
    let mut h = harness(1.0, opts(ShapeKind::Rectangle), ShapeDragRules::PHOTOSHOP);
    down(&mut h, (100.0, 100.0));
    assert_eq!(h.query_all_by_role(Role::GraphicsObject).count(), 0, "no node before the drag begins");
    to(&mut h, (160.0, 140.0), Modifiers::NONE);
    let node = h.get_by_label("Drawing 60 by 40");
    assert_eq!(node.value().unwrap_or_default(), "Drag to draw · Click or press Enter to set a size");
    up(&mut h, (160.0, 140.0), Modifiers::NONE);
    assert_eq!(h.query_all_by_label("Drawing 60 by 40").count(), 0, "the node goes with the drag");
}
