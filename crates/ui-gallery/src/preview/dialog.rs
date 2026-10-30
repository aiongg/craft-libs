//! The Image Size dialog.

use egui::Ui;

use super::model::Mock;

/// Opens the dialog's draft from the document.
pub fn open_image_size(mock: &mut Mock) {
    mock.open_image_size();
}

/// Draws the dialog while `mock.dialog` is set. Nothing is drawn yet.
pub fn show(_ui: &mut Ui, _mock: &mut Mock) {}
