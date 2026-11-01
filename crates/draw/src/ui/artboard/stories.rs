//! Stories of the Artboard tool, bar and Properties section (task ST-SH1): stubs, not registered.
//! Registering them (task RG2) adds `stories()` to craft-draw's stories and the components to
//! `COMPONENTS`.
//!
//! Every story uses fake boards on a `Pasteboard`; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/artboard-tool` | Draw, move, resize, "+", Photoshop and Illustrator rules, a locked board |
//! | `draw/artboard-bar` | With and without a selection; a preset list |
//! | `draw/artboard-properties` | Preset, W/H, X/Y, background |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
