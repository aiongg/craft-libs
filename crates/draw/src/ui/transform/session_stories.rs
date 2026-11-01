//! Stories of the Free Transform session (task ST-TR1): stubs, not registered. Registering them
//! (task RG2) adds `stories()` to the draw stories and the components to `COMPONENTS`, and
//! regenerates the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! A picture over a fake document; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/transform-session/session` | The overlay and the bar over a fake picture: modes, scale rule, reference point shown or hidden, the readout, Apply and Discard reopening a box. Args: mode, rule, reference point, interpolation |
//! | `composites/transform-session/bar` | The bar alone in its phases: Free, the Content-Aware subset (`BarGroups`), Warp with a stub warp group |

use craft_ui::story::Story;

/// The stories above.
#[expect(dead_code, clippy::todo, reason = "skeleton: registered and filled in tasks ST-TR1 and RG2")]
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
