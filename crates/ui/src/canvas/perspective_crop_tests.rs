use egui::accesskit::Role;
use egui::epaint::ColorMode;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Point;

use super::{PerspectiveCropQuad, PerspectiveHit, PerspectiveQuad, english_names, grid_lines, grid_steps, hit, is_valid, ordered};
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::theme::{Base, ToEgui, current, install};

/// A valid quad: a skewed page.
const QUAD: [Point; 4] = [Point::new(100.0, 80.0), Point::new(300.0, 100.0), Point::new(280.0, 220.0), Point::new(120.0, 200.0)];

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

// Pure geometry.

#[test]
fn ordered_puts_shuffled_corners_clockwise_from_the_top_left() {
    let want = [p(0.0, 0.0), p(100.0, 5.0), p(90.0, 100.0), p(10.0, 90.0)];
    assert_eq!(ordered([p(10.0, 90.0), p(100.0, 5.0), p(0.0, 0.0), p(90.0, 100.0)]), want);
    assert_eq!(ordered([p(90.0, 100.0), p(10.0, 90.0), p(100.0, 5.0), p(0.0, 0.0)]), want);
    assert_eq!(ordered(want), want);
}

#[test]
fn a_quad_is_valid_when_convex_and_wound_clockwise_on_screen() {
    assert!(is_valid(&QUAD));
    let [a, b, c, d] = QUAD;
    assert!(!is_valid(&[a, c, b, d]), "a bow-tie");
    assert!(!is_valid(&[d, c, b, a]), "wound the other way");
    assert!(!is_valid(&[a, b, p(200.0, 90.0), d]), "a straight angle");
    assert!(!is_valid(&[a, b, c, p(200.0, 100.0)]), "a dent");
    assert!(!is_valid(&[a, b, c, p(f64::NAN, 1.0)]));
    assert!(is_valid(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]), "one unit² is enough");
    assert!(!is_valid(&[p(0.0, 0.0), p(0.9, 0.0), p(0.9, 0.9), p(0.0, 0.9)]), "under one unit² of area");
}

#[test]
fn the_nearest_corner_within_reach_wins_over_inside() {
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), p(200.0, 150.0), 1.0);
    let at = |q: Point| view.to_screen(q);
    assert_eq!(hit(&view, &QUAD, at(p(104.0, 83.0)), 8.0), PerspectiveHit::Corner(0), "inside the quad, near a corner");
    assert_eq!(hit(&view, &QUAD, at(p(95.0, 76.0)), 8.0), PerspectiveHit::Corner(0), "outside, near a corner");
    assert_eq!(hit(&view, &QUAD, at(p(200.0, 150.0)), 8.0), PerspectiveHit::Inside);
    assert_eq!(hit(&view, &QUAD, at(p(20.0, 20.0)), 8.0), PerspectiveHit::Outside);
    // Two corners in reach: the nearer one.
    let tight = [p(100.0, 80.0), p(106.0, 80.0), p(106.0, 140.0), p(100.0, 140.0)];
    assert_eq!(hit(&view, &tight, at(p(104.0, 80.0)), 8.0), PerspectiveHit::Corner(1));
}

#[test]
fn the_grid_step_count_follows_the_side_and_stays_from_2_to_12() {
    assert_eq!(grid_steps(10.0, 48.0), 2);
    assert_eq!(grid_steps(48.0 * 5.0, 48.0), 5);
    assert_eq!(grid_steps(48.0 * 5.4, 48.0), 5);
    assert_eq!(grid_steps(48.0 * 5.6, 48.0), 6);
    assert_eq!(grid_steps(10_000.0, 48.0), 12);
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), p(200.0, 150.0), 1.0);
    for side in [30.0, 240.0, 5000.0] {
        let q = [p(0.0, 0.0), p(side, 0.0), p(side, side), p(0.0, side)];
        let n = grid_steps(side as f32, 48.0);
        assert_eq!(grid_lines(&view, &q, 48.0).len(), 2 * (n - 1), "side {side}");
    }
}

#[test]
fn grid_lines_meet_at_the_quads_vanishing_points() {
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), p(200.0, 150.0), 1.0);
    let q = [p(100.0, 80.0), p(300.0, 100.0), p(260.0, 240.0), p(130.0, 200.0)];
    let lines = grid_lines(&view, &q, 48.0);
    assert!(lines.len() >= 6);
    let meet = |a: [Pos2; 2], b: [Pos2; 2]| {
        let (d1, d2) = (a[1] - a[0], b[1] - b[0]);
        let t = (b[0] - a[0]).x * d2.y - (b[0] - a[0]).y * d2.x;
        a[0] + d1 * (t / (d1.x * d2.y - d1.y * d2.x))
    };
    let s = q.map(|c| view.to_screen(c));
    // Lines come in pairs (a column, then a row). The columns meet where the left and right sides do.
    let sides = meet([s[0], s[3]], [s[1], s[2]]);
    let columns: Vec<_> = lines.iter().step_by(2).copied().collect();
    for pair in columns.windows(2) {
        assert!(meet(pair[0], pair[1]).distance(sides) < 0.5, "{:?} vs {sides:?}", meet(pair[0], pair[1]));
    }
    let tops = meet([s[0], s[1]], [s[3], s[2]]);
    let rows: Vec<_> = lines.iter().skip(1).step_by(2).copied().collect();
    for pair in rows.windows(2) {
        assert!(meet(pair[0], pair[1]).distance(tops) < 0.5);
    }
}

// The component, through real input.

struct Host {
    quad: PerspectiveQuad,
    before: Option<PerspectiveQuad>,
    view: Option<CanvasView>,
    events: Vec<EditEvent<PerspectiveQuad>>,
    applied: usize,
    discarded: usize,
    picked: Option<usize>,
    committable: bool,
    dragging: bool,
    placing: usize,
    grid: bool,
    text: bool,
    text_value: String,
    disabled: bool,
    /// A button beside the canvas takes the keyboard focus.
    button: bool,
}

fn harness(quad: PerspectiveQuad, setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        quad,
        before: None,
        view: None,
        events: Vec::new(),
        applied: 0,
        discarded: 0,
        picked: None,
        committable: false,
        dragging: false,
        placing: 0,
        grid: true,
        text: false,
        text_value: String::new(),
        disabled: false,
        button: false,
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            if host.disabled {
                ui.disable();
            }
            if host.text {
                let label = ui.label("Name");
                let field = ui.add(egui::TextEdit::singleline(&mut host.text_value).id_salt("text field")).labelled_by(label.id);
                field.request_focus();
            }
            if host.button {
                ui.button("Other").request_focus();
            }
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the quad.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, p(200.0, 150.0), 1.0);
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let quad = host.quad;
            let r = PerspectiveCropQuad::new("pc", &quad, english_names()).show_grid(host.grid).units(&px, &px).show(ui, &canvas, &view);
            match &r.edit {
                Some(EditEvent::Begin { .. }) => host.before = Some(host.quad),
                Some(EditEvent::Update { value, .. }) => host.quad = *value,
                Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                    host.before = None;
                    host.quad = *value;
                }
                Some(EditEvent::Cancel { .. }) => {
                    if let Some(before) = host.before.take() {
                        host.quad = before;
                    }
                }
                None => {}
            }
            host.events.extend(r.edit.clone());
            host.applied += usize::from(r.apply);
            host.discarded += usize::from(r.discard);
            (host.picked, host.committable, host.dragging, host.placing) = (r.picked, r.committable, r.dragging, r.placing);
        },
        host,
    )
}

fn screen(h: &Harness<'_, Host>, q: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(q)
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Presses at `from`, moves to `to` and releases there.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false));
    h.run();
}

/// Presses at `from` and moves to `to`, the button held.
fn hold(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
}

/// A click: the press and the release in one frame, at a later time than the one before.
fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    let time = h.input().time.unwrap_or(0.0) + 1.0;
    h.input_mut().time = Some(time);
    h.event(Event::PointerMoved(at));
    h.event(button(at, true));
    h.event(button(at, false));
    h.run();
}

fn quad(h: &Harness<'_, Host>) -> PerspectiveQuad {
    h.state().quad
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-6
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

#[test]
fn dragging_a_corner_keeps_the_press_offset_and_moves_that_corner_alone() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    let from = screen(&h, QUAD[2]) + vec2(3.0, -2.0);
    drag(&mut h, from, from + vec2(20.0, 15.0));
    let q = quad(&h).unwrap();
    assert!(near(q[2], QUAD[2] + kurbo::Vec2::new(20.0, 15.0)), "{q:?}");
    assert!([0, 1, 3].iter().all(|&i| near(q[i], QUAD[i])));
    assert_eq!(kinds(&h), ["begin", "update", "commit"]);
    assert_eq!(h.state().picked, Some(2));
}

#[test]
fn dragging_inside_moves_the_quad_and_a_press_outside_does_nothing() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    let from = screen(&h, p(200.0, 150.0));
    drag(&mut h, from, from + vec2(30.0, -10.0));
    let q = quad(&h).unwrap();
    assert!((0..4).all(|i| near(q[i], QUAD[i] + kurbo::Vec2::new(30.0, -10.0))));
    let before = quad(&h);
    let count = h.state().events.len();
    let out = screen(&h, p(10.0, 10.0));
    drag(&mut h, out, out + vec2(40.0, 40.0));
    assert_eq!((quad(&h), h.state().events.len()), (before, count));
}

#[test]
fn a_box_drag_makes_an_upright_quad_and_one_with_no_area_makes_none() {
    let mut h = harness(None, |_| {});
    h.run();
    let a = screen(&h, p(20.0, 10.0));
    let to = screen(&h, p(120.0, 90.0));
    drag(&mut h, a, to);
    assert_eq!(quad(&h), Some([p(20.0, 10.0), p(120.0, 10.0), p(120.0, 90.0), p(20.0, 90.0)]));
    assert!(h.state().committable);
    assert_eq!(kinds(&h).first(), Some(&"begin"));

    for to in [p(120.0, 10.0), p(20.0, 90.0)] {
        let mut h = harness(None, |_| {});
        h.run();
        let a = screen(&h, p(20.0, 10.0));
        let to = screen(&h, to);
        drag(&mut h, a, to);
        assert_eq!(quad(&h), None, "drag to {to:?}");
        assert_eq!(h.state().placing, 0);
        assert!(!h.state().dragging);
    }
}

#[test]
fn four_clicks_make_an_ordered_quad_and_a_drag_clears_the_pending_clicks() {
    let mut h = harness(None, |_| {});
    h.run();
    let pts = [p(200.0, 200.0), p(100.0, 80.0), p(110.0, 210.0), p(210.0, 90.0)];
    for (i, c) in pts.iter().enumerate().take(3) {
        let at = screen(&h, *c);
        click(&mut h, at);
        assert_eq!((quad(&h), h.state().placing), (None, i + 1));
    }
    let at = screen(&h, pts[3]);
    click(&mut h, at);
    assert_eq!(h.state().placing, 0);
    let want = ordered(pts);
    let q = quad(&h).expect("the fourth click makes the quad");
    assert!((0..4).all(|i| near(q[i], want[i])), "{q:?}");
    assert_eq!(kinds(&h), ["set"]);
    assert!(h.state().committable);

    let mut h = harness(None, |_| {});
    h.run();
    let at = screen(&h, p(50.0, 50.0));
    click(&mut h, at);
    let at = screen(&h, p(150.0, 50.0));
    click(&mut h, at);
    assert_eq!(h.state().placing, 2);
    let a = screen(&h, p(30.0, 120.0));
    let to = screen(&h, p(130.0, 200.0));
    drag(&mut h, a, to);
    assert_eq!(h.state().placing, 0, "a drag clears the clicks");
    assert!(quad(&h).is_some());
}

#[test]
fn esc_during_a_drag_restores_the_quad_then_discards() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    let from = screen(&h, QUAD[0]);
    hold(&mut h, from, from + vec2(25.0, 25.0));
    assert!(h.state().dragging);
    assert!(near(quad(&h).unwrap()[0], QUAD[0] + kurbo::Vec2::new(25.0, 25.0)));
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(quad(&h), Some(QUAD), "Esc restores the quad at the press");
    assert_eq!(h.state().discarded, 0);
    assert_eq!(kinds(&h).last(), Some(&"cancel"));
    h.event(Event::PointerMoved(from + vec2(50.0, 50.0)));
    h.event(button(from + vec2(50.0, 50.0), false));
    h.run();
    assert_eq!(quad(&h), Some(QUAD), "the rest of the press does nothing");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 1);
}

#[test]
fn esc_drops_the_pending_clicks() {
    let mut h = harness(None, |_| {});
    h.run();
    let at = screen(&h, p(50.0, 50.0));
    click(&mut h, at);
    assert_eq!(h.state().placing, 1);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!((h.state().placing, h.state().discarded), (0, 0), "the first Esc drops the clicks only");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 1);
}

fn shapes_of(h: &Harness<'_, Host>, keep: impl Fn(&Shape) -> bool) -> usize {
    fn count(shape: &Shape, keep: &dyn Fn(&Shape) -> bool, n: &mut usize) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| count(s, keep, n)),
            s if keep(s) => *n += 1,
            _ => {}
        }
    }
    let mut n = 0;
    h.output().shapes.iter().for_each(|c| count(&c.shape, &keep, &mut n));
    n
}

fn grid_color(h: &Harness<'_, Host>) -> Color32 {
    current(&h.ctx).color.overlay.crop.to_egui().gamma_multiply(150.0 / 255.0)
}

#[test]
fn a_valid_quad_draws_its_grid_and_an_invalid_one_draws_red_without_it() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    let grid = grid_color(&h);
    let is_grid = move |s: &Shape| matches!(s, Shape::LineSegment { stroke, .. } if stroke.color == grid && (stroke.width - 0.75).abs() < 1e-6);
    let n = grid_steps(200.0, 48.0);
    assert!(h.state().committable);
    assert_eq!(shapes_of(&h, is_grid), 2 * (n - 1));
    let mut h = harness(Some(QUAD), |host| host.grid = false);
    h.run();
    assert_eq!(shapes_of(&h, is_grid), 0, "Show Grid off");

    let [a, b, c, d] = QUAD;
    let red = current(&h.ctx).color.overlay.crop_invalid.to_egui();
    let is_red =
        move |s: &Shape| matches!(s, Shape::Path(p) if p.stroke.color == ColorMode::Solid(red) && (p.stroke.width.round() - 2.0).abs() <= 1.0);
    let mut h = harness(Some([a, c, b, d]), |_| {});
    h.run();
    assert!(!h.state().committable);
    assert_eq!(shapes_of(&h, is_grid), 0);
    assert_eq!(shapes_of(&h, is_red), 1, "the outline");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().applied, 0, "an invalid quad cannot be committed");
}

#[test]
fn enter_is_ignored_while_another_widget_has_the_focus() {
    let mut h = harness(Some(QUAD), |host| host.button = true);
    h.run();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().applied, 0, "Enter belongs to the focused button");
}

#[test]
fn enter_applies_a_valid_quad() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!((h.state().applied, h.state().discarded), (1, 0));
}

#[test]
fn arrows_move_the_picked_corner_by_1_or_10_and_the_whole_quad_without_focus() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert!((0..4).all(|i| near(quad(&h).unwrap()[i], QUAD[i] + kurbo::Vec2::new(1.0, 0.0))), "no focus: the whole quad");

    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().picked, Some(0));
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(1));
    h.key_press(Key::ArrowDown);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    h.run();
    let q = quad(&h).unwrap();
    assert!(near(q[1], QUAD[1] + kurbo::Vec2::new(-10.0, 1.0)), "{q:?}");
    assert!([0, 2, 3].iter().all(|&i| near(q[i], QUAD[i])));
    assert_eq!(kinds(&h), ["set", "set"]);
}

#[test]
fn keys_are_ignored_while_a_text_field_has_focus() {
    let mut h = harness(Some(QUAD), |host| host.text = true);
    h.run();
    h.key_press(Key::Enter);
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!((h.state().applied, quad(&h)), (0, Some(QUAD)));
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn the_group_and_its_corners_are_named_in_the_accesskit_tree() {
    let mut h = harness(Some(QUAD), |_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let group = h.get_by_label("Perspective crop");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let labels: Vec<_> = objects.iter().map(|(l, ..)| l.as_str()).collect();
    assert_eq!(labels, ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"]);
    assert_eq!(objects[2].1, "280.0 px, 220.0 px");
    assert_eq!(objects.iter().filter(|(.., s)| *s == Some(true)).count(), 1);
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "one Tab stop");
}

#[test]
fn a_disabled_quad_draws_but_senses_nothing() {
    let mut h = harness(None, |host| host.disabled = true);
    h.run();
    let a = screen(&h, p(20.0, 10.0));
    let to = screen(&h, p(120.0, 90.0));
    drag(&mut h, a, to);
    assert_eq!((quad(&h), h.state().events.len()), (None, 0));
    let mut h = harness(Some(QUAD), |host| host.disabled = true);
    h.run();
    h.key_press(Key::Enter);
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!((h.state().applied, quad(&h)), (0, Some(QUAD)));
    assert!(shapes_of(&h, |s| matches!(s, Shape::Path(_))) >= 1, "the outline is drawn");
}
