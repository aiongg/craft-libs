//! Stories of the raster display.

use super::RasterDisplay;
use crate::canvas::{Canvas, PixelRect, RasterSource, ToolPath};
use crate::contract::{Show as _, ShowContainer as _};
use crate::primitives::Revision;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, PixelSize};
use egui::{Color32, ColorImage, Ui};

/// The document's size in pixels.
const DOCUMENT: PixelSize = PixelSize::new(320, 200);

/// The document's centre, where the views look.
const CENTER: Point = Point::new(160.0, 100.0);

/// How far the zoomed story magnifies the document, so single pixels show sharp.
const MAGNIFIED: f64 = 4.0;

/// The turn of the turned story, in degrees.
const TURN: f64 = 30.0;

/// A diagonal gradient with a transparent top left quarter, so the checkerboard shows through.
struct Gradient;

impl RasterSource for Gradient {
    fn size(&self) -> PixelSize {
        DOCUMENT
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [width, height] = out.size;
        let (full_width, full_height) = (DOCUMENT.width as f32, DOCUMENT.height as f32);
        for row in 0..height {
            for column in 0..width {
                let x = region.x0 as f32 + (column as f32 + 0.5) * factor as f32;
                let y = region.y0 as f32 + (row as f32 + 0.5) * factor as f32;
                let seen = x > full_width / 2.0 || y > full_height / 2.0;
                let level = ((x / full_width + y / full_height) * 127.0) as u8;
                out.pixels[row * width + column] = if seen { Color32::from_rgb(level, 160, u8::MAX - level) } else { Color32::TRANSPARENT };
            }
        }
    }
}

/// Shows the gradient on a canvas whose view is `view` of the canvas's own rect.
fn on_canvas(ui: &mut Ui, view: impl FnOnce(CanvasView) -> CanvasView) -> StoryOutput {
    static SOURCE: Gradient = Gradient;
    let level = view(CanvasView::new(ui.max_rect(), CENTER, 1.0));
    let shown = Canvas::new("Document", ToolPath::Freehand)
        .view(level)
        .show(ui, |ui, body| RasterDisplay::new("Picture", &SOURCE).checkerboard().show_at(ui, body.rect));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn document(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |view| view)
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn magnified(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |mut view| {
        view.zoom = MAGNIFIED;
        view
    })
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn turned(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |view| view.rotation(TURN))
}
