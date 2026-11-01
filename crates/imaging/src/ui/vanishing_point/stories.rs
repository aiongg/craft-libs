//! Stories of Vanishing Point (task ST-WS2): stubs, not registered. Registering them (task RG2)
//! adds `vanishing_point::stories` to the crate's `register!` list and `VanishingPointWorkspace`
//! to `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories use `MockScene`: a vanishing scene over `MockImage` with a valid, a questionable
//! and an invalid plane, an affine stamp ring, and no stamp painting (the preview is the image).
//! Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/vanishing-point/planes` | Valid, questionable and invalid planes, the selected plane's nodes, a torn-off wall, Create Plane's pending corners |
//! | `dialogs/vanishing-point/stamp` | The source cross, the foreshortened ring, the paste image on a plane |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
