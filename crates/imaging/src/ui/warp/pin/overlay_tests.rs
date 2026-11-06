//! Tests of the pin overlay and the mesh painter.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Point;

use super::*;
use crate::ui::warp::{MeshStyle, paint_mesh};
use craft_ui::canvas::CanvasView;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

/// A solver whose mesh is the 400 × 300 canvas, undeformed unless a test turns it.
struct Flat {
    rest: Vec<Point>,
    deformed: Vec<Point>,
    triangles: Vec<[u32; 3]>,
    order: Vec<u32>,
}

impl Flat {
    fn new() -> Self {
        let rest = vec![Point::new(0.0, 0.0), Point::new(400.0, 0.0), Point::new(400.0, 300.0), Point::new(0.0, 300.0)];
        Self { deformed: rest.clone(), rest, triangles: vec![[0, 1, 2], [0, 2, 3]], order: vec![0, 1] }
    }
}

impl PinSolver for Flat {
    fn rebind(&mut self, _: &[Pin], _: &PinWarpOptions) {}
    fn solve(&mut self, _: &[Pin], _: &PinWarpOptions) {}
    fn remesh(&mut self, _: &PinWarpOptions) -> Result<(), String> {
        Ok(())
    }
    fn mesh(&self) -> PinMeshView<'_> {
        PinMeshView { rest: &self.rest, deformed: &self.deformed, triangles: &self.triangles, order: &self.order }
    }
}

struct Host {
    session: PinWarpSession,
    solver: Flat,
    view: Option<CanvasView>,
    layer: Option<egui::LayerId>,
    modifiers: Modifiers,
    edits: Vec<PinEdit>,
    apply: bool,
    discard: bool,
    claimed: bool,
    hovered: Option<usize>,
    /// What the overlay drew in the last pass.
    drawn: Vec<Shape>,
}

fn pin(x: f64, y: f64) -> Pin {
    Pin { rest: Point::new(x, y), at: Point::new(x, y), angle: None, depth: 0 }
}

fn harness(pins: Vec<Pin>, selected: Vec<usize>) -> Harness<'static, Host> {
    let mut session = PinWarpSession::from_pins(pins, &PinWarpSettings::default());
    let mut solver = Flat::new();
    session.apply(PinEdit::Select(selected), &mut solver, 0.0).expect("select");
    let host = Host {
        session,
        solver,
        view: None,
        layer: None,
        modifiers: Modifiers::NONE,
        edits: Vec::new(),
        apply: false,
        discard: false,
        claimed: false,
        hovered: None,
        drawn: Vec::new(),
    };
    let mut h = Harness::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            host.layer = Some(ui.layer_id());
            let r = PinWarp::new("pins", &host.session).modifiers(host.modifiers).show(ui, &canvas, &view, host.solver.mesh());
            host.drawn =
                ui.ctx().graphics(|g| g.get(ui.layer_id()).map(|list| list.all_entries().map(|c| c.shape.clone()).collect()).unwrap_or_default());
            let now = ui.input(|i| i.time);
            for edit in &r.edits {
                host.session.apply(edit.clone(), &mut host.solver, now).expect("applies");
            }
            host.session.settle(now, ui.input(|i| i.pointer.any_down()));
            host.edits.extend(r.edits);
            host.apply |= r.apply;
            host.discard |= r.discard;
            (host.claimed, host.hovered) = (r.claimed, r.hovered);
        },
        host,
    );
    h.run_steps(3);
    h
}

fn at(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    h.state().view.expect("drawn").to_screen(Point::new(x, y))
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn drag(h: &mut Harness<'_, Host>, from: (f64, f64), to: (f64, f64), modifiers: Modifiers) {
    let (from, to) = (at(h, from.0, from.1), at(h, to.0, to.1));
    h.state_mut().modifiers = modifiers;
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false));
    h.run_steps(2);
    h.state_mut().modifiers = Modifiers::NONE;
}

fn click(h: &mut Harness<'_, Host>, p: (f64, f64), modifiers: Modifiers) {
    drag(h, p, p, modifiers);
}

fn selected(h: &Harness<'_, Host>) -> Vec<usize> {
    h.state().session.selected().to_vec()
}

fn places(h: &Harness<'_, Host>) -> Vec<Point> {
    h.state().session.pins().iter().map(|p| p.at).collect()
}

fn pins_events(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .edits
        .iter()
        .filter_map(|e| match e {
            PinEdit::Pins(EditEvent::Begin { .. }) => Some("begin"),
            PinEdit::Pins(EditEvent::Commit { .. }) => Some("commit"),
            PinEdit::Pins(EditEvent::Cancel { .. }) => Some("cancel"),
            PinEdit::Pins(EditEvent::Set { .. }) => Some("set"),
            _ => None,
        })
        .collect()
}

/// The shapes the overlay drew in the last pass.
fn shapes(h: &Harness<'_, Host>) -> Vec<Shape> {
    h.state().drawn.clone()
}

// A click on the mesh places a pin at the rest position under it, selects it and its drag moves it:
// one step. A click off the mesh does nothing.
#[test]
fn a_click_on_the_mesh_places_a_selected_pin_and_its_drag_moves_it() {
    let mut h = harness(vec![pin(100.0, 100.0)], vec![]);
    drag(&mut h, (200.0, 150.0), (230.0, 170.0), Modifiers::NONE);
    assert_eq!(h.state().session.pins().len(), 2);
    let placed = h.state().session.pins()[1];
    assert_eq!(placed.rest, Point::new(200.0, 150.0), "attached where it was placed");
    assert_eq!(placed.at, Point::new(230.0, 170.0), "and moved by the drag");
    assert_eq!(selected(&h), vec![1]);
    assert_eq!(pins_events(&h), ["begin", "commit"]);
    assert!(h.state().claimed || !h.state().session.dragging());

    let before = h.state().edits.len();
    let off = Pos2::new(5.0, 5.0);
    h.event(Event::PointerMoved(off));
    h.event(button(off, true));
    h.event(button(off, false));
    h.run_steps(2);
    assert_eq!(h.state().edits.len(), before, "a click off the canvas does nothing");
    assert_eq!(h.state().session.pins().len(), 2);
}

// Shift-click adds a pin to the selection and takes it out; dragging a selected pin moves every
// selected pin as one Pins gesture.
#[test]
fn shift_click_toggles_the_selection_and_a_drag_moves_every_selected_pin() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0), pin(300.0, 100.0)], vec![0]);
    click(&mut h, (200.0, 100.0), Modifiers::SHIFT);
    assert_eq!(selected(&h), vec![0, 1]);
    click(&mut h, (300.0, 100.0), Modifiers::SHIFT);
    click(&mut h, (300.0, 100.0), Modifiers::SHIFT);
    assert_eq!(selected(&h), vec![0, 1], "the second shift-click takes it out");

    h.state_mut().edits.clear();
    drag(&mut h, (100.0, 100.0), (130.0, 120.0), Modifiers::NONE);
    assert_eq!(places(&h), vec![Point::new(130.0, 120.0), Point::new(230.0, 120.0), Point::new(300.0, 100.0)]);
    assert_eq!(pins_events(&h), ["begin", "commit"], "one gesture for both pins");
    assert_eq!(selected(&h), vec![0, 1]);

    // A plain press on an unselected pin selects only it.
    click(&mut h, (300.0, 100.0), Modifiers::NONE);
    assert_eq!(selected(&h), vec![2]);
}

// ⌥-drag within the turn radius of a selected pin, not on it, reports the pin with its angle
// set to the drag's angle.
#[test]
fn alt_drag_near_a_selected_pin_turns_it_and_fixes_its_rotation() {
    let mut h = harness(vec![pin(200.0, 150.0)], vec![0]);
    let turn = craft_ui::style::transform::TransformStyle::new(&craft_ui::theme::current(&h.ctx)).pin_turn;
    drag(&mut h, (200.0 + f64::from(turn) - 2.0, 150.0), (200.0, 190.0), Modifiers::ALT);
    let turned = h.state().session.pins()[0];
    assert!((turned.angle.expect("fixed") - 90.0).abs() < 1e-9, "{:?}", turned.angle);
    assert_eq!(turned.at, Point::new(200.0, 150.0), "the pin stays where it is");
    assert_eq!(pins_events(&h), ["begin", "commit"]);

    // Far from the pin the press is not the overlay's.
    h.state_mut().edits.clear();
    drag(&mut h, (300.0, 150.0), (300.0, 190.0), Modifiers::ALT);
    assert!(h.state().edits.is_empty(), "{:?}", h.state().edits);
}

// ⌥-click on a pin deletes it as one step; Esc during a drag puts the pins back and Esc with no
// drag discards the session.
#[test]
fn alt_click_deletes_a_pin_and_esc_cancels_a_drag_or_discards() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    click(&mut h, (100.0, 100.0), Modifiers::ALT);
    assert_eq!(places(&h), vec![Point::new(200.0, 100.0)]);
    assert_eq!(pins_events(&h), ["set"]);

    let from = at(&h, 200.0, 100.0);
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from + vec2(40.0, 0.0)));
    h.run_steps(2);
    assert_eq!(places(&h), vec![Point::new(240.0, 100.0)]);
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert_eq!(places(&h), vec![Point::new(200.0, 100.0)], "cancelled");
    assert!(pins_events(&h).contains(&"cancel"));
    assert!(!h.state().discard, "the Esc ended the drag");
    h.event(button(from, false));
    h.run_steps(2);
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(h.state().discard);
}

// Delete reports the pins without the selected ones; ⌘A reports a selection of every pin.
#[test]
fn delete_removes_the_selected_pins_and_select_all_selects_every_pin() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0), pin(300.0, 100.0)], vec![]);
    click(&mut h, (100.0, 100.0), Modifiers::NONE);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(2);
    assert_eq!(selected(&h), vec![0, 1, 2]);
    h.key_press(Key::Delete);
    h.run_steps(2);
    assert!(h.state().session.pins().is_empty());

    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    click(&mut h, (200.0, 100.0), Modifiers::NONE);
    h.key_press(Key::Backspace);
    h.run_steps(2);
    assert_eq!(places(&h), vec![Point::new(100.0, 100.0)]);
    h.key_press(Key::Enter);
    h.run_steps(2);
    assert!(h.state().apply);
}

// Insert places a pin (at the pointer's rest position, else the mesh centre), Page keys pick,
// arrows move the picked pin, Delete removes it.
#[test]
fn the_keyboard_places_picks_moves_and_removes_pins() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    click(&mut h, (100.0, 100.0), Modifiers::NONE);
    h.key_press(Key::PageDown);
    h.run_steps(2);
    assert_eq!(selected(&h), vec![1]);
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(places(&h), vec![Point::new(100.0, 100.0), Point::new(201.0, 110.0)]);

    // The pointer is on the mesh: the pin goes under it.
    h.event(Event::PointerMoved(at(&h, 50.0, 60.0)));
    h.run_steps(2);
    h.key_press(Key::Insert);
    h.run_steps(2);
    assert_eq!(places(&h)[2], Point::new(50.0, 60.0));
    assert_eq!(h.state().session.pins()[2].rest, Point::new(50.0, 60.0));
    assert_eq!(selected(&h), vec![2]);

    // The pointer is off the canvas: the mesh centre.
    h.event(Event::PointerMoved(Pos2::new(3.0, 3.0)));
    h.run_steps(2);
    h.key_press(Key::Insert);
    h.run_steps(2);
    assert_eq!(places(&h)[3], Point::new(200.0, 150.0));

    h.key_press(Key::Delete);
    h.run_steps(2);
    assert_eq!(h.state().session.pins().len(), 3);
}

// Pins draw in depth order; the selected pin shows its core and, when fixed, its turn ring.
#[test]
fn pins_draw_in_depth_order_with_the_selected_core_and_turn_ring() {
    let mut top = pin(100.0, 100.0);
    top.depth = 5;
    let mut turned = pin(300.0, 100.0);
    turned.angle = Some(0.0);
    let h = harness(vec![top, pin(200.0, 100.0), turned], vec![2]);
    let style = PinWarp::style(&craft_ui::theme::current(&h.ctx));
    let circles: Vec<_> = shapes(&h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Circle(c) => Some(c),
            _ => None,
        })
        .collect();
    let near = |a: Pos2, b: Pos2| a.distance(b) <= 1.0;
    let discs: Vec<Pos2> = circles.iter().filter(|c| c.fill == style.fill && (c.radius - style.pin / 2.0).abs() <= 1.0).map(|c| c.center).collect();
    let want = [at(&h, 200.0, 100.0), at(&h, 300.0, 100.0), at(&h, 100.0, 100.0)];
    assert!(discs.len() == 3 && discs.iter().zip(want).all(|(a, b)| near(*a, b)), "the deepest first, the highest on top: {discs:?}");
    let cores: Vec<Pos2> = circles.iter().filter(|c| c.fill == style.core_color && c.radius == style.core / 2.0).map(|c| c.center).collect();
    assert!(cores.len() == 1 && near(cores[0], at(&h, 300.0, 100.0)), "only the selected pin has a core: {cores:?}");
    let rings = circles.iter().filter(|c| c.radius == style.turn_radius && c.stroke == style.turn).count();
    assert_eq!(rings, 1, "and only a fixed one a turn ring");
}

// `paint_mesh` draws each edge once in the mesh style; Show Mesh off draws no mesh line.
#[test]
fn the_mesh_painter_draws_each_edge_once() {
    let drawn = std::rc::Rc::new(std::cell::Cell::new(0));
    let seen = drawn.clone();
    let mut direct =
        Harness::builder().with_size([100.0, 100.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(move |ui| {
            let style = MeshStyle::of(&craft_ui::theme::current(ui.ctx()));
            let edges = [
                [Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0)],
                [Pos2::new(10.0, 0.0), Pos2::new(0.0, 10.0)],
                [Pos2::new(0.0, 10.0), Pos2::new(0.0, 0.0)],
            ];
            let before = ui.ctx().graphics(|g| g.get(ui.layer_id()).map_or(0, |l| l.all_entries().len()));
            paint_mesh(ui.painter(), edges, &style);
            let after = ui.ctx().graphics(|g| g.get(ui.layer_id()).map_or(0, |l| l.all_entries().len()));
            seen.set(after - before);
        });
    direct.run();
    assert_eq!(drawn.get(), 3, "one line per edge");

    // Through the overlay: the two triangles of the test mesh have five edges.
    let mut h = harness(vec![], vec![]);
    let mesh_style = PinWarp::style(&craft_ui::theme::current(&h.ctx)).mesh;
    let count =
        |h: &Harness<'_, Host>| shapes(h).iter().filter(|s| matches!(s, Shape::LineSegment { stroke, .. } if *stroke == mesh_style.line)).count();
    h.state_mut().session.apply(PinEdit::ShowMesh(true), &mut Flat::new(), 0.0).expect("applies");
    h.run_steps(2);
    assert_eq!(count(&h), 5);
    h.state_mut().session.apply(PinEdit::ShowMesh(false), &mut Flat::new(), 0.0).expect("applies");
    h.run_steps(2);
    assert_eq!(count(&h), 0);
}

// AccessKit: a group with one graphics object per pin named "Pin 2 of 5" with value "x, y".
#[test]
fn every_pin_is_named_and_valued_for_assistive_technology() {
    let mut turned = pin(300.0, 100.0);
    turned.angle = Some(30.0);
    let h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0), turned], vec![1]);
    assert!(h.query_by_role_and_label(Role::Group, "Puppet Warp").is_some());
    let second = h.get_by_label("Pin 2 of 3");
    assert_eq!(second.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(second.accesskit_node().value().as_deref(), Some("200.0 px, 100.0 px"));
    assert_eq!(second.accesskit_node().is_selected(), Some(true));
    assert_eq!(h.get_by_label("Pin 3 of 3").accesskit_node().value().as_deref(), Some("300.0 px, 100.0 px, turn 30°"));
    assert_eq!(h.get_by_label("Pin 1 of 3").accesskit_node().is_selected(), Some(false));
}

// ⌥-drag turns by the angle swept from the press: a pin fixed at 30° pressed at 90° and dragged to
// 180° ends at 120°, not 180°.
#[test]
fn alt_drag_turns_a_fixed_pin_by_the_swept_angle() {
    let mut fixed = pin(200.0, 150.0);
    fixed.angle = Some(30.0);
    let mut h = harness(vec![fixed], vec![0]);
    drag(&mut h, (200.0, 164.0), (186.0, 150.0), Modifiers::ALT);
    let angle = h.state().session.pins()[0].angle.expect("fixed");
    assert!((angle - 120.0).abs() < 1e-9, "{angle}");
}

// An Auto pin starts its turn from the deformed mesh's turn under it, so a press does not jump.
#[test]
fn alt_drag_turns_an_auto_pin_from_the_meshs_turn() {
    let mut h = harness(vec![pin(200.0, 150.0)], vec![0]);
    let (sin, cos) = 20f64.to_radians().sin_cos();
    let centre = Point::new(200.0, 150.0);
    h.state_mut().solver.deformed = h
        .state()
        .solver
        .rest
        .iter()
        .map(|p| {
            let (x, y) = (p.x - centre.x, p.y - centre.y);
            Point::new(centre.x + x * cos - y * sin, centre.y + x * sin + y * cos)
        })
        .collect();
    h.run_steps(2);
    drag(&mut h, (200.0, 164.0), (186.0, 150.0), Modifiers::ALT);
    let angle = h.state().session.pins()[0].angle.expect("fixed");
    assert!((angle - 110.0).abs() < 1e-9, "{angle}");
}

// Page Down past the last pin picks none, then the first; Page Up from the first picks none.
#[test]
fn page_keys_cycle_through_every_pin_and_then_none() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    click(&mut h, (100.0, 100.0), Modifiers::NONE);
    let mut picks = Vec::new();
    for key in [Key::PageDown, Key::PageDown, Key::PageDown, Key::PageUp] {
        h.key_press(key);
        h.run_steps(2);
        picks.push(selected(&h));
    }
    assert_eq!(picks, [vec![1], vec![], vec![0], vec![]]);
}

// Insert places nothing where the mesh has no rest position, not even at its centre.
#[test]
fn insert_places_no_pin_where_the_mesh_has_no_rest_position() {
    let mut h = harness(vec![pin(100.0, 100.0)], vec![]);
    click(&mut h, (100.0, 100.0), Modifiers::NONE);
    h.state_mut().solver.triangles.clear();
    h.event(Event::PointerMoved(Pos2::new(3.0, 3.0)));
    h.run_steps(2);
    h.key_press(Key::Insert);
    h.run_steps(2);
    assert_eq!(places(&h), vec![Point::new(100.0, 100.0)]);
}
