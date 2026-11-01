//! Stories of the transform preview (task ST-TR2): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the imaging stories and the component to `COMPONENTS`, and
//! regenerates the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! A checked picture with a tinted fake mip pyramid (each level its own tint, so the level in use
//! shows); every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/transform-preview/maps` | The picture through a quad (rotate, skew, distort past the horizon, perspective), a warp mesh, a pin mesh (fake solver) and a function map; Bicubic and Nearest at three zooms; opacity; clip to a canvas; a styled variant with a fake `StyledSource` (a drop shadow drawn by the story) |

use craft_ui::story::Story;

/// The stories above.
#[expect(dead_code, clippy::todo, reason = "skeleton: registered and filled in tasks ST-TR2 and RG2")]
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
