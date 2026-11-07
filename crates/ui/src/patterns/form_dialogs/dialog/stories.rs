//! Stories of `ParamDialog`.

use super::{ParamDialog, ParamLayout};
use crate::contract::Bind;
use crate::patterns::form_dialogs::Param;
use crate::primitives::TextureCache;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::load::SizedTexture;
use egui::{Color32, ColorImage, Ui, Vec2};

const SPEC: &str = r#"{"radius":0.1..1000=1,"method":"spin|zoom","smooth":bool}"#;

/// The picture's size in points at zoom 1.
const PICTURE: [usize; 2] = [360, 240];

fn picture(ui: &Ui) -> SizedTexture {
    let id = TextureCache::texture(ui.ctx(), "form-dialogs-dialog-story", || ColorImage::filled(PICTURE, Color32::from_rgb(0x8a, 0x6f, 0x4d)));
    SizedTexture::new(id, Vec2::new(PICTURE[0] as f32, PICTURE[1] as f32))
}

#[story(ParamDialog<'static>, covers = [], drive = Drive::Never)]
fn param_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ParamDialog::new("blur", "Blur", Param::parse_list(SPEC)).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(ParamDialog<'static>, covers = [], drive = Drive::Never)]
fn param_dialog_with_preview(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let texture = picture(ui);
    ParamDialog::new("blur", "Blur", Param::parse_list(SPEC)).opened(Bind::bound(data.value("open", true))).preview(texture).show(ui).into()
}

#[story(ParamDialog<'static>, covers = [], drive = Drive::Never)]
fn param_dialog_with_preview_below(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let texture = picture(ui);
    ParamDialog::new("blur", "Blur", Param::parse_list(SPEC))
        .opened(Bind::bound(data.value("open", true)))
        .preview(texture)
        .layout(ParamLayout::Below)
        .show(ui)
        .into()
}
