//! Stories of the paint kinds and the paint list's row actions (task ST-SH2): stubs, not
//! registered. Registering them (task RG2) adds `stories()` to the colour stories.
//!
//! Every story uses fake pattern and picture previews and fake paints; every string goes through
//! story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `color/paint-chip-kinds` | A chip with five tabs (pattern and picture editors from fakes) |
//! | `color/paint-list` | Rows with hex, opacity, a bound style, remove (`PaintListOptions::SELECTION_COLORS`) |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
