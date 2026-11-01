//! Stories of the Warp options-bar group (task ST-TR2): stubs, not registered. Registering them
//! (task RG2) adds `stories()` to the imaging stories and the component to `COMPONENTS`, and
//! regenerates the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! A fake style list; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/warp-fields/group` | The group with a preset and with Custom; an armed split; the type-layer variant (Split and Grid disabled, no Custom) |

use craft_ui::story::Story;

/// The stories above.
#[expect(dead_code, clippy::todo, reason = "skeleton: registered and filled in tasks ST-TR2 and RG2")]
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
