//! The Blur Gallery's pins (Filter › Blur Gallery › Field Blur, Iris Blur, Spin Blur,
//! Tilt-Shift): an overlay on the main canvas and the rows of the filter dialog.
//!
//! As PhotoCraft (lead call 48), the Blur Gallery is the ordinary filter dialog: no full-window
//! frame. [`BlurPins`] is the typed state (PhotoCraft kept a JSON map in the dialog's fields);
//! [`BlurPinsOverlay`] draws and edits the pins over the main canvas while the dialog is open, and
//! [`BlurPinsPanel`] adds the selected pin's amount and the hints to the dialog's body. The app
//! maps the pins to the filter's parameters and scales them for its proxy preview.
//!
//! ```ignore
//! let out = BlurPinsOverlay::new("pins", &mut app.blur_pins).show(ui, &canvas, &view);
//! if let Some(edit) = out.edit { app.blur_preview_stale(edit) }
//! BlurPinsPanel::new("pin-rows", &mut app.blur_pins).show(body_ui);
//! ```
//!
//! Files: `model` and `grab` (task WS-BP1); `draw` and `panel` (WS-BP2); `style` (TK4).

mod draw;
mod grab;
mod labels;
mod model;
#[cfg(test)]
mod model_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod overlay_tests;
mod panel;
pub(crate) mod stories;
mod style;

pub use draw::{BlurPinsOverlay, BlurPinsResponse};
pub use grab::{BlurGrab, drag, grab};
pub use labels::BlurPinsLabels;
pub use model::{BlurKind, BlurPin, BlurPins, DEFAULT_AMOUNT, IrisShape, MAX_PINS, TiltBand};
pub use panel::{BlurPinsPanel, BlurPinsPanelOutput};
pub use style::BlurPinsStyle;
