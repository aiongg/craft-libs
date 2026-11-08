//! Stories of the frame preview.

use super::{FrameFit, FramePreview};
use craft_ui::contract::Show as _;
use craft_ui::primitives::TextureCache;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, ColorImage, TextureId, Ui, vec2};

/// The test card's size in pixels: 4:3.
const CARD: [usize; 2] = [16, 12];

/// A 4:3 card shaded across and down, uploaded once.
fn card(ui: &Ui) -> TextureId {
    TextureCache::texture(ui.ctx(), "timeline-preview-card", || {
        let mut pixels = Vec::with_capacity(CARD[0] * CARD[1]);
        for y in 0..CARD[1] {
            for x in 0..CARD[0] {
                let (across, down) = (x * usize::from(u8::MAX) / CARD[0], y * usize::from(u8::MAX) / CARD[1]);
                pixels.push(Color32::from_rgb(across as u8, down as u8, ((across + down) / 2) as u8));
            }
        }
        ColorImage::new(CARD, pixels)
    })
}

fn shown(ui: &mut Ui, fit: FrameFit, status: &str) -> StoryOutput {
    let frame = Some((card(ui), vec2(CARD[0] as f32, CARD[1] as f32)));
    FramePreview::new("Program frame", frame).fit(fit).status(status.to_owned()).show(ui).into()
}

#[story(FramePreview, covers = [Default], drive = Drive::Never)]
fn frame_preview_letterboxed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    shown(ui, FrameFit::Letterbox, "Test card \u{b7} 00:00:01:00")
}

#[story(FramePreview, covers = [], drive = Drive::Never)]
fn frame_preview_cropped(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    shown(ui, FrameFit::Crop([0.1, 0.1, 0.1, 0.1]), "Test card \u{b7} cropped")
}

#[story(FramePreview, covers = [Empty], drive = Drive::Never)]
fn frame_preview_offline(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FramePreview::new("Program frame", None).status("Offline").show(ui).into()
}
