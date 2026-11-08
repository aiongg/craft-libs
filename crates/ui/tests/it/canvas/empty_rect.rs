//! A full-window dialog whose side columns need more than the window: the canvas region is empty, never inverted,
//! and a drag over it reaches the tool without asking to scroll past an edge.

use craft_behavior::{CancelReason, PixelSize, PointerButton};
use craft_ui::buttons::Button;
use craft_ui::canvas::{CanvasTool, PixelRect, RasterSource, ToolAction, ToolContext, ToolHost, ToolInput, ToolPath, ToolPointer};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::{FullWindowCanvas, FullWindowDialog, FullWindowSide, FullWindowView};
use craft_ui::layout_components::Row;
use craft_ui::primitives::{Draw, Revision};
use craft_ui::testing::{Fixture, trial};
use egui::{Color32, ColorImage, Id, Rect, Vec2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Grey;

impl RasterSource for Grey {
    fn size(&self) -> PixelSize {
        PixelSize::new(64, 48)
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::GRAY);
    }
}

/// A tool that claims every press and scrolls past the edge, as a marquee does.
#[derive(Default)]
struct Marquee {
    drags: usize,
}

impl CanvasTool for Marquee {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Claimed
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {
        self.drags += 1;
    }

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Select Area... command")
    }
}

/// What the canvas showed: its rect and id, and every movement past the edge the host reported.
#[derive(Default)]
struct Seen {
    canvas: Option<(Rect, Id)>,
    past_edge: Vec<Vec2>,
}

/// A dialog in a window narrower than its side column, which holds a row of buttons that does not wrap.
fn squeezed() -> (Fixture, Rc<RefCell<Seen>>, Rc<RefCell<Marquee>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let tool = Rc::new(RefCell::new(Marquee::default()));
    let (shared, held) = (seen.clone(), tool.clone());
    let view = Rc::new(Cell::new(FullWindowView::new()));
    let mut fixture = Fixture::new(move |ui| {
        let mut current = view.get();
        FullWindowDialog::new("Content-Aware Fill")
            .sides(|ui, _| {
                FullWindowSide::new("Settings").show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        for name in ["Sampling", "Colour", "Rotation", "Scale", "Mirror", "Output"] {
                            Button::new(name).show(ui);
                        }
                    });
                });
            })
            .show(ui, |ui, _| {
                let mut tool = held.borrow_mut();
                let shown = FullWindowCanvas::new(Bind::bound(&mut current), &Grey).tool(&mut *tool).show(ui);
                let mut seen = shared.borrow_mut();
                seen.canvas = Some((shown.response.rect, shown.response.id));
                seen.past_edge.extend(shown.tool.past_edge);
            });
        view.set(current);
    });
    fixture.room(vec2(200.0, 300.0)).settle();
    (fixture, seen, tool)
}

#[trial]
fn a_side_column_wider_than_the_window_leaves_an_empty_canvas_region() {
    let (_fixture, seen, _) = squeezed();
    let (rect, _) = seen.borrow().canvas.expect("the dialog shows its canvas");
    assert!(rect.min.x <= rect.max.x && rect.min.y <= rect.max.y, "the canvas region is never inverted: {rect:?}");
    assert!(!rect.is_positive(), "the side column took the room, so the canvas region is empty: {rect:?}");
}

#[trial]
fn a_drag_over_an_empty_canvas_reaches_the_tool_and_scrolls_past_no_edge() {
    let (mut fixture, seen, tool) = squeezed();
    let (rect, id) = seen.borrow().canvas.expect("the dialog shows its canvas");
    seen.borrow_mut().past_edge.clear();
    let ctx = fixture.ctx().clone();
    ToolHost::inject(&ctx, id, ToolInput::down(PointerButton::Primary, rect.min));
    fixture.settle();
    ToolHost::inject(&ctx, id, ToolInput::moved(rect.min + vec2(40.0, 30.0)));
    fixture.settle();
    ToolHost::inject(&ctx, id, ToolInput::up(rect.min + vec2(40.0, 30.0)));
    fixture.settle();
    assert!(tool.borrow().drags > 0, "the injected drag reached the tool");
    assert!(seen.borrow().past_edge.is_empty(), "an empty canvas has no edge to scroll past: {:?}", seen.borrow().past_edge);
}
