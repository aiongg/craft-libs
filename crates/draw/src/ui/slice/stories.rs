//! Stories of the Slice tools (task ST-SH1): stubs, not registered. Registering them (task RG2)
//! adds `stories()` to craft-draw's stories and the components to `COMPONENTS`.
//!
//! Every story uses fake slices (user, auto and layer slices on a fake document); every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/slice-tool` | Slice and Select modes, auto, user and layer slices, a lock |
//! | `draw/slice-bar` | Both modes |
//! | `draw/slice-options-dialog` | Image, No Image, HTML Text |
//! | `draw/divide-slices-dialog` | Count and size modes |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
