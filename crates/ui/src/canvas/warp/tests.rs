use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect, Vec2};

use super::story::{WarpHost, english_names, eval, surface_at};
use super::{WarpEdit, WarpGrid, WarpMesh};
use crate::canvas::guides::GuideHost;
use crate::canvas::tests::TypingField;
use crate::canvas::{CanvasView, Guide, GuideNames, Guides, Orientation, RulerUnits};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::primitives::FocusRing;
use crate::theme::{Base, current, install};

/// The mesh before any drag: one patch, points every third of the way.
const RECT: Rect = Rect::new(100.0, 80.0, 280.0, 200.0);

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, turned by
/// `rotation` and mirrored by `flip`, with a warp mesh and optionally guides under it.
struct Host {
    host: WarpHost,
    rotation: f64,
    flip: bool,
    editable: bool,
    view: Option<CanvasView>,
    events: Vec<EditEvent<WarpEdit>>,
    picked: Option<usize>,
    claimed: bool,
    guides: Option<GuideHost>,
    guide_events: usize,
    field: TypingField,
}

fn harness_with(mesh: WarpMesh, rotation: f64, flip: bool, setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        host: WarpHost::new(mesh),
        rotation,
        flip,
        editable: true,
        view: None,
        events: Vec::new(),
        picked: None,
        claimed: false,
        guides: None,
        guide_events: 0,
        field: TypingField::default(),
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.field.show(ui);
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the grid.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let mesh = host.host.mesh.clone();
            let r = WarpGrid::new("warp", &mesh, english_names())
                .editable(host.editable)
                .surface(|p| surface_at(&mesh, p))
                .units(&px, &px)
                .show(ui, &canvas, &view);
            host.host.apply(&r);
            host.field.note(r.event.is_some());
            host.events.extend(r.event.clone());
            (host.picked, host.claimed) = (r.picked, r.claimed);
            if let Some(guides) = &mut host.guides {
                let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
                let g = Guides::new("guides", &guides.guides, &px, &px, names).interactive(true).show(ui, &canvas, &view);
                host.guide_events += usize::from(g.event.is_some());
                guides.apply(&g);
            }
        },
        host,
    )
}

fn harness(setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    harness_with(WarpMesh::identity(RECT, 1, 1), 0.0, false, setup)
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(p)
}

fn mesh<'h>(h: &'h Harness<'_, Host>) -> &'h WarpMesh {
    &h.state().host.mesh
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Moves to `from`, presses, moves to `to` and releases there.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false));
    h.run();
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
        })
        .collect()
}

/// The value of the last event that carries one.
fn last_edit(h: &Harness<'_, Host>) -> Option<WarpEdit> {
    h.state().events.iter().rev().find_map(|e| match e {
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(*value),
        _ => None,
    })
}

fn flat(shapes: &[Shape]) -> Vec<&Shape> {
    shapes.iter().flat_map(|s| if let Shape::Vec(v) = s { flat(v) } else { vec![s] }).collect()
}

/// The anchors and the handle arms the last frame drew.
fn drawn(h: &Harness<'_, Host>) -> (usize, usize) {
    let style = WarpGrid::style(&current(&h.ctx));
    let shapes: Vec<Shape> = h.output().shapes.iter().map(|c| c.shape.clone()).collect();
    let shapes = flat(&shapes);
    let anchors = shapes.iter().filter(|s| matches!(s, Shape::Rect(r) if r.fill == style.anchor.fill && r.stroke.width == 0.0)).count();
    let arms = shapes.iter().filter(|s| matches!(s, Shape::LineSegment { stroke, .. } if *stroke == style.arm)).count();
    (anchors, arms)
}

#[test]
fn anchors_always_show_and_handles_only_near_the_pointer_or_picked() {
    let mut h = harness(|_| {});
    h.run();
    assert_eq!(drawn(&h), (4, 0), "no pointer: the anchors only");
    let anchor = screen(&h, Point::new(RECT.x0, RECT.y0));
    let out = vec2(-1.0, -1.0).normalized();
    h.event(Event::PointerMoved(anchor + out * 55.0));
    h.run();
    assert_eq!(drawn(&h), (4, 2), "55 pt from the top left anchor: its right and down handles");
    h.event(Event::PointerMoved(anchor + out * 57.0));
    h.run();
    assert_eq!(drawn(&h), (4, 0), "57 pt away");
    // Near the right handle of the top left anchor, more than 56 pt from the anchor itself.
    h.event(Event::PointerMoved(screen(&h, Point::new(160.0, 60.0))));
    h.run();
    assert_eq!(drawn(&h), (4, 1), "near a handle, away from its anchor: that handle");

    h.event(Event::PointerGone);
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(1));
    assert_eq!(drawn(&h), (4, 2), "a picked handle shows with its anchor's");
    h.key_press(Key::End);
    h.run();
    assert_eq!(h.state().picked, Some(14));
    assert_eq!(drawn(&h), (4, 2), "the bottom right anchor's");

    let mut h = harness(|host| host.editable = false);
    h.run();
    h.event(Event::PointerMoved(anchor));
    h.run();
    assert_eq!(drawn(&h), (0, 0), "not editable: the patch boundaries only");
    drag(&mut h, anchor, anchor + vec2(20.0, 0.0));
    assert!(h.state().events.is_empty() && !h.state().claimed);
}

#[test]
fn the_nearest_point_within_12_points_takes_the_press_in_a_turned_flipped_view() {
    for (offset, taken) in [(11.9, true), (12.1, false)] {
        let mut h = harness_with(WarpMesh::identity(RECT, 1, 1), 30.0, true, |_| {});
        h.run();
        // Out of the mesh along the bottom right anchor's diagonal, as the view draws it.
        let anchor = screen(&h, Point::new(RECT.x1, RECT.y1));
        let out = (anchor - screen(&h, RECT.center())).normalized();
        let at = anchor + out * offset;
        drag(&mut h, at, at + out * 20.0);
        match last_edit(&h) {
            Some(WarpEdit::Point { index, by }) => {
                assert!(taken, "{offset} pt");
                assert_eq!(index, 15);
                assert!((by.hypot() - 20.0).abs() < 1e-3, "{by:?}");
            }
            other => assert!(!taken && other.is_none(), "{offset} pt: {other:?}"),
        }
    }

    // On a small mesh a handle 3 pt away beats its anchor 5 pt away.
    let mut h = harness_with(WarpMesh::identity(Rect::new(190.0, 140.0, 214.0, 158.0), 1, 1), 0.0, false, |_| {});
    h.run();
    let at = screen(&h, Point::new(195.0, 140.0));
    drag(&mut h, at, at + vec2(0.0, -10.0));
    assert!(matches!(last_edit(&h), Some(WarpEdit::Point { index: 1, .. })), "{:?}", last_edit(&h));
    assert_eq!(h.state().picked, Some(1));
}

#[test]
fn a_press_on_the_surface_pulls_it_at_its_parameters() {
    let mut h = harness(|_| {});
    h.run();
    let centre = RECT.center();
    let from = screen(&h, centre);
    drag(&mut h, from, from + vec2(20.0, 10.0));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
    let Some(WarpEdit::Surface { s, t, by }) = last_edit(&h) else { panic!("{:?}", h.state().events) };
    assert!((s - 0.5).abs() < 1e-3 && (t - 0.5).abs() < 1e-3, "({s}, {t})");
    assert!((by - Vec2::new(20.0, 10.0)).hypot() < 1e-3, "{by:?}");
    assert!(eval(mesh(&h), 0.5, 0.5).distance(centre + by) < 1e-6, "the host's pull follows the pointer");
    assert!(!h.state().claimed, "released");

    // Off the surface and off the points the press is the tool's.
    let off = screen(&h, Point::new(40.0, 40.0));
    h.event(Event::PointerMoved(off));
    h.event(button(off, true));
    h.run();
    assert!(!h.state().claimed);
    h.event(button(off, false));
    h.run();
}

#[test]
fn esc_cancels_a_drag_and_the_rest_of_the_press_does_nothing() {
    let mut h = harness(|_| {});
    h.run();
    let before = mesh(&h).clone();
    let anchor = screen(&h, Point::new(RECT.x0, RECT.y0));
    h.event(Event::PointerMoved(anchor));
    h.event(button(anchor, true));
    h.run();
    h.event(Event::PointerMoved(anchor + vec2(30.0, 0.0)));
    h.run();
    assert_eq!(mesh(&h).points[0], Point::new(RECT.x0 + 30.0, RECT.y0), "the host shows the drag");
    assert_eq!(mesh(&h).points[1].x - before.points[1].x, 30.0, "the anchor carries its handles");
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert_eq!(*mesh(&h), before);
    assert!(h.state().claimed, "the press stays the grid's");
    let count = h.state().events.len();
    h.event(Event::PointerMoved(anchor + vec2(60.0, 0.0)));
    h.event(button(anchor + vec2(60.0, 0.0), false));
    h.run();
    assert_eq!(h.state().events.len(), count, "the rest of the press does nothing");
    assert_eq!(*mesh(&h), before);
}

#[test]
fn arrows_move_the_picked_point() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert!(h.state().events.is_empty(), "no keys without focus");
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().picked, Some(0), "focus from the keyboard picks the first anchor");
    h.key_press(Key::PageDown);
    h.run();
    let handle = mesh(&h).points[1];
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(last_edit(&h), Some(WarpEdit::Point { index: 1, by: Vec2::new(1.0, 0.0) }));
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(last_edit(&h), Some(WarpEdit::Point { index: 1, by: Vec2::new(0.0, 10.0) }));
    assert_eq!(kinds(&h), ["set", "set"]);
    assert_eq!(mesh(&h).points[1], handle + Vec2::new(1.0, 10.0));
    assert_eq!(mesh(&h).points[0], Point::new(RECT.x0, RECT.y0), "the handle moves alone");
}

#[test]
fn the_grid_takes_a_press_on_a_point_before_a_guide_under_it() {
    let mut h = harness(|host| host.guides = Some(GuideHost::new(vec![Guide::new(1, Orientation::Vertical, RECT.x0)])));
    h.run();
    let anchor = screen(&h, Point::new(RECT.x0, RECT.y0));
    drag(&mut h, anchor, anchor + vec2(20.0, 10.0));
    assert!(matches!(last_edit(&h), Some(WarpEdit::Point { index: 0, .. })));
    assert_eq!(h.state().guide_events, 0);
    assert_eq!(h.state().guides.as_ref().map(|g| g.guides[0].position), Some(RECT.x0));
}

/// Queues `events` into one frame, as winit delivers a quick gesture, and runs it.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

#[test]
fn a_quick_drag_in_one_frame_starts_at_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(RECT.x1, RECT.y0));
    let to = from + vec2(40.0, 10.0);
    frame(&mut h, [Event::PointerMoved(from), button(from, true), Event::PointerMoved(from + vec2(20.0, 5.0)), Event::PointerMoved(to)]);
    frame(&mut h, [button(to, false)]);
    h.run();
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
    assert_eq!(last_edit(&h), Some(WarpEdit::Point { index: 3, by: Vec2::new(40.0, 10.0) }));
    assert_eq!(mesh(&h).points[3], Point::new(RECT.x1 + 40.0, RECT.y0 + 10.0));
}

/// The shapes of the last frame stroked in `color`.
fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    let shapes: Vec<Shape> = h.output().shapes.iter().map(|c| c.shape.clone()).collect();
    flat(&shapes).iter().filter(|s| matches!(s, Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0)).count()
}

#[test]
fn the_grid_is_one_tab_stop_with_its_focus_after_keyboard_input_only() {
    let mut h = harness(|_| {});
    h.run();
    let focus = FocusRing::style(&current(&h.ctx)).stroke.color;
    let anchor = screen(&h, Point::new(RECT.x0, RECT.y0));
    drag(&mut h, anchor, anchor);
    assert_eq!(h.state().picked, Some(0), "the pointer picks the point it took");
    assert!(h.state().events.is_empty(), "a click on a point is no event");
    assert_eq!(painted(&h, focus), 0, "no focus after the pointer");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(1), "the release gave the grid the keyboard");
    assert_eq!(painted(&h, focus), 1, "the ring round the picked handle");
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn the_points_are_graphics_objects_in_a_group() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    let group = h.get_by_label("Warp");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "one Tab stop");
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let s = |name: &str, value: &str, selected: bool| (name.to_owned(), value.to_owned(), Some(selected));
    assert_eq!(
        objects,
        [
            s("Row 1, column 1", "100.0 px, 80.0 px", false),
            s("Row 1, column 1, right handle", "160.0 px, 80.0 px", true),
            s("Row 1, column 1, down handle", "100.0 px, 120.0 px", false),
            s("Row 1, column 2", "280.0 px, 80.0 px", false),
            s("Row 1, column 2, left handle", "220.0 px, 80.0 px", false),
            s("Row 1, column 2, down handle", "280.0 px, 120.0 px", false),
            s("Row 2, column 1", "100.0 px, 200.0 px", false),
            s("Row 2, column 1, up handle", "100.0 px, 160.0 px", false),
            s("Row 2, column 1, right handle", "160.0 px, 200.0 px", false),
            s("Row 2, column 2", "280.0 px, 200.0 px", false),
            s("Row 2, column 2, up handle", "280.0 px, 160.0 px", false),
            s("Row 2, column 2, left handle", "220.0 px, 200.0 px", false),
        ],
        "anchors row by row, each followed by its handles"
    );
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Row 1, column 1, right handle"));
}

#[test]
fn a_mesh_names_its_anchors_and_handles() {
    let m = WarpMesh::identity(RECT, 3, 2);
    assert!(m.is_valid());
    assert_eq!((m.width(), m.height(), m.points.len()), (10, 7, 70));
    // Column 4, row 3 is an anchor; column 5 of that row its right handle; column 4, row 4 the
    // down handle; column 5, row 4 a point inside a patch.
    assert_eq!(m.anchor_of(3 * 10 + 3), Some(33));
    assert_eq!(m.anchor_of(3 * 10 + 4), Some(33));
    assert_eq!(m.anchor_of(3 * 10 + 5), Some(36));
    assert_eq!(m.anchor_of(4 * 10 + 3), Some(33));
    assert_eq!(m.anchor_of(4 * 10 + 4), None);
}

/// A press on a point during a text field's typing run begins after the run's Commit and still
/// drags from the press.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = harness(|host| host.field.on = true);
    h.step();
    let anchor = screen(&h, Point::new(RECT.x1, RECT.y1));
    h.event(Event::PointerMoved(anchor));
    h.step();
    h.event(button(anchor, true));
    h.step();
    assert!(h.state().events.is_empty(), "the press waits: {:?}", h.state().events);
    for i in 1..=6 {
        h.event(Event::PointerMoved(anchor + vec2(4.0 * i as f32, 0.0)));
        h.step();
    }
    h.event(button(anchor + vec2(24.0, 0.0), false));
    h.step();
    h.step();
    let events = &h.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");
    assert!(h.state().field.commit_came_first());
}
