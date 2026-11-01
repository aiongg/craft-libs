//! Stories of the Layer Style dialog (task ST-AD2): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the imaging stories and the components to the registry, and
//! regenerates the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! A fake image (a rounded shape on a checkerboard) stands in for the document, and the story
//! paints a rough shadow from the draft so the shadow drag shows without an engine. Every string
//! goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `imaging/layer-style/dialog` | Drop Shadow selected over the fake image; a drag on the image moves the shadow |
//! | `imaging/layer-style/blending` | Blending Options with Blend If on Gray, R, G, B; a CMYK switch |
//! | `imaging/layer-style/effects/<kind>` | One story per effect page, ten ids (`bevel-emboss`, `stroke`, `inner-shadow`, `inner-glow`, `satin`, `color-overlay`, `gradient-overlay`, `pattern-overlay`, `outer-glow`, `drop-shadow`) |
//! | `imaging/layer-style/styles` | The Styles page with fake thumbnails |
//! | `imaging/layer-style/contour` | The contour field, its picker and the Contour Editor |
//! | `imaging/layer-style/blend-if` | One Blend If bar, split and unsplit, with its keys |

use craft_ui::story::Story;

/// The stories above.
#[expect(dead_code, clippy::todo, reason = "skeleton: task ST-AD2")]
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
