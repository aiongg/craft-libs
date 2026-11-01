//! Stories of the tables: stubs, not yet added to [`super::stories`]. Registering them changes
//! `crates/ui/tests/dumps/stories/registry.txt`, and `every_story_shows_with_its_default_args`
//! would run the `todo!()` bodies; the stories fill task writes them, registers them, adds their
//! components to [`super::COMPONENTS`] and regenerates the dump.
//!
//! Every string a story shows (column labels, file names, metadata, event messages) goes
//! through the story words, so the gallery's pseudo-locale test sees no stray English. The event
//! story passes a fixed `now_ms`, so its ages don't change between screenshots.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The story ids:
/// - `composites/table/files`: a `Table` of 200 files (Name, Pages, Size, Modified, Warnings;
///   Size and Modified hideable, Pages and Size numeric), sorting and resizing (pointer and
///   keys) live; args for striped and movable; prints the events.
/// - `composites/table/metadata`: a `MetadataTable` of a photo's EXIF and XMP groups with the
///   filter; an arg turns on the column picker.
/// - `composites/table/events`: an `EventLog` of a dozen events of each level, two repeated.
pub(super) const IDS: [&str; 3] = ["composites/table/files", "composites/table/metadata", "composites/table/events"];

pub(super) fn stories() -> Vec<Story> {
    todo!()
}
