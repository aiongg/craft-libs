//! [`FilterTree`]: the middle column's category folders with thumbnails (task WS-FG2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_ui::canvas::textures::Thumbnails;

use super::FilterGalleryLabels;
use super::catalog::{FilterCategory, GalleryThumb};

/// What [`FilterTree::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterTreeResponse {
    pub response: Response,
    /// The filter whose thumbnail was clicked (or picked with Enter or Space).
    pub picked: Option<String>,
}

/// The middle column: one collapsible folder per category (its open state in egui memory; the
/// selected filter's folder opens), each a grid of thumbnails, three a row, 80 × 56 with the
/// filter's name under it in the caption role; the selected filter's thumbnail has the selection
/// ring. Thumbnails come from a `Thumbnails<GalleryThumb>` cache over the app's `ThumbSource`
/// (placeholder while loading).
///
/// Keyboard: each folder header is a Tab stop; the thumbnails of an open folder are one grid
/// (`GridNav`: arrows move, Enter or Space picks). AccessKit: a folder is a tree item, a
/// thumbnail a button named by its filter, `selected` when current.
#[expect(dead_code, reason = "skeleton: `show` (task WS-FG2) reads the fields")]
pub struct FilterTree<'a> {
    id_salt: IdSalt,
    categories: &'a [FilterCategory],
    labels_of: &'a dyn Fn(&str) -> String,
    selected: &'a str,
    thumbs: &'a Thumbnails<GalleryThumb>,
    labels: &'a FilterGalleryLabels,
}

impl<'a> FilterTree<'a> {
    /// The folders of `categories`; `labels_of` gives a filter key's translated name.
    pub fn new(
        id_salt: impl AsIdSalt,
        categories: &'a [FilterCategory],
        labels_of: &'a dyn Fn(&str) -> String,
        selected: &'a str,
        thumbs: &'a Thumbnails<GalleryThumb>,
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), categories, labels_of, selected, thumbs, labels }
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn show(self, ui: &mut Ui) -> FilterTreeResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:702-754 (folders, thumbnails 80 × 56, three
        // a row, names under them), :616 `update_thumbs`.
        todo!()
    }
}
