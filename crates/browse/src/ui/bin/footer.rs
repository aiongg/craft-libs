//! The bin footer: view buttons, zoom, the host's actions and their drops (`BinFooterAction`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::icons::Icon;

/// A host button in the bin's footer (FilmCraft: Automate to Sequence, Find, New Item, Clear).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinFooterAction {
    /// The id reported in `BinEvent::Footer` and `FooterDrop`.
    pub id: &'static str,
    /// The glyph.
    pub icon: Icon,
    /// The tooltip and accessible name, translated.
    pub label: String,
    /// Whether a drag of items may drop on it (FilmCraft's New Item makes a sequence from a clip).
    pub accepts_drops: bool,
    /// Enabled.
    pub enabled: bool,
}

impl BinFooterAction {
    /// An enabled action that takes no drops.
    pub fn new(id: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer` buttons).
        todo!()
    }

    /// Lets a drag of items drop on it.
    #[must_use]
    pub fn accepts_drops(self, on: bool) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (a clip dropped on New Item makes a sequence).
        todo!()
    }

    /// Enables or disables it.
    #[must_use]
    pub fn enabled(self, on: bool) -> Self {
        // new: builder.
        todo!()
    }
}

/// Draws the footer: a `PanelFooter` (height `panel_footer`) with the view `Segmented` (List,
/// Icon, Freeform glyphs), a `ThumbnailSlider`, Sort Icons (Icon view), each action as an
/// `IconButton` followed by `PanelFooter::drop_target::<ListDrag<K>>` when it accepts drops, and
/// New Bin. Returns the footer's events.
pub(super) fn footer_events() {
    // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`).
    todo!()
}
