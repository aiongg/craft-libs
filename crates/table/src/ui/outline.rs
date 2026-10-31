//! The outline bands of grouped rows and columns (N6.03).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Response, Ui};

use super::grid::GridGeometry;

/// One lane's width at 100 % zoom, points (GridCraft `outline.rs:22`).
pub const LANE: f32 = 15.0;

/// A group of lines `start..=end` at `level` (1 = outermost), with its summary line after it
/// (below or right; before it when `summary_before`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutlineGroup {
    pub start: u32,
    pub end: u32,
    pub level: u8,
    pub collapsed: bool,
    pub summary_before: bool,
}

/// One direction's outline: its deepest level and its groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlineAxis {
    pub levels: u8,
    pub groups: Vec<OutlineGroup>,
}

/// What a band button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineAction {
    /// Show levels 1…`level` (the corner buttons 1…n+1).
    Level { rows: bool, level: u8 },
    /// Collapse or expand a group (its − / + button).
    Toggle { rows: bool, group: OutlineGroup },
}

/// A lane's width at `zoom` (sheet zoom, 1.0 = 100 %): `LANE × max(zoom, 0.75)`, rounded.
// gridcraft: crates/ui-egui/src/outline.rs:44 `lane`.
pub fn lane(zoom: f32) -> f32 {
    todo!()
}

/// Excel's outline bands (GridCraft): a strip left of the row headers and one above the column
/// headers, a lane per level; each expanded group a bracket along its lane from its first detail
/// line to its button, with a tick at the start; − / + on the summary line; the level buttons
/// 1…n+1 in the corner. Layout and hit tests are in logical coordinates and mirrored with the
/// grid. Buttons are Tab stops (level buttons first), Space presses; AccessKit names "Level 2",
/// "Collapse rows 4–9".
pub struct OutlineBand<'a> {
    rows: &'a OutlineAxis,
    cols: &'a OutlineAxis,
    geometry: &'a GridGeometry,
    zoom: f32,
}

impl<'a> OutlineBand<'a> {
    pub fn new(rows: &'a OutlineAxis, cols: &'a OutlineAxis, geometry: &'a GridGeometry, zoom: f32) -> Self {
        Self { rows, cols, geometry, zoom }
    }

    /// The band's thickness for an outline `levels` deep: `(levels + 1)` lanes plus 3; 0 without
    /// an outline.
    // gridcraft: crates/ui-egui/src/outline.rs:50 `band`.
    pub fn thickness(levels: u8, zoom: f32) -> f32 {
        todo!()
    }

    // gridcraft: crates/ui-egui/src/outline.rs:88 `Button`, `:96` `Bracket`, `:105` `Layout`,
    // `:113` `bands` and the layout below.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<OutlineAction>) {
        todo!()
    }
}
