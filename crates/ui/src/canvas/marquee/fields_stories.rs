//! Stories of the marquee style cells (task KS1): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the canvas stories and the components to `COMPONENTS`, and regenerates
//! the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! The cells sit in an `OptionsBar` over a fake 2000 × 1500 px document at 300 ppi, so Fixed Size
//! shows its units menu with real conversions. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/marquee/constraint-fields` | Normal (fields greyed), Fixed Ratio 1 : 1 after the switch, Fixed Size 64 × 64 px, swap, a picked unit (in), the whole group disabled (single row marquee) |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
