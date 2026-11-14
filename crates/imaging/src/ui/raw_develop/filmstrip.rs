//! The filmstrip of the raws being opened.

use craft_ui::contract::{Bind, Pattern};
use egui::{Response, Ui, load::SizedTexture};

/// One filmstrip tile.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RawFilmItem {
    /// The raw's name.
    pub name: String,
    /// The raw has edits.
    pub edited: bool,
    /// The tile's picture.
    pub picture: Option<SizedTexture>,
}

/// A one-column grid of tiles at the start edge, shown for more than one raw: the edited dot and the current frame.
#[must_use = "show the filmstrip with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(dialogs-w9): dl.K66
pub struct RawFilmstrip<'a> {
    items: &'a [RawFilmItem],
    current: Bind<'a, usize>,
}

impl<'a> RawFilmstrip<'a> {
    /// The filmstrip over `items` with `current` picked.
    pub fn new(items: &'a [RawFilmItem], current: Bind<'a, usize>) -> Self {
        let _ = (items, current);
        todo!("dl.K66")
    }

    /// Draws the tiles.
    pub fn show(self, ui: &mut Ui) -> Response {
        let _ = (self, ui);
        todo!("dl.K66")
    }
}
