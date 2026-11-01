//! Stories of the Crop options bar and its menus (task ST-TO1): stubs, not registered (task RG2
//! registers them).
//!
//! Every story fills an `OptionsBar` with `CropOptionsBar` over settings kept in the story's state,
//! with a fake document and two fake user presets, and shows the bar's events in a status line.
//! Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/crop-options/bar` | The whole bar in a narrow and a wide width, with saved presets and the New and Delete entries |
//! | `patterns/crop-options/overlay-menu` | The overlay menu with its shortcuts |
//! | `patterns/crop-options/gear-menu` | The gear popover, Classic Mode greying two rows, the Custom colour request |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
