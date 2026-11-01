//! Stories of the before and after view (task KS1): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the canvas stories and the components to `COMPONENTS`, and regenerates
//! the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! The two pictures are fakes drawn by the story (a gradient and the same gradient toned), so
//! every mode shows without a renderer. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/before-after/modes` | The six modes over the two fakes, captions on and off, a right-to-left layout |
//! | `canvas/before-after/divider` | A Split and a SplitStacked view with the fraction readout, the keys, `drag_anywhere` on and off, hold Backslash for the original |

use crate::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
