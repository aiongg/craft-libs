//! Stories of the Content-Aware Fill workspace (task ST-WS2): stubs, not registered. Registering
//! them (task RG2) adds `fill::stories` to the crate's `register!` list and
//! `ContentAwareFillWorkspace` to `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories run the workspace over `MockImage` through a fake `FillSource`: a rectangular fill
//! area, a sampling plane painted from the strokes by the story itself, and a preview that is the
//! image shifted (no engine). Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/content-aware-fill/brush` | The sampling tint, the three sampling options, Indicates both ways |
//! | `dialogs/content-aware-fill/lasso` | The fill area's ants, the four operations, Expand and Contract |
//! | `dialogs/content-aware-fill/preview` | The Preview panel framed on the sampling window, recomputing, failed |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
