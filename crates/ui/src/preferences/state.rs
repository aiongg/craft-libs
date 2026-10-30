//! What the window remembers between passes, in egui's memory.

use egui::{Context, Id};

/// The page the user chose, the search text and the error from the last Apply.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreferencesState {
    /// The user's page. While a search hides it another page shows, and this returns when the
    /// search is cleared.
    pub selected: String,
    pub search: String,
    /// The message of a failed Apply, shown over the buttons until the next edit or Apply.
    pub error: Option<String>,
    /// The page shown last pass: a change scrolls the page to its top.
    pub(super) shown: Option<String>,
    /// Whether the search field had the keyboard focus last pass.
    pub(super) search_focused: bool,
    /// The height the error notice took last pass.
    pub(super) notice_height: f32,
}

impl PreferencesState {
    pub fn load(ctx: &Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp::<PreferencesState>(id)).unwrap_or_default()
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}
