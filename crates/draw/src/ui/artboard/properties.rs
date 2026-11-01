//! [`ArtboardProperties`]: an artboard's section in the Properties panel (task SH-T8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};
use kurbo::Rect;

use super::{ArtboardBackground, ArtboardItem, ArtboardLabels, ArtboardPreset};
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;

/// What [`ArtboardProperties::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies the changes and answers the colour request"]
pub struct ArtboardPropertiesOutput {
    /// The board's new rect from the preset, W, H, X or Y (`Set`, or a caption scrub's gesture).
    pub rect: Option<EditEvent<Rect>>,
    /// A background picked from the dropdown (White, Black, Transparent, or Other… once its colour
    /// came back).
    pub background: Option<ArtboardBackground>,
    /// Other… was chosen (or its swatch clicked): the app opens its colour picker with this request
    /// (titled "Artboard Background Color") and sets `Custom(colour)` from the answer.
    pub color_request: Option<ColorRequest>,
}

/// The Artboard section: Size preset, W and H (1–300,000 px), X and Y (±300,000 px), Background
/// (White, Black, Transparent, Other… with a swatch of the custom colour). A collapsible
/// `SectionHeader` titled "Artboard"; every field a Tab stop in reading order; the grid mirrors
/// right to left.
pub struct ArtboardProperties<'a> {
    id_salt: IdSalt,
    board: &'a ArtboardItem,
    presets: &'a [ArtboardPreset],
    labels: Option<ArtboardLabels>,
}

impl<'a> ArtboardProperties<'a> {
    /// The section of `board`, with the app's `presets`.
    pub fn new(id_salt: impl AsIdSalt, board: &'a ArtboardItem, presets: &'a [ArtboardPreset]) -> Self {
        ArtboardProperties { id_salt: IdSalt::new(id_salt), board, presets, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ArtboardLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ArtboardPropertiesOutput {
        // pc: crates/ui-egui/src/canvas/artboard_ui.rs:130-205 `properties` (Artboard :137, X :166,
        // Y :167, the background dropdown :174-184, Other… :182).
        // new: `SectionHeader`, `NumberField`, `Select`, `Swatch`; `ColorRequest` as `NewGuideDialog`.
        todo!()
    }
}
