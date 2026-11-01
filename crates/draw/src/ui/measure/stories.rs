//! Stories of the Ruler tool, its bar and the distances between boxes (task ST-TO2): stubs, not
//! registered (task RG2 registers them).
//!
//! Every story runs the tool through `ToolHost` on a fake page (`craft_ui::testing::canvas_story`)
//! and keeps the line in the story's state. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/ruler-tool/line` | Draw, grab an end, ⇧ 45°, clear on a tiny line, keyboard parts |
//! | `primitives/ruler-tool/protractor` | ⌥ from an end; the angle between the arms |
//! | `patterns/ruler-bar/readouts` | The bar with and without a line, Use Measurement Scale at a fake factor |
//! | `primitives/box-distances/cases` | Apart, overlapping, diagonal, inside the canvas, drawn by `SmartGuides` |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
