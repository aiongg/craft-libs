//! Stories of the hover outline (task KS1): stubs, not registered. Registering them (task RG2)
//! adds `stories()` to the canvas stories and the components to `COMPONENTS`, and regenerates the
//! registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! Every story uses fakes: a document drawn by the story (shapes on a light and a dark image) and a
//! [`ManualJobs`](craft_host::ManualJobs) behind the lookup with a "Finish next" button, so each
//! lookup state shows one step at a time. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/hover/outline` | `HoverShape::Rect`, a `Path` in a layer colour and a `Mask` tint, on a light and a dark image |
//! | `canvas/hover/lookup` | A fake finder whose job takes 300 ms: debounce, pending, hit, miss, a cached hit inside a found object, cleared by a press, the empty-layer hint |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
