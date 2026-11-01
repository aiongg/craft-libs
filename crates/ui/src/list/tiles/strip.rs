//! The grid's parts for page panels (Acrobat's Pages panel, PowerPoint's slide strip; craft-pages'
//! `PageThumbnails` is built on them): the current tile, captions at the reading start with
//! marks, groups that open and close, group menus, the new item row and file drops from outside.
//!
//! Skeleton: the builders on [`TileGrid`](super::TileGrid) store these; a grid that sets any of
//! them reaches `todo!()` until the bodies land. Existing callers set none.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::icons::Icon;

/// Where a `Cards` caption goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CaptionPlace {
    /// Under the picture (Bridge; Acrobat's page labels).
    #[default]
    Below,
    /// In a column at the picture's reading start, top aligned, `tile.start_caption_width` wide
    /// (PowerPoint's slide numbers): the name in `tile.caption` (`tile.caption_selected` when
    /// selected), crossed by a slash when [`TileContent::hidden`](super::TileContent::hidden), and
    /// the tile's [`marks`](super::TileContent::marks) under it, one per line, in `tile.detail`.
    /// The arrangement makes each card that much wider; no caption band under the picture.
    Start,
}

/// A glyph under a [`CaptionPlace::Start`] caption (PowerPoint's animation star).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileMark {
    /// The glyph, drawn at `IconSize::Sm`.
    pub icon: Icon,
    /// Its tooltip, and a part of the tile's accessible name ("Has animations").
    pub tip: String,
}

/// What the strip builders set on one grid.
pub(super) struct Strip<'a, K> {
    pub current: Option<K>,
    pub caption_place: CaptionPlace,
    pub group_open: Option<&'a [bool]>,
    pub group_menus: bool,
    pub new_item: Option<String>,
    pub file_drops: bool,
}

impl<K> Default for Strip<'_, K> {
    fn default() -> Self {
        Self { current: None, caption_place: CaptionPlace::Below, group_open: None, group_menus: false, new_item: None, file_drops: false }
    }
}

impl<K> Strip<'_, K> {
    /// Whether any strip builder was used.
    pub fn is_set(&self) -> bool {
        self.current.is_some()
            || self.caption_place != CaptionPlace::Below
            || self.group_open.is_some()
            || self.group_menus
            || self.new_item.is_some()
            || self.file_drops
    }

    /// The strip parts, each where the grid's pass does the same for its own parts: the current
    /// ring after the selection ring in `Pass::tile` (`tile.ring.current`, `accent.solid`) with
    /// `set_aria_current` on the option node, and a reveal (`State::reveal`) when the current
    /// key differs from the last pass's, except while dragging; the `Start` caption in `arrange`
    /// (cards wider by `tile.start_caption_width`, no caption band) and in `paint_tile` (the
    /// slash across the name, the marks); closed groups dropped from `arrange`, `GridNav` and
    /// AccessKit, the chevron in `groups::group_header` (`paint_disclosure`) with Expand and
    /// Collapse actions; group
    /// menus from a secondary click in `Pass::group_picks`; the new item row after the last row
    /// in `Pass::draw` (its own focusable `Button` node); file drops in `Pass::drag`'s manner over
    /// `hovered_files` with `grid_drop`, filling `TileGridOutput::file_gap`.
    pub fn show_unbuilt(&self) {
        // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 `pages` (current page border, file gap :277 `panel_gap`);
        // deckcraft: crates/ui-egui/src/panels/thumbs.rs:116 `list` (sections :146, number in a 30 pt column :184 and :193, New Slide :284).
        todo!()
    }
}

/// [`paint_tile`](super::paint_tile)'s part for [`TileContent::hidden`](super::TileContent::hidden)
/// and [`TileContent::marks`](super::TileContent::marks): the veil over a hidden tile's picture
/// (`dimmed`'s), and with a `Start` caption the slash across the name and the marks.
pub(super) fn paint_hidden_and_marks() {
    // deckcraft: crates/ui-egui/src/panels/thumbs.rs:198 (the hidden slide's slash, its picture at alpha 140) and :202 (the animation star under the number).
    todo!()
}
