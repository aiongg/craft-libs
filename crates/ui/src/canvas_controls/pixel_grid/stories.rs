//! Stories of `PixelGrid`.

use super::PixelGrid;
use crate::canvas::{PixelRect, RasterSource};
use crate::canvas_controls::marquee::stories::on_canvas;
use crate::contract::Show as _;
use crate::primitives::Revision;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::PixelSize;
use egui::{Color32, ColorImage, Ui};

/// The device zoom the story shows the document at: above the grid's threshold.
const ZOOM: f64 = 8.0;

const SIDE: u32 = 16;

/// A document of two grey halves.
struct Halves;

impl RasterSource for Halves {
    fn size(&self) -> PixelSize {
        PixelSize::new(SIDE, SIDE)
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        Some(PixelRect::default())
    }

    fn read(&self, region: PixelRect, _: u32, out: &mut ColorImage) {
        let width = region.width() as usize;
        for (at, pixel) in out.pixels.iter_mut().enumerate() {
            let x = region.x0 + (at % width.max(1)) as i32;
            *pixel = if x < (SIDE / 2) as i32 { Color32::from_gray(30) } else { Color32::from_gray(225) };
        }
    }
}

#[story(PixelGrid<'static>, covers = [Default], drive = Drive::Never)]
fn magnified(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, ZOOM, |ui, at| PixelGrid::new("Pixel grid", &Halves).show_at(ui, at))
}
