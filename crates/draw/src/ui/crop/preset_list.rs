//! [`CropPresetList`]: the crop preset list as one cell for the crop bar and the task bar.

use super::{CropDocument, CropRatioChange, CropSettings, CropUserPreset};
use craft_ui::contract::{NoValue, Outcome};
use egui::{Response, Ui};

/// The preset list: Free, Size, the ratios, the sizes, the user presets, Front Image, New, Delete,
/// and From Selection when [`selection_entry`](Self::selection_entry) is set. Picks go through
/// `CropSettings::choose`.
#[must_use = "show the list with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
pub struct CropPresetList<'a> {
    settings: &'a mut CropSettings,
    document: &'a CropDocument,
    /// The user's saved crop presets, oldest first.
    #[prop]
    user_presets: &'a [CropUserPreset],
    /// Offers From Selection.
    #[prop]
    selection_entry: bool,
}

/// What [`CropPresetList::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies what the list reports"]
pub struct CropPresetListOutput {
    /// The list.
    pub response: Response,
    /// The ratio changed.
    pub ratio: Option<CropRatioChange>,
    /// New Crop Preset… was picked.
    pub new_preset: bool,
    /// Delete Crop Preset… was picked.
    pub delete_preset: bool,
    /// From Selection was picked.
    pub selection: bool,
}

impl Outcome for CropPresetListOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl<'a> CropPresetList<'a> {
    /// The list editing `settings` over `document`.
    pub fn new(settings: &'a mut CropSettings, document: &'a CropDocument) -> Self {
        CropPresetList { settings, document, user_presets: &[], selection_entry: false }
    }

    /// Shows the list.
    pub fn show(self, ui: &mut Ui) -> CropPresetListOutput {
        let CropPresetList { settings, document, user_presets, selection_entry } = self;
        let _ = (ui, settings, document, user_presets, selection_entry);
        todo!("tl.T46")
    }
}
