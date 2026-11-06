//! The selected cells of a document table (N3.25), drawn over the table as a tint with an accent
//! outline, as InDesign and Word show selected cells.
//!
//! The selection and its rules are craft-ui's shared cell model
//! ([`craft_ui::behavior::cells`]), the one craft-table's sheet uses too. This module keeps what
//! is a document table's own: where its rows and columns are on screen ([`TableGeometry`]) and the
//! tint ([`CellSelectionOverlay`]). The app's table tool runs the input over the model:
//!
//! - the table as a [`CellSpace`]: `CellSpace::new(rows, columns).merges(&merges)`, each merge a
//!   [`CellRange`] (a merged cell counts as one; ranges grow over the merges they cross,
//!   [`CellRange::covering_merges`]);
//! - a press in a cell [`CellSelection::select`]; a drag or ⇧-click
//!   [`CellSelection::extend_to`]; ⇧-arrows [`CellSelection::extend`] with a [`CellDirection`];
//! - Tab and ⇧-Tab: [`CellSpace::next_in_reading_order`] from the active cell, wrapping to the
//!   next row; `None` past the last cell, where the app adds a row (InDesign, Word);
//! - Esc toggles between the cells and the first cell's text (the app's text tool).
//!
//! [`CellSpace`]: craft_ui::behavior::cells::CellSpace
//! [`CellRange`]: craft_ui::behavior::cells::CellRange
//! [`CellRange::covering_merges`]: craft_ui::behavior::cells::CellRange::covering_merges
//! [`CellSelection::select`]: craft_ui::behavior::cells::CellSelection::select
//! [`CellSelection::extend_to`]: craft_ui::behavior::cells::CellSelection::extend_to
//! [`CellSelection::extend`]: craft_ui::behavior::cells::CellSelection::extend
//! [`CellDirection`]: craft_ui::behavior::cells::CellDirection
//! [`CellSpace::next_in_reading_order`]: craft_ui::behavior::cells::CellSpace::next_in_reading_order

use craft_ui::behavior::cells::{CellRange, CellRef, CellSelection};
use craft_ui::theme::current;
use egui::{Painter, Rect, StrokeKind, pos2};

use crate::ui::CellSelectionStyle;

/// The alpha of the selected cells' tint.
const FILL_ALPHA: u8 = 0x6E;

/// Where the table's rows and columns are on screen: the edges, top to bottom and left to right
/// (n + 1 values for n rows).
#[derive(Clone, Debug, PartialEq)]
pub struct TableGeometry {
    /// Screen y of each row boundary.
    pub row_edges: Vec<f32>,
    /// Screen x of each column boundary.
    pub column_edges: Vec<f32>,
}

impl TableGeometry {
    /// The cell under a screen point, if any (a merged cell answers with the cell the point is
    /// in; the model's `CellSpace::merge_at` maps it to the merge).
    pub fn cell_at(&self, p: egui::Pos2) -> Option<CellRef> {
        let row = band(&self.row_edges, p.y)?;
        let column = band(&self.column_edges, p.x)?;
        Some(CellRef::new(row, column))
    }

    /// The screen rectangle of a range: its first row's and column's leading edges to its last
    /// row's and column's trailing edges.
    pub fn rect(&self, range: CellRange) -> Rect {
        let edge = |edges: &[f32], i: u32| edges.get(i as usize).or(edges.last()).copied();
        match (
            edge(&self.column_edges, range.start.column),
            edge(&self.row_edges, range.start.row),
            edge(&self.column_edges, range.end.column + 1),
            edge(&self.row_edges, range.end.row + 1),
        ) {
            (Some(left), Some(top), Some(right), Some(bottom)) => Rect::from_min_max(pos2(left, top), pos2(right, bottom)),
            _ => Rect::NOTHING,
        }
    }
}

/// The index of the band of `edges` (n + 1 ascending values for n bands) holding `v`: a band
/// owns its leading edge and not its trailing one.
fn band(edges: &[f32], v: f32) -> Option<u32> {
    let after = edges.partition_point(|&e| e <= v);
    let last = edges.len().checked_sub(1)?;
    (1..=last).contains(&after).then(|| (after - 1) as u32)
}

/// The overlay over selected cells, painted after the table.
///
/// - Every range of the selection ([`CellSelection::ranges`]) tints with
///   [`CellSelectionStyle`]'s fill, the active cell included
///   (InDesign tints it too, unlike Excel), and each range is outlined with its outline (accent).
///   A caret or text selection inside one cell draws as text (InDesign; DesignCraft).
/// - Paint only: the app's table tool runs clicks, drags and keys over the shared model (module
///   docs).
pub struct CellSelectionOverlay<'a> {
    geometry: &'a TableGeometry,
    selection: &'a CellSelection,
}

impl<'a> CellSelectionOverlay<'a> {
    /// An overlay of `selection` over the table at `geometry`.
    pub fn new(geometry: &'a TableGeometry, selection: &'a CellSelection) -> Self {
        CellSelectionOverlay { geometry, selection }
    }

    /// The overlay's colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> CellSelectionStyle {
        use craft_ui::theme::ToEgui as _;
        let accent = tokens.color.accent.solid;
        CellSelectionStyle {
            fill: accent.with_alpha(FILL_ALPHA).to_egui(),
            outline: egui::Stroke::new(tokens.stroke.thin, accent.to_egui()),
        }
    }

    // designcraft: crates/ui-egui/src/canvas/text_overlay.rs:16 `draw_text_selection` (the tint of every selected cell; `:126` `draw_cell_selection` is the in-cell caret).
    /// Paints the tint over every selected range.
    pub fn paint(self, painter: &Painter) {
        let style = Self::style(&current(painter.ctx()));
        for &range in self.selection.ranges() {
            let rect = self.geometry.rect(range);
            painter.rect_filled(rect, 0.0, style.fill);
            painter.rect_stroke(rect, 0.0, style.outline, StrokeKind::Inside);
        }
    }
}
