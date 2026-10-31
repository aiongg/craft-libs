//! The dialogs the mock app opens through the library's `DialogHost`: Gaussian Blur as a
//! `ParamDialog` and the layer delete question as a `StandardPrompt`.

use craft_ui::dialog::{DialogHost, DialogResult, ImageSizeDialog, Param, ParamDialog, PixelSize, PromptAnswer, PromptChoice, StandardPrompt};
use egui::Context;

use super::model::Mock;

const BLUR_SPEC: &str = r#"{"radius":0.1..250=4,"quality":"draft|good|best","monochromatic":bool}"#;

/// The host, cloned as an empty one: a copy of the app drawn in another context starts without
/// an open dialog.
#[derive(Debug, Default)]
pub struct Hosted(std::sync::Mutex<DialogHost>);

impl Hosted {
    /// The host. The mutex makes the value `Sync` for egui's memory, which a `DialogHost` no
    /// longer is on its own.
    pub fn host(&mut self) -> &mut DialogHost {
        self.0.get_mut().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Clone for Hosted {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// Opens Filter > Gaussian Blur.
pub fn open_blur(host: &mut DialogHost) {
    host.open(ParamDialog::new("gaussian-blur", "Gaussian Blur", Param::parse_list(BLUR_SPEC)).remember("filter.gaussianBlur").preview(true));
}

/// Opens Image Size on the document's size and resolution.
pub fn open_image_size(host: &mut DialogHost, mock: &Mock) {
    let [width, height] = mock.doc.size;
    host.open(ImageSizeDialog::new(PixelSize { width, height, pixels_per_inch: mock.doc.resolution }));
}

/// Opens the question for deleting the selected layers.
pub fn open_delete_layer(host: &mut DialogHost, mock: &Mock) {
    let names: Vec<String> = mock.doc.selected.iter().filter_map(|id| mock.doc.layer(*id)).map(|layer| layer.name.clone()).collect();
    if !names.is_empty() {
        let count = names.len();
        host.open(StandardPrompt::confirm_delete(names, count).noun("{n} layer", "{n} layers"));
    }
}

/// Draws the open dialogs and applies their answers to the app.
pub fn show(ctx: &Context, host: &mut DialogHost, mock: &mut Mock) {
    host.show(ctx);
    if let Some(DialogResult::Ok(done)) = host.take::<ParamDialog>() {
        let radius = done.fields.number("radius").unwrap_or_default();
        mock.status = format!("Gaussian Blur, radius {radius:.1} px");
    }
    if let Some(DialogResult::Ok(result)) = host.take::<ImageSizeDialog>() {
        mock.apply_size(&result);
    }
    if let Some(DialogResult::Ok(PromptAnswer { choice: PromptChoice::Proceed, .. })) = host.take::<StandardPrompt>() {
        mock.delete_selected();
    }
}
