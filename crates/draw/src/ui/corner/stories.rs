//! Stories of the corner widgets (task ST-PE1): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the draw stories and the components to `COMPONENTS`, and regenerates
//! the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! The story holds fake targets and applies the edits to them; every string goes through story
//! words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/corner-widgets/edit` | A fake rectangle, a star and a path's corner anchors; args: kinds, hide-above angle, individual |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the stories land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
