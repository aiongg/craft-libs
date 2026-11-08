//! Stories of the raster display.

use super::RasterDisplay;
use super::RasterDisplayOutput;
use crate::canvas::{PixelRect, RasterSource};
use crate::contract::Show as _;
use crate::primitives::{Checker, Revision};
use crate::testing::canvas_story::on_canvas_view;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, PixelSize};
use egui::{Color32, ColorImage, Rect, Ui};

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

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        Some(PixelRect::default())
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

/// The view of the canvas over `room` that `adjust` makes of the one showing the whole document.
fn view_of(room: Rect, adjust: impl FnOnce(CanvasView) -> CanvasView) -> CanvasView {
    adjust(CanvasView::new(room, CENTER, 1.0))
}

/// The gradient, filling the canvas.
fn gradient(ui: &mut Ui, at: Rect, _: &CanvasView) -> RasterDisplayOutput {
    static SOURCE: Gradient = Gradient;
    RasterDisplay::new("Picture", &SOURCE).checkerboard(Checker::new()).show_at(ui, at)
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn document(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas_view(ui, |room| view_of(room, |view| view), gradient)
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn magnified(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let magnify = |mut view: CanvasView| {
        view.zoom = MAGNIFIED;
        view
    };
    on_canvas_view(ui, |room| view_of(room, magnify), gradient)
}

#[story(RasterDisplay<'static>, covers = [Default], drive = Drive::Never)]
fn turned(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas_view(ui, |room| view_of(room, |view| view.rotation(TURN)), gradient)
}
