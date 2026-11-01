//! Stories of the textures (task ST1): stubs, not registered. Registering them (task RG2) adds
//! `stories()` to the canvas stories, the components to `COMPONENTS` and regenerates the registry
//! dump; until then `every_story_shows_with_its_default_args` would run the `todo!()` bodies.
//!
//! Every story uses fakes: [`FakeThumbSource`](super::FakeThumbSource) and a
//! [`ManualJobs`](craft_host::ManualJobs) in the options' `jobs`, with a "Finish next" button that
//! calls `run_next`, so loading, stand-in and failure states show one step at a time. Every string
//! goes through story words (the gallery's pseudo-locale test).
//!
//! | Story id | Shows |
//! |---|---|
//! | `canvas/textures/cache` | `TextureCache` with a small budget: Ready, Loading with stand-in, Empty, Failed, eviction as the key count grows, a `TextureStatistics` readout |
//! | `canvas/textures/thumbnails` | `Thumbnails` over `FakeThumbSource`: synchronous and background renders, portrait and landscape frames, a transparent picture on the checkerboard, `hold` |
//! | `canvas/textures/queue` | The order of visible, near and background requests with `ManualJobs`; requests scrolled away dropped |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The three stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
