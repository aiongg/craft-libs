//! The raster image editing widgets, for egui.
//!
//! - [`warp`]: the [`WarpGrid`] on a canvas, the mesh of PhotoCraft's Edit › Transform › Warp, its
//!   anchors and handles dragged and nudged.
//! - [`adjust`]: the [`LevelsEditor`], a channel's levels over its histogram.

pub mod adjust;
pub mod warp;

#[cfg(test)]
mod id_tests;
#[cfg(test)]
mod words_tests;

pub use adjust::{Levels, LevelsClip, LevelsEditor, LevelsLabels, LevelsStyle};
pub use warp::{
    WarpEdit, WarpGrid, WarpMesh, WarpNames, WarpPress, WarpRelease, WarpResponse, WarpSide, WarpSplit, WarpSplitAt, WarpSplitPress, WarpStyle,
    WarpTarget, WarpTrack,
};

craft_ui::story::register! {
    warp::story,
    adjust,
}

/// The stories of the imaging widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The labels tables of the imaging widgets, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] =
    &[<WarpNames<'static> as craft_ui::localized::Labels>::WORDS, <LevelsLabels as craft_ui::localized::Labels>::WORDS];
