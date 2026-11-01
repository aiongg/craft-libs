//! [`BlurPinsPanel`]: the selected pin's rows in the Blur Gallery dialog (task WS-BP2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_ui::edit::EditEvent;

use super::BlurPinsLabels;
use super::model::BlurPins;

/// What [`BlurPinsPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BlurPinsPanelOutput {
    pub response: Response,
    /// The amount slider's gesture, with the pins after it.
    pub edit: Option<EditEvent<BlurPins>>,
}

/// The rows the Blur Gallery's filter dialog shows for its pins (in the dialog's body, above its
/// Preview check box): the selected pin's Blur (0..=500 px), or for Spin Blur Angle (0..=360°), a
/// `SliderRow`; "No pin selected" in `text.muted` without one; then the hints of the kind in the
/// caption role, `text.muted`, wrapped.
#[expect(dead_code, reason = "skeleton: `show` (task WS-BP2) reads the fields")]
pub struct BlurPinsPanel<'a> {
    id_salt: IdSalt,
    pins: &'a mut BlurPins,
    labels: &'a BlurPinsLabels,
}

impl<'a> BlurPinsPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, pins: &'a mut BlurPins, labels: &'a BlurPinsLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP2")]
    pub fn show(self, ui: &mut Ui) -> BlurPinsPanelOutput {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:398 `body` (the slider, the hints).
        todo!()
    }
}
