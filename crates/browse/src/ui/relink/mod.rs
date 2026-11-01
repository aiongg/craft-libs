//! `LinkList` (the Links panel), `StatusBadge` and `RelinkDialog` (Link Media).
//!
//! ```ignore
//! let out = LinkList::new("links", &links, monitor.statuses(), &mut state.links)
//!     .thumbnails(&thumbs, &link_thumbs).show(ui);
//! if out.settings_changed { settings.links = state.links.settings.clone() }
//! for event in out.events {
//!     match event {
//!         LinkListEvent::Relink(keys) => relink_with_file_pick(keys),        // one file: Files::pick, no dialog
//!         LinkListEvent::RelinkToFolder(keys) => relink_to_folder(keys),      // docpath::links::folder_matches in a job
//!         LinkListEvent::FindMissing => dialogs.open(RelinkDialog::new(missing_rows(), files.clone(), RelinkOptions::default())),
//!         LinkListEvent::GoTo(key) => go_to(key),
//!         _ => {}
//!     }
//! }
//! if let Some(DialogResult::Ok(outcome)) = dialogs.take::<RelinkDialog<LinkId>>() { doc.relink(outcome.linked) }   // one undo step
//! ```

mod badge;
mod dialog;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
mod list;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod list_tests;
pub(crate) mod stories;
mod style;

pub use badge::StatusBadge;
pub use dialog::{
    CandidateChecker, RelinkColumn, RelinkCriterion, RelinkDialog, RelinkLabels, RelinkLayout, RelinkOptions, RelinkOutcome, RelinkRow, SEARCHING,
};
pub use list::{LinkFilter, LinkList, LinkListEvent, LinkListLabels, LinkListOutput, LinkListSettings, LinkListState, LinkSort};
pub use style::RelinkStyle;
