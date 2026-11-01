//! Stories of the Filter Gallery workspace (task ST-WS1): stubs, not registered. Registering them
//! (task RG2) adds `filter_gallery::stories` to the crate's `register!` list and
//! `FilterGalleryWorkspace` to `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories use `MockGallery`: a six-filter catalog in two folders with `Param` lists (a Range,
//! a Choice, a Bool, a colour), thumbnails from `FakeThumbSource`, and a preview that is
//! `MockImage` tinted per effect (no engine). Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `dialogs/filter-gallery/default` | Folders open and closed, the selected thumbnail, the settings, a three-layer stack with one eye off |
//! | `dialogs/filter-gallery/rendering` | The zoom bar with a progress bar, then the note |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
