//! The gallery's story canvas: a checkerboard document under a [`CanvasView`] set by the story's
//! args, which every canvas overlay story draws on.

use egui::accesskit::Role;
use egui::{Color32, EventFilter, Id, Response, Sense, Shape, Stroke, Ui, WidgetInfo, vec2};
use kurbo::{Point, Rect};

use super::{CanvasView, Claim, Readout, crisp, hit};
use crate::behavior::nudge;
use crate::primitives::{Corner, FocusRing, Handle, HandleShape};
use crate::story::{self, Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

/// The story document, in document units (pixels).
pub(crate) const DOC: Rect = Rect::new(0.0, 0.0, 640.0, 400.0);
/// Side of a checkerboard square.
const SQUARE: f64 = 40.0;
/// The `ppp` arg: the gallery's own scale, or a fixed number of pixels per point.
const PPP: &[&str] = &["gallery", "1", "1.25", "1.5", "2"];

/// The story canvas's args: `zoom` in percent of device pixels per document pixel (PhotoCraft's
/// 100 %), `rotation` in degrees, `flip`, pixel `aspect` and `ppp`.
pub(crate) fn args() -> Args {
    Args::new()
        .with_number("zoom", 100.0, 10.0, 800.0)
        .with_number("rotation", 0.0, -180.0, 180.0)
        .with_bool("flip", false)
        .with_number("aspect", 1.0, 0.5, 2.0)
        .with_choice("ppp", PPP, 0)
}

/// The story canvas drawn by [`document`]: its response (overlays sense through it) and view.
pub(crate) struct StoryCanvas {
    pub response: Response,
    pub view: CanvasView,
    /// The area allocated: the canvas and the margin around its top and left for rulers.
    pub full: egui::Rect,
}

/// Draws the story document filling the story's width, centred, under the view the args set,
/// and senses clicks, drags and keyboard focus on it. Overlays draw over it with `ui.painter_at
/// (canvas.view.rect)`.
pub(crate) fn document(ui: &mut Ui, args: &Args, label: &str) -> StoryCanvas {
    document_in(ui, args, label, 0.0)
}

/// [`document`] with `margin` points left free above and left of the canvas, for rulers.
pub(crate) fn document_in(ui: &mut Ui, args: &Args, label: &str, margin: f32) -> StoryCanvas {
    if let Ok(ppp) = args.choice("ppp").parse::<f32>() {
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
    let view = CanvasView {
        rect,
        center: DOC.center(),
        zoom: args.number("zoom") / 100.0 / ppp,
        rotation: args.number("rotation"),
        flip_x: args.bool("flip"),
        aspect: args.number("aspect"),
    };
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
pub const COMPONENTS: &[&str] = &["Canvas", "Readout"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story { id: "primitives/canvas/document", level: Level::Primitives, component: "Canvas", name: "Document", args, show: show_document },
        Story {
            id: "primitives/readout/values",
            level: Level::Primitives,
            component: "Readout",
            name: "Values",
            args: Args::new,
            show: show_readout,
        },
    ]
}

/// A point on the document: drag it (it claims the press), nudge it with the arrow keys while the
/// canvas has focus; crisp lines along the document axes through it; the pointer's document
/// position in a readout.
fn show_document(ui: &mut Ui, args: &mut Args) {
    let canvas = document(ui, args, "Document");
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

/// A readout of a transform's size and angle.
fn show_readout(ui: &mut Ui, _: &mut Args) {
    let (rect, _) = ui.allocate_exact_size(vec2(200.0, 80.0), Sense::hover());
    Readout::new(ui.scope_id().with("readout-story")).row("W:", "120 px").row("H:", "80 px").row("Angle:", "15.0°").show(ui.ctx(), rect.min);
}
