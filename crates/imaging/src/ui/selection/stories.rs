//! Stories of the selection overlay (task ST-SE2): stubs, not registered. Registering them (task
//! RG2) adds `selection::stories` to the crate's `register!` list and `SelectionOverlay` to
//! `COMPONENTS`, and regenerates the registry dump; until then the gallery would run the `todo!()`
//! bodies.
//!
//! Every story uses a fake `MaskSource` (a soft-edged disc, a 40 000 × 30 000 mask for the huge
//! case) over `mock_image`. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/selection/ants` | The disc at four zooms; the huge mask at 12 %; a floating offset; a mapped (turned) outline; hidden |
//! | `primitives/selection/preview` | The five preview modes; the Quick Mask colour, opacity and Color Indicates |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
