//! The background cells the bar and the Properties section share: White, Black, Transparent and
//! Other… with its colour request.

use super::{ArtboardBackground, ArtboardWords};
use craft_ui::runtime::requests::ColorRequest;
use egui::Ui;

/// What the background cells report this pass.
#[derive(Debug, Default)]
#[allow(dead_code)] // P3(tools-w18): tl.T56
pub(super) struct BackgroundPick {
    /// The background picked.
    pub picked: Option<ArtboardBackground>,
    /// The request for the app's colour picker (Other…).
    pub pick_color: Option<ColorRequest>,
}

/// Draws the cells for `background` and reports a pick.
#[allow(dead_code)] // P3(tools-w18): tl.T56
pub(super) fn background_cells(ui: &mut Ui, words: &ArtboardWords, background: ArtboardBackground) -> BackgroundPick {
    let _ = (ui, words, background);
    todo!("tl.T56")
}
