//! Stories of the Pen: a draft on a canvas under the tool host, and the Path Options popover.

use super::bezier::{EditSubpath, Knot};
use super::{PathOptionsPopover, PathOptionsSettings, PenMode, PenTool};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_theme::Srgba;
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

/// Where the third anchor's out handle lies, relative to it.
const PULL: (f64, f64) = (30.0, -20.0);

fn corner(x: f64, y: f64) -> Knot {
    Knot::corner(Point::new(x, y))
}

/// Three anchors, the last with a handle pulled out.
fn pen_draft() -> EditSubpath {
    let last = Knot::smooth(Point::new(260.0, 120.0), Point::new(260.0 + PULL.0, 120.0 + PULL.1));
    EditSubpath { knots: vec![corner(80.0, 120.0), corner(160.0, 60.0), last], closed: false }
}

/// Four anchors with the curve flowing through them.
fn curvature_draft() -> EditSubpath {
    let smooth = |x: f64, y: f64| Knot { smooth: true, ..corner(x, y) };
    let mut draft = EditSubpath { knots: vec![smooth(80.0, 120.0), smooth(160.0, 60.0), smooth(240.0, 120.0), smooth(320.0, 60.0)], closed: false };
    draft.reflow();
    draft
}

/// Shows `tool` over a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, tool: &mut PenTool) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let shown = Canvas::new("Document").view(view).show(ui, |_, _| {});
    let _ = ToolHost::new().run(ui, &shown.output, tool);
    StoryOutput::default()
}

#[story(PenTool, covers = [Default], drive = Drive::Never)]
fn pen_draft_with_a_pulled_handle(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut tool = PenTool::new();
    tool.draft(Some(&pen_draft()));
    on_canvas(ui, &mut tool)
}

#[story(PenTool, covers = [], drive = Drive::Never)]
fn curvature_draft_through_four_anchors(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut tool = PenTool::new();
    tool.mode(PenMode::Curvature).draft(Some(&curvature_draft()));
    on_canvas(ui, &mut tool)
}

/// The colour the popover's swatch shows while the settings hold none.
fn line_color() -> Srgba {
    Srgba::rgb(0.0, 0.5, 1.0)
}

#[story(PathOptionsPopover<'static>, covers = [Default], drive = Drive::Never)]
fn path_options_defaults(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut open = *data.value("open", true);
    let settings = data.value("defaults", PathOptionsSettings::default());
    let shown = PathOptionsPopover::new("Path Options", Bind::bound(&mut *settings)).default_color(line_color()).show(ui, &mut open);
    *data.value("open", true) = open;
    shown.into()
}

#[story(PathOptionsPopover<'static>, covers = [], drive = Drive::Never)]
fn path_options_with_a_color(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut open = *data.value("open-set", true);
    let settings = data.value("set", PathOptionsSettings { rubber_band: true, color: Some([200, 40, 40]), thickness: Some(2.0) });
    let shown = PathOptionsPopover::new("Path Options", Bind::bound(&mut *settings)).show(ui, &mut open);
    *data.value("open-set", true) = open;
    shown.into()
}
