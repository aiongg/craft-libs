//! Stories of painting symmetry (task ST-TO2): stubs, not registered (task RG2 registers them).
//!
//! Every story draws a fake page (`craft_ui::testing::canvas_story`) and keeps the symmetry in the
//! story's state. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/symmetry/modes` | The four modes at the page centre and off-centre, clipped to the page |
//! | `primitives/symmetry/transform` | Transform Symmetry through `ToolHost`: move the centre, turn with ⇧ steps, Enter and Esc, keyboard parts |
//! | `patterns/symmetry-menu/menu` | The menu with paths, Off, Transform disabled and enabled |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
