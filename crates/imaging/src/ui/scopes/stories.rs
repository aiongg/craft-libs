//! Stories of the scope panel (task ST6): stubs, not registered. Registering them (task RG2) adds
//! `stories()` to `craft_imaging::ui`'s `register!` list, the words to `WORDS` and regenerates the
//! registry dump; until then the story tests would run the `todo!()` bodies.
//!
//! Every story uses fakes: histograms and vectorscope counts generated from a seed (RGB and CMYK
//! modes), a generated preview image for the clipping overlay, and story words (the gallery's
//! pseudo-locale test).
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/imaging/scopes/compact` | Compact View; arg: RGB or CMYK mode |
//! | `patterns/imaging/scopes/expanded` | Expanded View with the Channel and Source rows and statistics; arg: mode |
//! | `patterns/imaging/scopes/all-channels` | All Channels View; arg: mode |
//! | `patterns/imaging/scopes/colors-overlay` | the Colors channel, light and ink |
//! | `patterns/imaging/scopes/adjusted-over-original` | the outlined original and the stale mark |
//! | `patterns/imaging/scopes/tone-zones` | five zones, the hover band, a drag |
//! | `patterns/imaging/scopes/vectorscope` | the cloud, the hover target; args: red position, skin line |
//! | `patterns/imaging/scopes/clipping` | the warnings on and off, the overlay over a generated image |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The eight stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
