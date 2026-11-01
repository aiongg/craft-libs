//! Stories of the Connector tool and bar (task ST-SH2): stubs, not registered. Registering them
//! (task RG2) adds `stories()` to craft-draw's stories and the components to `COMPONENTS`.
//!
//! Every story uses fake targets (three boxes, one with numbered sites); every string goes through
//! story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `draw/connector-tool` | Straight, elbow and curved; attach points; reattach; reshape |
//! | `draw/connector-routes` | The route gallery: every side pair and boxes in the way |
//! | `draw/connector-bar` | Routing, caps, line style |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
