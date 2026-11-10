//! Stories of the frame preview.

use super::{FrameFit, FramePreview};
use craft_ui::contract::Show as _;
use craft_ui::primitives::TextureCache;
use craft_ui::testing::pictures::{TEST_CARD_SIZE, test_card};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{TextureId, Ui, Vec2, vec2};

/// The test card, uploaded once.
fn card(ui: &Ui) -> (TextureId, Vec2) {
    let [width, height] = TEST_CARD_SIZE;
    (TextureCache::texture(ui.ctx(), "timeline-preview-card", test_card), vec2(width as f32, height as f32))
}

fn shown(ui: &mut Ui, fit: FrameFit, status: &str) -> StoryOutput {
    let frame = Some(card(ui));
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
