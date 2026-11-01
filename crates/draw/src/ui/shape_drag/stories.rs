//! Stories of the Shape tools and Create Shape (task ST-SH1): stubs, not registered. Registering
//! them (task RG2) adds `stories()` to craft-draw's stories and the components to `COMPONENTS`.
//!
//! Every story uses a fake canvas view and fake ruler units; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/shape-drag` | An interactive canvas: every kind, both frames (Photoshop and Illustrator rules), ⇧ ⌥ Space, a click opening Create Shape |
//! | `draw/create-shape-dialog` | Rectangle with four radii, Polygon with a star ratio, the units menu, a bad size |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
