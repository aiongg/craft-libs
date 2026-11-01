//! Stories of the Blur Gallery pins (task ST-WS2): stubs, not registered. Registering them (task
//! RG2) adds `blur_pins::stories` to the crate's `register!` list and `BlurPinsOverlay` to
//! `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories draw `MockImage` on a canvas with the overlay and the panel beside it; no blur is
//! computed. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/blur-pins/field` | Three Field Blur pins, one selected, its amount on the ring |
//! | `canvas/blur-pins/iris` | An Iris pin with its outline, handles, rounding square and feather dots |
//! | `canvas/blur-pins/spin` | A Spin pin; the ring sets the blur angle |
//! | `canvas/blur-pins/tilt` | Tilt-Shift's band with its solid and dashed lines and turn dots |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
