//! Stories of the margin handles: the edges of a page on the rulers of a page canvas.

use super::{MarginHandles, MarginRules, PageEdges};
use crate::setup::Margins;
use crate::ui::fixtures::demo_placed;
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, RulerUnits};
use craft_ui::canvas::{Canvas, RulerText, Rulers};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// The document point the view is centred on.
const CENTRE: Point = Point::new(300.0, 400.0);

/// How far the drag moves the left edge, in drag thresholds.
const DRAG_THRESHOLDS: f32 = 12.0;

fn drag_left_edge(fixture: &mut Fixture) {
    let Some(target) = fixture.by_label("Left Margin") else { return };
    let travel = DRAG_THRESHOLDS * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&target, Vec2::new(travel, 0.0));
}

#[story(MarginHandles<'static>, covers = [Default], drive = Drive::Custom(drag_left_edge))]
fn margin_handles_on_rulers(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (edges, zero) = data.value("state", (PageEdges::new(Margins::uniform(72.0)).header_footer(36.0, 36.0), Point::ZERO));
    let full = ui.available_rect_before_wrap();
    let view = CanvasView::new(full, CENTRE, 1.0);
    let text = RulerText::new(RulerUnits::pixels(), UnitKind::Points);
    Canvas::new("Pages")
        .view(view)
        .rulers(text, text, *zero)
        .show(ui, |ui, body| {
            let page = demo_placed(1, CanvasView::new(body.rect, CENTRE, 1.0).to_screen(Point::ZERO)).remove(0);
            let rules = MarginRules::new((612.0, 792.0), *edges);
            Rulers::new("Rulers", Bind::bound(&mut *zero)).margins(MarginHandles::spans(&rules, Point::ZERO)).show_at(ui, body.rect);
            MarginHandles::new("Margins", &page, Bind::bound(&mut *edges)).header_footer().unit(UnitKind::Inches).show_at(ui, body.rect)
        })
        .inner
        .map(StoryOutput::from)
        .unwrap_or_default()
}
