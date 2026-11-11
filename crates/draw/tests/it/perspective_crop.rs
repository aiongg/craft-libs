//! The Perspective Crop quad and tool through the public API: the pure quad rules, the quad's gestures and keys as a
//! canvas control, and the tool's box drag and four clicks on the background.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CanvasView, EditEvent};
use craft_draw::ui::crop::{PerspectiveCropQuad, PerspectiveCropTool, PerspectiveQuad, grid_lines, grid_steps, is_valid};
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const SIZE: Size = Size::new(400.0, 300.0);

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

    fn quad(&self) -> PerspectiveQuad {
        *self.host.borrow().tool.quad()
    }
}
