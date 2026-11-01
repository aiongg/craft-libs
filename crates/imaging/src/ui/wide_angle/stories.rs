//! Stories of the Adaptive Wide Angle workspace (task ST-WS1): stubs, not registered. Registering
//! them (task RG2) adds `wide_angle::stories` to the crate's `register!` list and
//! `WideAngleWorkspace` to `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories use `MockWideAngle`: a curved-line geometry (a barrel distortion of the image
//! centre) and a "corrected" preview that is `MockImage` itself. Every string goes through story
//! words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/wide-angle/constraints` | Free, Horizontal and Vertical constraints on the source, the same over the corrected preview, the constraint menu |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
