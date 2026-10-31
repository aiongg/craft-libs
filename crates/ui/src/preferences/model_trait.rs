//! [`PreferencesModel`]: what the app implements.

use egui::{Context, Ui};

use super::model::{PageSpec, PrefValue};

/// The app's side of the Preferences window. The model keeps the saved settings and the draft;
/// the window draws the pages [`pages`](PreferencesModel::pages) returns, filters them and reports
/// edits through [`set`](PreferencesModel::set).
///
/// The window reads its captions from [`Words`](crate::localized::Words) and its numbers from
/// [`LocaleNumbers`](crate::localized::LocaleNumbers), so both follow the installed
/// [`Host`](craft_host::Host). To preview a language while the window is open, install
/// `host.with_lang(draft_lang)`; the window follows on the next pass. [`cancel`](PreferencesModel::cancel)
/// installs the saved language again.
pub trait PreferencesModel {
    /// The pages, built from the draft.
    fn pages(&self) -> Vec<PageSpec<'_>>;
    /// A row changed in the draft. A cancelled gesture reports the value from before it.
    fn set(&mut self, key: &str, value: PrefValue);
    /// Draws a [`RowKind::Custom`](super::RowKind::Custom) row's control.
    fn custom_row(&mut self, ui: &mut Ui, key: &str);
    /// Draws a [`PageSpec::custom`] page.
    fn custom_page(&mut self, ui: &mut Ui, page: &str);
    /// Whether the draft differs from the saved settings: enables Apply.
    fn differs(&self) -> bool;
    /// Whether OK may close the window.
    fn can_ok(&self) -> bool {
        true
    }
    /// Commits the draft. `Err` keeps it and the window open and shows the message.
    fn apply(&mut self) -> Result<(), String>;
    /// Drops the draft and undoes previews (the language).
    fn cancel(&mut self);
    /// Resets the page's rows in the draft to their defaults.
    fn reset_page(&mut self, page: &str);
    /// Draws the dialogs the host opened from this window (a colour picker) inside the same
    /// viewport. Called last in every pass.
    fn nested(&mut self, _ctx: &Context) {}
    /// Whether a nested dialog is open: the window takes no keys then, and refuses OK.
    fn nested_open(&self) -> bool {
        false
    }
}
