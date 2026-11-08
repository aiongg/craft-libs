//! Stories of the frame preview.

use super::{FrameFit, FramePreview};
use craft_ui::contract::Show as _;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, ColorImage, Context, Id, TextureHandle, TextureOptions, Ui, Vec2, vec2};

/// The test card's size in pixels: 4:3.
const CARD: [usize; 2] = [16, 12];
/// The box the story shows the frame in: 16:9.
const BOX: Vec2 = vec2(320.0, 180.0);

/// A 4:3 card of coloured stripes with a white frame, uploaded once.
fn card(ctx: &Context) -> TextureHandle {
    let id = Id::new("timeline-preview-card");
    if let Some(handle) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return handle;
    }
    let stripes = [Color32::RED, Color32::GREEN, Color32::BLUE, Color32::YELLOW];
    let mut bytes = Vec::with_capacity(CARD[0] * CARD[1] * 4);
    for y in 0..CARD[1] {
        for x in 0..CARD[0] {
            let edge = x == 0 || y == 0 || x == CARD[0] - 1 || y == CARD[1] - 1;
            let color = if edge { Color32::WHITE } else { stripes[x * stripes.len() / CARD[0]] };
            bytes.extend_from_slice(&color.to_array());
        }
    }
    let handle = ctx.load_texture("timeline-preview-card", ColorImage::from_rgba_unmultiplied(CARD, &bytes), TextureOptions::NEAREST);
    ctx.data_mut(|data| data.insert_temp(id, handle.clone()));
    handle
}

fn shown(ui: &mut Ui, fit: FrameFit, status: &str) -> StoryOutput {
    let texture = card(ui.ctx());
    let frame = Some((texture.id(), vec2(CARD[0] as f32, CARD[1] as f32)));
    ui.allocate_ui(BOX, |ui| FramePreview::new("Program frame", frame).fit(fit).status(status).show(ui)).inner.into()
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
    ui.allocate_ui(BOX, |ui| FramePreview::new("Program frame", None).status("Offline").show(ui)).inner.into()
}
