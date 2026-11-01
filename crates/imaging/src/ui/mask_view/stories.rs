//! Stories of the mask views (task ST-SE2): stubs, not registered. Registering them (task RG2)
//! adds `mask_view::stories` to the crate's `register!` list and `MaskViewControls` to
//! `COMPONENTS`, and regenerates the registry dump; until then the gallery would run the `todo!()`
//! bodies.
//!
//! The story composites the recipe itself over `MockImage` with `MockImage::subject` as the soft
//! mask (the house), so every view shows without an engine. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/mask-view/modes` | The seven views over the picture; the opacity kept per view; Show Edge; Show Original; hold X |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
