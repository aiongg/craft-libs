//! The browsing widgets.
//!
//! - `grid`: `TileGrid`, the thumbnail grid.
//! - `bin`: `MediaBin`, the project or media panel.
//! - `places`: `PlacesTree`, the folder tree.
//! - `compare`: `CompareTile` and the compare arrangements.
//! - `relink`: `LinkList` and `RelinkDialog`.

pub mod bin;
pub mod compare;
pub mod grid;
pub mod places;
pub mod relink;

pub use bin::{BinLabels, BinState, BinStyle, MediaBin};
pub use compare::{CompareLabels, CompareStyle, CompareTile};
pub use grid::{TileGrid, TileLabels, TileStyle};
pub use places::{PlacesLabels, PlacesStyle, PlacesTree};
pub use relink::{LinkList, LinkListLabels, RelinkDialog, RelinkLabels, RelinkStyle};

craft_ui::story::register! {
    crate::thumbs::stories,
    bin::stories,
    grid::stories,
    places::stories,
    compare::stories,
    relink::stories,
}

/// The stories of the browsing widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The labels tables of the browsing widgets, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    <grid::TileLabels as craft_ui::localized::Labels>::WORDS,
    <bin::BinLabels as craft_ui::localized::Labels>::WORDS,
    <places::PlacesLabels as craft_ui::localized::Labels>::WORDS,
    <compare::CompareLabels as craft_ui::localized::Labels>::WORDS,
    <relink::LinkListLabels as craft_ui::localized::Labels>::WORDS,
    <relink::RelinkLabels as craft_ui::localized::Labels>::WORDS,
    <crate::thumbs::stories::FileThumbStoryLabels as craft_ui::localized::Labels>::WORDS,
    <bin::stories::BinStoryLabels as craft_ui::localized::Labels>::WORDS,
    <grid::stories::GridStoryLabels as craft_ui::localized::Labels>::WORDS,
    <places::stories::PlacesStoryLabels as craft_ui::localized::Labels>::WORDS,
    <compare::stories::CompareStoryLabels as craft_ui::localized::Labels>::WORDS,
    <relink::stories::LinkStoryLabels as craft_ui::localized::Labels>::WORDS,
];

/// The count words of the browsing widgets, for the words registry.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[bin::ITEMS, places::HIDDEN_PLACES];
