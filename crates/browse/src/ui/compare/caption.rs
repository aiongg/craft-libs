//! `CompareCaption`: label, name, stars, flag and colour dot.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Rgba;
use egui::{Rect, Ui};

use super::CompareStyle;
use crate::ui::grid::TileFlag;

/// The strip under a compare tile's picture (`tile.compare_caption` 26 high): the label
/// (`BodyStrong`), the file name (`Caption`; dropped when less than `tile.compare_name_room`,
/// 110, remain), the stars (display only: `paint_rating`, `rating.cell` steps), the flag (14)
/// and the colour dot (10). Right under the picture, at the strip's bottom when zoomed in. Runs
/// from the right in a right-to-left interface.
#[derive(Clone, Debug, PartialEq)]
pub struct CompareCaption {
    /// The pane's label ("Select", "Candidate"), translated.
    pub label: String,
    /// The file name.
    pub name: String,
    /// Stars 0–5.
    pub rating: Option<u8>,
    /// Pick or reject.
    pub flag: Option<TileFlag>,
    /// The colour label.
    pub label_color: Option<Rgba>,
}

impl CompareCaption {
    /// A caption with a label and a name.
    pub fn new(label: impl Into<String>, name: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:345 (the caption strip; the name needs 110).
        todo!()
    }

    /// The stars.
    #[must_use]
    pub fn rating(self, rating: Option<u8>) -> Self {
        // new: builder.
        todo!()
    }

    /// The flag.
    #[must_use]
    pub fn flag(self, flag: Option<TileFlag>) -> Self {
        // new: builder.
        todo!()
    }

    /// The colour label.
    #[must_use]
    pub fn label_color(self, color: Option<Rgba>) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the strip in `rect`; returns the text an accessible name reads.
    pub(super) fn paint(&self, ui: &Ui, rect: Rect, style: &CompareStyle) -> String {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:345 (label 12 semibold, name 11.5, stars 13 apart, flag 14, dot 10).
        todo!()
    }
}
