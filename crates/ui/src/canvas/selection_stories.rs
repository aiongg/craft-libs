//! Stories of the selection frame, outlines, grips and range helpers (task KS1): stubs, not
//! registered. Registering them (task RG2) adds `stories()` to the canvas stories and the
//! components to `COMPONENTS`, and regenerates the registry dump; until then the gallery would run
//! the `todo!()` bodies.
//!
//! Every story draws its own fake objects on the canvas and keeps the edited boxes in the story's
//! state, so the frame and the grips can be dragged and nudged. Every string goes through story
//! words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/selection/frame` | One object, several (the union box), the key object, a rotated view, a content frame in brown with the item frame's grips off, grow 1–7, a rule that refuses the edges |
//! | `canvas/selection/grips` | Square, midpoint, arrow and triangle grips; cold, warm under the pointer, hot after a ⇧-click |
//! | `canvas/selection/outline` | Solid and dashed outlines over a list of shapes in two layer colours |
//! | `canvas/selection/range-bands` | A range with the active cell cut out; a 20-million-point column scrolled to its middle |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
