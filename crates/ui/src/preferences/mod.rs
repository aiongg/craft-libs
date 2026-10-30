//! The Preferences window's model, search and body.
//!
//! - [`PreferencesBody`]: the contents (search field, page list, page, OK, Cancel and Apply) in any
//!   `Ui`; what the window and the stories draw.
//! - [`PageSpec`], [`GroupSpec`], [`RowSpec`], [`RowKind`], [`PrefValue`]: the row model the host
//!   builds each frame from its settings and its draft. The library keeps no copy of the values.
//! - [`PreferencesHost`]: what the app implements: pages, draft, apply, cancel, reset, custom rows
//!   and nested dialogs.
//! - [`PreferencesTexts`]: every caption, already translated.
//! - [`matches`]: the search rule, public so hosts and tests share it.
//!
//! Sizes are the `preferences.*` and `group_box.*` tokens.

mod body;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod body_tests;
mod host;
mod model;
mod rows;
mod search;
#[cfg(test)]
mod search_tests;
mod state;
pub(crate) mod stories;

pub use body::PreferencesBody;
pub use host::PreferencesHost;
pub use model::{Closed, Columns, GroupSpec, PageSpec, PrefValue, PreferencesOutput, PreferencesTexts, RowKind, RowSpec};
pub use search::{Listed, list_pages, matches, row_matches, row_shown, shown_position};
pub use state::PreferencesState;
pub use stories::COMPONENTS;
