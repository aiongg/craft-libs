//! Stories of the Liquify workspace (task ST-WS1): stubs, not registered. Registering them (task
//! RG2) adds `liquify::stories` to the crate's `register!` list and `LiquifyWorkspace` to
//! `COMPONENTS`, and regenerates the registry dump; until then the gallery would run the `todo!()`
//! bodies.
//!
//! The stories run the workspace over `MockImage` (`ui::mock_image`) through a fake
//! `LiquifySource` that ignores the strokes (the field is the app's), a fake freeze mask and a
//! gently displaced mesh. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/liquify/tools` | The 13 tools in their groups, a stroke in progress with the brush outline, the Alt + right-drag resize with its readout |
//! | `dialogs/liquify/view-options` | The mesh at the three sizes, the mask overlay, the backdrop in the three modes |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
