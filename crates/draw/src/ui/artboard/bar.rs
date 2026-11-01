//! [`ArtboardBar`]: the Artboard tool's options bar cells (task SH-T8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};
use kurbo::Size;

use super::{ArtboardItem, ArtboardLabels, ArtboardPreset};
use craft_ui::edit::EditEvent;

/// What [`ArtboardBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app resizes the selected board or adds one"]
pub struct ArtboardBarOutput {
    /// The selected board's new size: a preset pick, Make Portrait or Make Landscape, or a typed W
    /// or H is `Set`; a caption scrub is a gesture. The app resizes the board (contents stay).
    pub size: Option<EditEvent<Size>>,
    /// "Add New Artboard": a board of the selected board's size, else the canvas's.
    pub add_new: bool,
}

/// The cells the Artboard tool adds to the options bar, in PhotoCraft's order: "Size" preset
/// dropdown (Custom, then the app's presets), W, H (1–300,000 px), Make Portrait (shown only when
/// the board is wider than tall), Make Landscape (only when taller than wide), "Add New Artboard".
/// Without a selected board the size cells are disabled. Each cell is a Tab stop in the bar's
/// order.
pub struct ArtboardBar<'a> {
    id_salt: IdSalt,
    selected: Option<&'a ArtboardItem>,
    presets: &'a [ArtboardPreset],
    labels: Option<ArtboardLabels>,
}

impl<'a> ArtboardBar<'a> {
    /// The bar for the `selected` board with the app's `presets`.
    pub fn new(id_salt: impl AsIdSalt, selected: Option<&'a ArtboardItem>, presets: &'a [ArtboardPreset]) -> Self {
        ArtboardBar { id_salt: IdSalt::new(id_salt), selected, presets, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ArtboardLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> ArtboardBarOutput {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:293-338 `options_bar` (Size :303, Custom :305,
        // W :311, H :313, Make Portrait :318, Make Landscape :321, Add New Artboard :331).
        todo!()
    }
}
