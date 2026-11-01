//! Stories of Perspective Warp (task ST-TR2): stubs, not registered. Registering them (task RG2)
//! adds `stories()` to the imaging stories and the components to `COMPONENTS`, and regenerates the
//! registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! A picture with a fake map (a bilinear map of each plane); every string goes through story
//! words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/perspective-warp/planes` | Layout with two linked planes; Warp with the picture following; a picked corner with keyboard focus |
//! | `composites/perspective-warp/bar` | Layout (count "2 planes", Remove Planes) and Warp (Straighten, Level, Both) |

use craft_ui::story::Story;

/// The stories above.
#[expect(dead_code, clippy::todo, reason = "skeleton: registered and filled in tasks ST-TR2 and RG2")]
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
