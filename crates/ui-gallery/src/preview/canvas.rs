//! The canvas region: pasteboard, document, rulers and scrollbars.

use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::Ui;

use super::model::Mock;

/// Fills the region with the pasteboard colour.
pub fn show(ui: &mut Ui, _mock: &mut Mock) {
    let canvas = Tokens::of(ui).color.surface.canvas.to_egui();
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, canvas);
    ui.allocate_rect(rect, egui::Sense::hover());
}
