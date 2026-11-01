//! Stories of the shape panel and options bar (task ST-SH1): stubs, not registered. Registering
//! them (task RG2) adds `stories()` to craft-draw's stories and the components to `COMPONENTS`.
//!
//! Every story uses a fake shape layer (a `LiveShape` and a `ShapeAppearance`); every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/shape-panel` | Appearance and Shape for Rect, Ellipse, Polygon, Star and Line; a locked layer |
//! | `draw/shape-options-bar` | The bar for each Shape tool kind |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
