//! `CompareTile`, `CompareCaption` and the compare arrangements (`ComparePair`,
//! `survey_columns`).
//!
//! ```ignore
//! let [left, right] = ComparePair::new().rtl(rtl).rects(area, &style);
//! let select = CompareTile::new(("compare", 0), &mut state.view, photo_a.image())   // one CanvasView for both
//!     .caption(CompareCaption::new(tr("Select"), &photo_a.name).rating(photo_a.rating).flag(photo_a.flag))
//!     .active(active == 0)
//!     .show(ui, left);
//! if select.double_clicked { CompareTile::toggle_zoom(&mut state.view, photo_a.size, left, ui.ctx().pixels_per_point()) }
//! ```

mod arrange;
#[cfg(test)]
mod arrange_tests;
mod caption;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod compare_tests;
pub(crate) mod stories;
mod style;
mod tile;

pub use arrange::{ComparePair, survey_columns};
pub use caption::CompareCaption;
pub use style::CompareStyle;
pub use tile::{CompareImage, CompareResponse, CompareTile, RenderState};

craft_ui::words! {
    /// The compare tile's words, English by default.
    #[non_exhaustive]
    pub struct CompareLabels {
        rendering: "browse.compareRendering" = "Rendering…";
        render_failed: "browse.compareRenderFailed" = "Render failed";
        /// The tile's accessible name: `{label}`: `{name}`, then `{details}` (stars, flag, label).
        tile_name: "browse.compareTileName" = "{label}: {name}{details}";
        /// One entry of `{details}`: `{n}` the rating.
        stars: "browse.compareStars" = "{n} of 5 stars";
        /// One entry of `{details}`: the pick flag.
        picked: "browse.comparePicked" = "picked";
        /// One entry of `{details}`: the reject flag.
        rejected: "browse.compareRejected" = "rejected";
    }
}
