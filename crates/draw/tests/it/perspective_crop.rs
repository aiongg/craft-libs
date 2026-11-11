//! The Perspective Crop quad and tool through the public API: the pure quad rules, the quad's gestures and keys as a
//! canvas control, and the tool's box drag and four clicks on the background.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, EditEvent};
use craft_draw::ui::crop::{
    PerspectiveCropEvent, PerspectiveCropQuad, PerspectiveCropTool, PerspectiveQuad, grid_lines, grid_steps, is_valid, ordered,
};
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::access::Role;
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const SIZE: Size = Size::new(400.0, 300.0);

const PAGE: [Point; 4] = [Point::new(100.0, 60.0), Point::new(300.0, 80.0), Point::new(280.0, 220.0), Point::new(120.0, 200.0)];

#[trial]
fn ordered_puts_shuffled_corners_clockwise_from_the_smallest_sum() {
    let shuffled = [PAGE[2], PAGE[0], PAGE[3], PAGE[1]];
    assert_eq!(ordered(shuffled), PAGE);
    assert_eq!(ordered(PAGE), PAGE);
}

#[trial]
fn a_quad_is_valid_when_convex_wound_clockwise_and_not_flat() {
    assert!(is_valid(&PAGE));
    assert!(!is_valid(&[PAGE[0], PAGE[3], PAGE[2], PAGE[1]]), "wound the other way");
    assert!(!is_valid(&[PAGE[0], PAGE[1], PAGE[2], Point::new(250.0, 90.0)]), "not convex");
    assert!(!is_valid(&[PAGE[0], PAGE[0], PAGE[0], PAGE[0]]), "no area");
    assert!(!is_valid(&[PAGE[0], PAGE[1], PAGE[2], Point::new(f64::NAN, 1.0)]), "not finite");
}

#[trial]
fn grid_lines_meet_at_the_quads_vanishing_points() {
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let upright = [Point::new(100.0, 50.0), Point::new(300.0, 50.0), Point::new(300.0, 190.0), Point::new(100.0, 190.0)];
    let lines = grid_lines(&view, &upright, 48.0);
    assert_eq!(lines.len(), 2 * (grid_steps(200.0, 48.0) - 1), "one line per step boundary in each direction");
    assert!(lines.iter().all(|[a, b]| (a.x - b.x).abs() < 1e-3 || (a.y - b.y).abs() < 1e-3), "an upright box has an axis-aligned grid");

    // A trapezoid narrower at the top: its rows crowd towards the far edge, as the object's own lines do.
    let trapezoid = [Point::new(150.0, 50.0), Point::new(250.0, 50.0), Point::new(300.0, 250.0), Point::new(100.0, 250.0)];
    let mut rows: Vec<f32> = grid_lines(&view, &trapezoid, 48.0).iter().filter(|[a, b]| (a.y - b.y).abs() < 1e-3).map(|[a, _]| a.y).collect();
    rows.sort_by(f32::total_cmp);
    let gaps: Vec<f32> = rows.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(gaps.len() >= 2 && gaps.windows(2).any(|pair| (pair[0] - pair[1]).abs() > 0.5), "perspective rows are not evenly spaced: {gaps:?}");
}

struct Host {
    tool: PerspectiveCropTool,
    apply: bool,
    discard: bool,
    picked: Option<usize>,
    edits: Vec<EditEvent<PerspectiveQuad>>,
}

struct Rig {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
}

impl Rig {
    fn new(quad: PerspectiveQuad) -> Self {
        Self::build(quad, false)
    }

    fn build(quad: PerspectiveQuad, refuse: bool) -> Self {
        let mut tool = PerspectiveCropTool::new();
        tool.set(quad);
        let host = Rc::new(RefCell::new(Host { tool, apply: false, discard: false, picked: None, edits: Vec::new() }));
        let shown = Rc::clone(&host);
        let mut fixture = Fixture::new(move |ui| {
            let room = ui.available_rect_before_wrap();
            let view = CanvasView::new(room, Point::new(200.0, 150.0), 1.0);
            let canvas = Canvas::new("Document").document(SIZE).view(view).show(ui, |ui, body| {
                let mut host = shown.borrow_mut();
                let bind = Bind::bound(host.tool.quad_mut()).check(move |quad: &PerspectiveQuad| match quad {
                    Some(quad) if refuse && !is_valid(quad) => Validity::Refuse(Message::new("The corners do not make a convex quad")),
                    _ => Validity::Ok,
                });
                let out = PerspectiveCropQuad::new("Perspective crop", bind).show_at(ui, body.rect);
                host.apply |= out.apply;
                host.discard |= out.discard;
                host.picked = out.picked;
                host.edits.extend(out.edit.events);
            });
            let mut host = shown.borrow_mut();
            let _ = ToolHost::new().run(ui, &canvas.output, &mut host.tool);
        });
        fixture.settle();
        Rig { fixture, host }
    }

    /// The screen point of document point `(x, y)`: the view shows (200, 150) at the canvas's centre.
    fn at(&self, p: Point) -> Pos2 {
        let canvas = self.fixture.by_role(Role::CANVAS).into_iter().next().expect("a canvas");
        canvas.center() + Vec2::new(p.x as f32 - 200.0, p.y as f32 - 150.0)
    }

    fn quad(&self) -> PerspectiveQuad {
        *self.host.borrow().tool.quad()
    }

    fn press_move_release(&mut self, from: Pos2, to: Pos2, escape_midway: bool) {
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![pointer::press(from)]);
        self.fixture.frame(vec![pointer::moved(from + (to - from) * 0.5)]);
        self.fixture.frame(vec![pointer::moved(to)]);
        if escape_midway {
            self.fixture.key(Key::Escape, Modifiers::NONE);
            self.fixture.frame(vec![pointer::moved(to + Vec2::new(5.0, 5.0))]);
        }
        self.fixture.frame(vec![pointer::release(to)]);
        self.fixture.frame(Vec::new());
    }

    fn click_at(&mut self, p: Point) {
        let at = self.at(p);
        self.fixture.frame(vec![pointer::moved(at)]);
        self.fixture.frame(vec![pointer::press(at)]);
        self.fixture.frame(vec![pointer::release(at)]);
        self.fixture.frame(Vec::new());
    }
}

#[trial]
fn dragging_a_corner_keeps_the_press_offset_and_moves_only_that_corner() {
    let mut rig = Rig::new(Some(PAGE));
    let corner = rig.at(PAGE[2]);
    rig.press_move_release(corner + Vec2::new(3.0, 2.0), corner + Vec2::new(23.0, -18.0), false);
    let quad = rig.quad().expect("the quad stays");
    assert_eq!((quad[2].x, quad[2].y), (PAGE[2].x + 20.0, PAGE[2].y - 20.0), "the press offset is kept");
    assert_eq!((quad[0], quad[1], quad[3]), (PAGE[0], PAGE[1], PAGE[3]));
}

#[trial]
fn a_box_drag_makes_an_upright_quad() {
    let mut rig = Rig::new(None);
    let (from, to) = (rig.at(Point::new(60.0, 50.0)), rig.at(Point::new(260.0, 190.0)));
    rig.press_move_release(from, to, false);
    assert_eq!(
        rig.quad(),
        Some([Point::new(60.0, 50.0), Point::new(260.0, 50.0), Point::new(260.0, 190.0), Point::new(60.0, 190.0)]),
        "a box drag makes an upright quad"
    );
    let events = rig.host.borrow_mut().tool.take_events();
    assert!(matches!(events.first(), Some(PerspectiveCropEvent::Edit(EditEvent::Begin { .. }))), "{events:?}");
    assert!(matches!(events.last(), Some(PerspectiveCropEvent::Edit(EditEvent::Commit { .. }))), "{events:?}");

    let mut rig = Rig::new(None);
    let at = rig.at(Point::new(60.0, 50.0));
    rig.press_move_release(at, at + Vec2::new(60.0, 0.0), false);
    assert_eq!(rig.quad(), None, "a box with no area is no quad");
}

#[trial]
fn four_clicks_make_an_ordered_quad_and_a_drag_clears_the_clicks() {
    let mut rig = Rig::new(None);
    for p in [PAGE[2], PAGE[0], PAGE[3]] {
        rig.click_at(p);
    }
    assert_eq!(rig.host.borrow().tool.placing(), 3);
    assert_eq!(rig.quad(), None);
    rig.click_at(PAGE[1]);
    assert_eq!(rig.quad(), Some(PAGE), "the fourth click makes the quad, corners put in order");
    assert_eq!(rig.host.borrow().tool.placing(), 0);

    let mut rig = Rig::new(None);
    rig.click_at(PAGE[0]);
    rig.click_at(PAGE[1]);
    let (from, to) = (rig.at(Point::new(60.0, 50.0)), rig.at(Point::new(260.0, 190.0)));
    rig.press_move_release(from, to, false);
    assert_eq!(rig.host.borrow().tool.placing(), 0, "a drag clears the clicks");
}
