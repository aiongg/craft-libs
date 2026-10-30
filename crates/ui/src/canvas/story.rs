//! The gallery's story canvas: a checkerboard document under a [`CanvasView`] set by the story's
//! args, which every canvas overlay story draws on.

use egui::accesskit::Role;
use egui::{Color32, EventFilter, Id, Response, Sense, Shape, Stroke, Ui, WidgetInfo, vec2};
use kurbo::{Point, Rect};

use super::{CanvasGrid, CanvasScrollbars, CanvasView, Claim, GridLine, Readout, ScrollbarNames, crisp, hit};
use crate::behavior::nudge;
use crate::primitives::{Corner, FocusRing, Handle, HandleShape};
use crate::story::{self, Level, NoArgs, Story, StoryArgs, StoryChoice, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf};
use crate::typed_story;

/// The story document, in document units (pixels).
pub(crate) const DOC: Rect = Rect::new(0.0, 0.0, 640.0, 400.0);
/// Side of a checkerboard square.
const SQUARE: f64 = 40.0;
/// The `ppp` arg: the gallery's own scale, or a fixed number of pixels per point for the whole
/// gallery.
#[derive(StoryChoice, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum PppArg {
    #[default]
    Gallery,
    #[choice(name = "1")]
    One,
    #[choice(name = "1.25")]
    OneQuarter,
    #[choice(name = "1.5")]
    OneHalf,
    #[choice(name = "2")]
    Two,
}

impl PppArg {
    /// The fixed scale, or `None` for the gallery's own.
    fn pixels_per_point(self) -> Option<f32> {
        match self {
            PppArg::Gallery => None,
            PppArg::One => Some(1.0),
            PppArg::OneQuarter => Some(1.25),
            PppArg::OneHalf => Some(1.5),
            PppArg::Two => Some(2.0),
        }
    }
}

/// The story canvas's view args as values: `zoom` in percent of device pixels per document pixel
/// (PhotoCraft's 100 %), `rotation` in degrees, `flip`, pixel `aspect` and `ppp`. Every canvas
/// story's args struct flattens it as its first field (`#[arg(flatten)] view: ViewArgs`) and
/// passes `args.view` to [`document`].
#[derive(StoryArgs, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewArgs {
    #[arg(range = 10.0..=800.0)]
    pub zoom: f64,
    #[arg(range = -180.0..=180.0)]
    pub rotation: f64,
    pub flip: bool,
    #[arg(range = 0.5..=2.0)]
    pub aspect: f64,
    pub ppp: PppArg,
}

impl Default for ViewArgs {
    fn default() -> Self {
        Self { zoom: 100.0, rotation: 0.0, flip: false, aspect: 1.0, ppp: PppArg::default() }
    }
}

/// The args of a canvas story with no args of its own.
#[derive(StoryArgs, Debug, Default, PartialEq)]
pub(crate) struct PlainArgs {
    #[arg(flatten)]
    pub view: ViewArgs,
}

/// The story canvas drawn by [`document`]: its response (overlays sense through it) and view.
pub(crate) struct StoryCanvas {
    pub response: Response,
    pub view: CanvasView,
    /// The area allocated: the canvas and the margin around its top and left for rulers.
    pub full: egui::Rect,
}

/// Draws the story document filling the story's width, centred, under the view `view` sets,
/// and senses clicks, drags and keyboard focus on it. Overlays draw over it with `ui.painter_at
/// (canvas.view.rect)`.
pub(crate) fn document(ui: &mut Ui, view: ViewArgs, label: &str) -> StoryCanvas {
    document_in(ui, view, label, 0.0)
}

/// [`document`] with `margin` points left free above and left of the canvas, for rulers.
pub(crate) fn document_in(ui: &mut Ui, view: ViewArgs, label: &str, margin: f32) -> StoryCanvas {
    document_at(ui, view, label, margin, DOC.center())
}

/// [`document_in`] with the view centred on `center`.
pub(crate) fn document_at(ui: &mut Ui, view: ViewArgs, label: &str, margin: f32, center: Point) -> StoryCanvas {
    if let Some(ppp) = view.ppp.pixels_per_point() {
        story::request_pixels_per_point(ui.ctx(), ppp);
    }
    let size = vec2((ui.available_width() - margin).clamp(240.0, 720.0), 360.0);
    let sense = Sense::click_and_drag() | Sense::FOCUSABLE;
    let (full, rect, response) = if margin > 0.0 {
        let (full, _) = ui.allocate_exact_size(size + vec2(margin, margin), Sense::hover());
        let rect = egui::Rect::from_min_max(full.min + vec2(margin, margin), full.max);
        (full, rect, ui.interact(rect, ui.scope_id().with(("craft_ui::canvas::story", label)), sense))
    } else {
        let (rect, response) = ui.allocate_exact_size(size, sense);
        (rect, rect, response)
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Canvas, ui.is_enabled(), label));
    // The arrow keys nudge on the canvas instead of moving focus.
    let arrows = EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(response.id, arrows));
    let ppp = f64::from(ui.pixels_per_point());
    let view = CanvasView { rect, center, zoom: view.zoom / 100.0 / ppp, rotation: view.rotation, flip_x: view.flip, aspect: view.aspect };
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, tokens.color.surface.canvas.to_egui());
    painter.add(Shape::convex_polygon(view.quad(DOC).to_vec(), Color32::WHITE, Stroke::NONE));
    let (columns, rows) = ((DOC.width() / SQUARE).ceil() as usize, (DOC.height() / SQUARE).ceil() as usize);
    for row in 0..rows {
        for column in (row % 2..columns).step_by(2) {
            let (x, y) = (DOC.x0 + column as f64 * SQUARE, DOC.y0 + row as f64 * SQUARE);
            let square = Rect::new(x, y, (x + SQUARE).min(DOC.x1), (y + SQUARE).min(DOC.y1));
            painter.add(Shape::convex_polygon(view.quad(square).to_vec(), Color32::from_gray(204), Stroke::NONE));
        }
    }
    FocusRing::paint(ui, rect, Corner::Control, &response);
    StoryCanvas { response, view, full }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Canvas", "Grid", "Readout"];

#[derive(StoryArgs, Debug, PartialEq)]
struct ScrollbarsArgs {
    #[arg(flatten)]
    view: ViewArgs,
    overscroll: bool,
}

impl Default for ScrollbarsArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), overscroll: true }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum GridStyleArg {
    Solid,
    Dashed,
    Dots,
    Crossings,
}

impl GridStyleArg {
    fn line(self) -> GridLine {
        match self {
            GridStyleArg::Solid => GridLine::Solid,
            GridStyleArg::Dashed => GridLine::Dashed,
            GridStyleArg::Dots => GridLine::Dots,
            GridStyleArg::Crossings => GridLine::Crossings,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct GridArgs {
    #[arg(flatten)]
    view: ViewArgs,
    style: GridStyleArg,
    #[arg(range = 1.0..=10.0)]
    subdivisions: u32,
    #[arg(range = 10.0..=200.0)]
    spacing: f64,
}

impl Default for GridArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), style: GridStyleArg::Solid, subdivisions: 4, spacing: 80.0 }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/grid/lines", Level::Primitives, "Grid", "Lines", GridArgs::default(), show_grid).on(Surface::Canvas),
        typed_story!("primitives/canvas/document", Level::Primitives, "Canvas", "Document", PlainArgs::default(), show_document).on(Surface::Canvas),
        typed_story!("primitives/canvas/scrollbars", Level::Primitives, "Canvas", "Scrollbars", ScrollbarsArgs::default(), show_scrollbars)
            .on(Surface::Canvas),
        typed_story!("primitives/readout/values", Level::Primitives, "Readout", "Values", NoArgs {}, show_readout).on(Surface::Canvas),
    ]
}

/// A point on the document: drag it (it claims the press), nudge it with the arrow keys while the
/// canvas has focus; crisp lines along the document axes through it; the pointer's document
/// position in a readout.
fn show_document(ui: &mut Ui, args: &mut PlainArgs) {
    let canvas = document(ui, args.view, "Document");
    let (response, view) = (&canvas.response, &canvas.view);
    let tokens = Tokens::of(ui);
    let key = ui.scope_id().with("canvas-story-point");
    let mut point: Point = ui.data(|d| d.get_temp(key)).unwrap_or(Point::new(200.0, 120.0));
    let marker = Id::unique("canvas-story-marker");
    let ctx = ui.ctx().clone();
    if response.is_pointer_button_down_on()
        && let Some(press) = ctx.input(|i| i.pointer.press_origin())
        && response.drag_started()
        && hit::point(press, view.to_screen(point)) <= tokens.component.handle.hit / 2.0
    {
        Claim::take(&ctx, response.id, marker);
    }
    if Claim::is_owner(&ctx, response.id, marker)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        point = view.to_doc(pointer);
    }
    if response.has_focus()
        && let Some(step) = nudge::read(ui, nudge::STEP, nudge::BIG_STEP)
    {
        point += step;
    }
    ui.data_mut(|d| d.insert_temp(key, point));

    let painter = ui.painter_at(view.rect);
    let line = Stroke::new(tokens.component.guide.width, tokens.color.overlay.guide.to_egui());
    let visible = view.visible();
    crisp(&painter, view.to_screen(Point::new(visible.x0, point.y)), view.to_screen(Point::new(visible.x1, point.y)), line);
    crisp(&painter, view.to_screen(Point::new(point.x, visible.y0)), view.to_screen(Point::new(point.x, visible.y1)), line);
    painter.extend(Handle::shapes(&tokens, HandleShape::Round, view.to_screen(point), ui.pixels_per_point()));
    if let Some(pointer) = response.hover_pos().or_else(|| response.interact_pointer_pos()) {
        let at = view.to_doc(pointer);
        Readout::new("canvas-story-readout").row("X:", format!("{:.1}", at.x)).row("Y:", format!("{:.1}", at.y)).show(&ctx, pointer);
    }
}

/// The grid over the document, inside the document rect.
fn show_grid(ui: &mut Ui, args: &mut GridArgs) {
    let canvas = document(ui, args.view, "Document");
    CanvasGrid::new("canvas-story-grid")
        .spacing(kurbo::Vec2::new(args.spacing, args.spacing))
        .subdivisions(args.subdivisions)
        .bounds(DOC)
        .line(args.style.line())
        .show(ui, &canvas.view);
}

/// The document under its scroll bars: drag a thumb, click a track to page, change the zoom to
/// see the thumb follow. With overscroll off the view stays on the document.
fn show_scrollbars(ui: &mut Ui, args: &mut ScrollbarsArgs) {
    let key = ui.scope_id().with("canvas-story-scroll");
    let center: Point = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| DOC.center());
    let canvas = document_at(ui, args.view, "Document", 0.0, center);
    let mut view = canvas.view;
    let overscroll = args.overscroll;
    if !overscroll {
        CanvasScrollbars::clamp(&mut view, DOC);
    }
    let names = ScrollbarNames::new("Horizontal scroll bar", "Vertical scroll bar");
    let _ = CanvasScrollbars::new("canvas-story-scrollbars", names).overscroll(overscroll).show(ui, &mut view, DOC);
    ui.data_mut(|d| d.insert_temp(key, view.center));
}

/// A readout of a transform's size and angle.
fn show_readout(ui: &mut Ui, _: &mut NoArgs) {
    let (rect, _) = ui.allocate_exact_size(vec2(200.0, 80.0), Sense::hover());
    Readout::new(ui.scope_id().with("readout-story")).row("W:", "120 px").row("H:", "80 px").row("Angle:", "15.0°").show(ui.ctx(), rect.min);
}
