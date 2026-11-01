//! Stories of the Navigator (task ST5): stubs, not registered. Registering them (task RG2) adds
//! `stories()` to the canvas stories, `Navigator` to `COMPONENTS` and regenerates the registry
//! dump; until then `every_story_shows_with_its_default_args` would run the `todo!()` bodies.
//! No `COMPONENTS` constant is declared here before RG2 (craft-ui's
//! `every_declared_component_has_a_story` counts them).
//!
//! Every story uses fakes: a generated gradient image put into a story-local `TextureCache` for
//! `NavigatorPreview::Texture`, a `CanvasView` the story keeps, and story words (the gallery's
//! pseudo-locale test). Canvas stories show `.on(Surface::Canvas)`.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/canvas/navigator/document` | a landscape document, the view zoomed to 400 %; args: zoom, rotation, flip, proxy colour, preview waiting |
//! | `patterns/canvas/navigator/whole-document` | the fit view: the proxy covers the preview |
//! | `patterns/canvas/navigator/options-dialog` | Panel Options with the colour list and Custom… |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The three stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
