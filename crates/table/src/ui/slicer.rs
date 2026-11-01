//! Slicers: tiles that filter a table by a column's values (N6.06).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{IdSalt, Rect, Response, Ui};

use super::style::SheetStyle;

craft_ui::words! {
    /// The slicer's words, English by default (Excel's).
    #[non_exhaustive]
    pub struct SlicerLabels {
        clear: "slicer.clear" = "Clear Filter";
    }
}

/// A slicer's parts.
#[derive(Clone, Debug, PartialEq)]
pub struct SlicerLayout {
    pub header: Rect,
    /// The clear-filter button in the header.
    pub clear: Rect,
    /// One rect per value tile, in order.
    pub tiles: Vec<Rect>,
}

/// The layout of a slicer in `rect` at `zoom` with `n` values in `columns` columns: the header,
/// padding, gap, tile height and clear button from `style` (`sheet.slicer_*`, each × zoom),
/// tiles filling the width.
// gridcraft: crates/ui-egui/src/chartview.rs:400 `slicer_layout`.
pub fn slicer_layout(rect: Rect, zoom: f32, n: usize, columns: u32, style: &SheetStyle) -> SlicerLayout {
    todo!()
}

/// What a click on a slicer did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlicerAction {
    Toggle(String),
    Clear,
}

/// Excel's slicer (GridCraft): a card with a soft shadow, the caption in the accent-soft header
/// with the funnel button (tinted while a filter is on), value tiles (on: accent with on-accent
/// text; off: hover with dim text); tiles that overflow the card are left out. A click on a tile
/// toggles its value, on the funnel clears; elsewhere it is the app's (select, drag the object).
/// The slicer is one Tab stop whose arrows move between the tiles in reading order
/// ([`GridNav`](craft_ui::behavior::GridNav) over the tile columns), Space toggles the focused
/// tile as a click does, and the funnel button is a Tab stop of
/// its own named by [`SlicerLabels::clear`]. AccessKit: a `ListBox` of `ListBoxOption`s with
/// `selected`. The tiles are the slicer's own (Excel's look); step 5's `Chip` is a removable tag
/// and `ToggleButton` a pressed button, neither of which is a slicer tile.
pub struct Slicer<'a> {
    id_salt: IdSalt,
    caption: &'a str,
    items: &'a [(String, bool)],
    columns: u32,
    filtered: bool,
    zoom: f32,
    labels: Option<SlicerLabels>,
}

impl<'a> Slicer<'a> {
    /// `items`: each value and whether it is on.
    pub fn new(id_salt: impl egui::AsIdSalt, caption: &'a str, items: &'a [(String, bool)]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), caption, items, columns: 1, filtered: false, zoom: 1.0, labels: None }
    }

    pub fn labels(mut self, labels: SlicerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn columns(mut self, columns: u32) -> Self {
        self.columns = columns;
        self
    }

    pub fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// The sheet zoom the slicer sits at.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    // gridcraft: crates/ui-egui/src/chartview.rs:424 `paint_slicers`, `:472` `slicer_click`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> SlicerOutput {
        todo!()
    }
}

/// What the slicer did this pass.
#[must_use]
pub struct SlicerOutput {
    pub response: Response,
    pub action: Option<SlicerAction>,
}
