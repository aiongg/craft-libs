//! Stories of the selection tools (task ST-SE1): stubs, not registered. Registering them (task
//! RG2) adds `select::stories` to the crate's `register!` list and the tools to `COMPONENTS`, and
//! regenerates the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! Every story uses fakes: a document drawn by the story (a photo-like gradient with a bright disc),
//! a `SelectionProbe` over the story's own rectangle or disc selection, a fake `EdgeTracer` that
//! snaps to the disc's rim, and a `SelectionOverlay` over the story's mask so a commit shows. Every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/select/marquee` | The four kinds; Normal, Fixed Ratio and Fixed Size; a drag inside the selection; a click that deselects |
//! | `patterns/select/lasso` | Freehand, ⌥ straight segments, retract, the close rules |
//! | `patterns/select/polygon-lasso` | ⇧ 45°, close, retract, the replaced selection hidden |
//! | `patterns/select/magnetic-lasso` | The fake tracer on the disc; Width, Contrast and Frequency from the options; fastening points, the closing ring |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
