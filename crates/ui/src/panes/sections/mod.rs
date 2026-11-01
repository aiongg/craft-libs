//! Reorderable sidebar sections the user folds, hides and drags: a pinned header, a header
//! button, the section menu, keys and a lenient saved layout (`ReorderableSections`,
//! `SectionLayout`, `SectionKey`, `SectionEvent`).
//!
//! ```ignore
//! let sections = [
//!     SectionInfo::new(Sidebar::Albums, tr("Albums")).count(albums.len()).empty_hint(tr("No albums yet"))
//!         .button(SectionButton::new("albumNew", Icon::Plus, tr("Create Album"))),
//!     SectionInfo::new(Sidebar::Local, tr("Local")).available(cfg!(not(target_arch = "wasm32"))),
//! ];
//! let out = ReorderableSections::new("left", &state.sidebar).show(ui, &sections, |ui, section| match section {
//!     Sidebar::Local => local_tree(ui),
//!     Sidebar::Albums => albums_list(ui),
//! });
//! for event in out.events { state.sidebar.apply(&event); }          // the widget reports, the host applies
//! ```

mod layout;
#[cfg(test)]
mod layout_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod sections_tests;
pub(crate) mod stories;
mod widget;

pub use layout::{SectionEvent, SectionKey, SectionLayout, SectionState};
pub use widget::{ReorderableSections, SectionButton, SectionInfo, SectionsLabels, SectionsOutput};
