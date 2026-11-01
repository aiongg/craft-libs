//! Stories of the live preview (task ST-AD1): a stub, not registered. Registering it (task RG2)
//! adds `stories()` to the imaging stories and regenerates the registry dump; until then the
//! gallery would run the `todo!()` bodies.
//!
//! The document is a fake drawn by the story (a gradient with a target rect), so the damage shows
//! without a renderer. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `imaging/preview/live` | A slider changes the settings and so the key; the story outlines each switch's damage rect and the covered area, pans the view to show `uncovered`, and reads out the settle state, the proxy factor at the zoom, and a `DraftScale` drag's scale |

use craft_ui::story::Story;

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
