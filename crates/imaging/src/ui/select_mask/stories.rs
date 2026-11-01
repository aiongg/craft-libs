//! Stories of the Select and Mask workspace (task ST-WS2): stubs, not registered. Registering them
//! (task RG2) adds `select_mask::stories` to the crate's `register!` list and
//! `SelectMaskWorkspace` to `COMPONENTS`, and regenerates the registry dump; until then the
//! gallery would run the `todo!()` bodies.
//!
//! The stories run the workspace over `MockImage` through a fake `SelectMaskSource` that
//! composites the recipe over `MockImage::subject` (no engine), answering each ticket at once. Every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/select-mask/views` | The seven views with their opacities |
//! | `dialogs/select-mask/tools` | Each tool's options bar, a Polygonal Lasso in progress, Refine Hair and Object Aware disabled with their reasons |
//! | `dialogs/select-mask/refine` | Edge Detection and Global Refinements, Output To with Decontaminate Colors, the limit message |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
