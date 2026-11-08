//! Stories of the full-window parts.

use super::{FullWindowCanvas, FullWindowSide, FullWindowStatus, FullWindowTitle, FullWindowView, FullWindowZoomBar, RasterOverlay, SideFooter};
use crate::canvas::{HandTool, NavigationOptions, PixelRect, RasterSource, ToolPath};
use crate::contract::{Bind, Show as _, ShowContainer as _};
use crate::display::Text;
use crate::primitives::{Checker, Revision};
use crate::runtime::pending::Pending;
use crate::sections::{DialogRole, FooterButton};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::PixelSize;
use egui::{Color32, ColorImage, Ui};

/// The story image's size in pixels.
const IMAGE: PixelSize = PixelSize::new(320, 200);

/// A solid image with a transparent left half, so the checkerboard shows.
struct Flat {
    colour: Color32,
    half: bool,
}

impl RasterSource for Flat {
    fn size(&self) -> PixelSize {
        IMAGE
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        Some(PixelRect::default())
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [width, height] = out.size;
        for row in 0..height {
            for column in 0..width {
                let x = region.x0 + ((column as u32 + 1) * factor) as i32 - 1;
                let transparent = self.half && x < (IMAGE.width / 2) as i32;
                out.pixels[row * width + column] = if transparent { Color32::TRANSPARENT } else { self.colour };
            }
        }
    }
}

#[story(FullWindowSide<'static>, covers = [Default], drive = Drive::Never)]
fn side(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let footer = SideFooter::bottom([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)]);
    FullWindowSide::new("Properties")
        .footer(footer)
        .show(ui, |ui, _| {
            Text::new("Radius").show(ui);
        })
        .output
        .into()
}

#[story(FullWindowSide<'static>, covers = [], drive = Drive::Never)]
fn side_with_top_footer(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let footer = SideFooter::top([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)]);
    FullWindowSide::new("Settings")
        .footer(footer)
        .show(ui, |ui, _| {
            Text::new("Effects").show(ui);
        })
        .output
        .into()
}

#[story(FullWindowZoomBar, covers = [], drive = Drive::Never)]
fn zoom_bar(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", FullWindowView::new());
    FullWindowZoomBar::new(Bind::bound(view)).note("Preview 120 ms").show(ui).into()
}

#[story(FullWindowZoomBar, covers = [], drive = Drive::Never)]
fn zoom_bar_with_progress(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", FullWindowView::new());
    FullWindowZoomBar::new(Bind::bound(view)).progress(Some(Some(0.4))).show(ui).into()
}

#[story(FullWindowTitle, covers = [], drive = Drive::Never)]
fn title(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FullWindowTitle::new("Liquify…").subject("Layer 1").zoom(66.7).show(ui).into()
}

#[story(FullWindowStatus, covers = [], drive = Drive::Never)]
fn status_loading(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let pending: Pending<(), String> = Pending::default();
    FullWindowStatus::of(&pending).show(ui).into()
}

#[story(FullWindowStatus, covers = [], drive = Drive::Never)]
fn status_failed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let pending: Pending<(), String> = Pending::Failed("The filter ran out of memory.".to_owned());
    FullWindowStatus::of(&pending).show(ui).into()
}

#[story(FullWindowCanvas<'static>, covers = [], drive = Drive::Never)]
fn canvas(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    static IMAGE_SOURCE: Flat = Flat { colour: Color32::from_rgb(0x4a, 0x7f, 0xc1), half: true };
    static MASK: Flat = Flat { colour: Color32::from_rgba_premultiplied(0x40, 0x00, 0x00, 0x40), half: false };
    let view = data.value("view", FullWindowView::new());
    let mut hand = HandTool::new(NavigationOptions::default());
    let shown = FullWindowCanvas::new(ToolPath::Freehand, view, &IMAGE_SOURCE)
        .checkerboard(Checker::new())
        .overlay(RasterOverlay::new(&MASK).tint(Color32::from_rgb(0xc0, 0x40, 0x40)))
        .tool(&mut hand)
        .show(ui);
    *view = shown.view.unwrap_or(shown.shown);
    shown.into()
}
