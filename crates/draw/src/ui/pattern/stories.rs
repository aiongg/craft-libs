//! Stories of the pattern fill editor and dialog (task ST-SH2): stubs, not registered.
//! Registering them (task RG2) adds `stories()` to craft-draw's stories and the components to
//! `COMPONENTS`.
//!
//! Every story uses fake pattern swatches painted by a closure; every string goes through story
//! words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/pattern-editor` | Bitmap mode and two-colour preset mode |
//! | `draw/pattern-fill-dialog` | Live values and Snap to Origin |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
