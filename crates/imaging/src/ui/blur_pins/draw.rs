//! [`BlurPinsOverlay`]: the pins on the main canvas (task WS-BP2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_ui::canvas::CanvasView;
use craft_ui::edit::EditEvent;

use super::model::BlurPins;

/// What [`BlurPinsOverlay::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BlurPinsResponse {
    /// A pin added (`Set`), a drag (Begin, Update, Commit) or a pin removed (`Set`); the pins
    /// changed in place too.
    pub edit: Option<EditEvent<BlurPins>>,
    /// The overlay took the press: the canvas's own tool gets none.
    pub claimed: bool,
}

/// The Blur Gallery's pins over the main canvas while its filter dialog is open (lead call 48:
/// PhotoCraft's form, an overlay with a panel, no workspace).
///
/// - Pointer: a click on nothing adds a pin ([`BlurPins::add`]); a press grabs per
///   [`grab`](super::grab) and drags per [`drag`](super::drag); the cursor follows the grab
///   ([`BlurGrab::cursor`](super::BlurGrab::cursor)).
/// - Keyboard: the pins are parts of one `PartFocus` group ("Pin 3, Blur 15 px"); Page picks
///   (and selects), arrows move the pin one document pixel (Shift ten), Delete and Backspace
///   remove the selected pin unless a text field has focus.
/// - Drawing: each pin's dot (`blur_pins.dot` 5) and ring (`.ring` 24, the amount as an arc of
///   it); the selected pin's outline (Iris's superellipse, Spin's ellipse) with its handles
///   (`.handle` 4.5), the rounding square and the feather dots; Tilt-Shift's solid focus lines,
///   dashed transition lines and turn dots; all over the halo.
#[expect(dead_code, reason = "skeleton: `show` (task WS-BP2) reads the fields")]
pub struct BlurPinsOverlay<'a> {
    id_salt: IdSalt,
    pins: &'a mut BlurPins,
    labels: Option<&'a super::BlurPinsLabels>,
}

impl<'a> BlurPinsOverlay<'a> {
    pub fn new(id_salt: impl AsIdSalt, pins: &'a mut BlurPins) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, labels: None }
    }

    pub fn labels(mut self, labels: &'a super::BlurPinsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Runs and draws the overlay over `canvas` (the main canvas's response) through `view`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP2")]
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> BlurPinsResponse {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:533 `canvas`, :689 `draw`, :675 `draw_line`.
        todo!()
    }
}
