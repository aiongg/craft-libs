//! Stories of the export dialogs.

use super::{
    ExportAsDialog, ExportOptionsKind, ExportPreview, FormatOptions, FormatOptionsDialog, JpegOptions, OutputPathRow, SizeEstimate, TargaOptions,
};
use crate::contract::Bind;
use crate::patterns::form_dialogs::PreviewState;
use crate::primitives::TextureCache;
use crate::runtime::env::Label;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::PixelSize;
use egui::load::SizedTexture;
use egui::{Color32, ColorImage, Ui, Vec2};

/// The document's size in pixels.
const DOCUMENT: PixelSize = PixelSize::new(1200, 800);
/// The preview picture's size in points at zoom 1.
const PICTURE: [usize; 2] = [360, 240];
/// The size estimate of the preview, in bytes.
const ESTIMATE: u64 = 1_258_291;

fn picture(ui: &Ui) -> SizedTexture {
    let id = TextureCache::texture(ui.ctx(), "form-dialogs-export-story", || ColorImage::filled(PICTURE, Color32::from_rgb(0x6f, 0x8a, 0x4d)));
    SizedTexture::new(id, Vec2::new(PICTURE[0] as f32, PICTURE[1] as f32))
}

#[story(ExportAsDialog<'static>, covers = [], drive = Drive::Never)]
fn export_as(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let preview = ExportPreview::new(1, PreviewState::Ready(picture(ui))).estimate(SizeEstimate::Proxy(ESTIMATE));
    ExportAsDialog::new("Poster", DOCUMENT, Bind::kept()).opened(Bind::bound(data.value("open", true))).preview(preview).show(ui).into()
}

#[story(ExportAsDialog<'static>, covers = [], drive = Drive::Never)]
fn export_as_rendering(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    ExportAsDialog::new("Poster", DOCUMENT, Bind::kept()).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(FormatOptionsDialog<'static>, covers = [], drive = Drive::Never)]
fn jpeg_options(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    FormatOptionsDialog::new(ExportOptionsKind::Jpeg)
        .opened(Bind::bound(data.value("open", true)))
        .initial(FormatOptions::Jpeg(JpegOptions::new(6)))
        .show(ui)
        .into()
}

#[story(FormatOptionsDialog<'static>, covers = [], drive = Drive::Never)]
fn tiff_options(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    FormatOptionsDialog::new(ExportOptionsKind::Tiff).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(FormatOptionsDialog<'static>, covers = [], drive = Drive::Never)]
fn targa_options(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    FormatOptionsDialog::new(ExportOptionsKind::Targa)
        .opened(Bind::bound(data.value("open", true)))
        .initial(FormatOptions::Targa(TargaOptions::new(32)))
        .show(ui)
        .into()
}

#[story(FormatOptionsDialog<'static>, covers = [], drive = Drive::Never)]
fn dds_options(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    FormatOptionsDialog::new(ExportOptionsKind::Dds).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(OutputPathRow<'static>, covers = [], drive = Drive::Never)]
fn output_path_row(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let templates = vec![
        ("[Comp Name].[fileExtension]".to_owned(), Label::from("Comp name")),
        ("[Comp Name]_[#####].[fileExtension]".to_owned(), Label::from("Comp name and frame")),
    ];
    OutputPathRow::new(Bind::bound(data.value("path", "[Comp Name].[fileExtension]".to_owned()))).templates(templates).show(ui).into()
}
