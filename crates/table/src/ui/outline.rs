//! The outline bands of grouped rows and columns (N6.03).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::grid::SheetGeometry;

craft_ui::words! {
    /// The band buttons' accessible names, English by default (Excel's).
    #[non_exhaustive]
    pub struct OutlineBandLabels {
        level: "outline.level" = "Level {n}";
        collapse_rows: "outline.collapseRows" = "Hide Detail, rows {first}–{last}";
        expand_rows: "outline.expandRows" = "Show Detail, rows {first}–{last}";
        collapse_columns: "outline.collapseColumns" = "Hide Detail, columns {first}–{last}";
        expand_columns: "outline.expandColumns" = "Show Detail, columns {first}–{last}";
    }
}

/// The least sheet zoom a lane's width follows; below it lanes stay at this share of
/// `sheet.outline_lane` (GridCraft `outline.rs:44`). A proportion, not a size.
pub const LANE_MIN_ZOOM: f32 = 0.75;

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

/// A lane's width at `zoom` (sheet zoom, 1.0 = 100 %): `lane × max(zoom, LANE_MIN_ZOOM)`,
/// rounded, `lane` being `sheet.outline_lane` ([`SheetStyle`](super::SheetStyle)).
// gridcraft: crates/ui-egui/src/outline.rs:44 `lane`.
pub fn lane(lane: f32, zoom: f32) -> f32 {
    todo!()
}

/// Excel's outline bands (GridCraft): a strip left of the row headers and one above the column
/// headers, a lane per level; each expanded group a bracket along its lane from its first detail
/// line to its button, with a tick at the start; − / + on the summary line; the level buttons
/// 1…n+1 in the corner. Layout and hit tests are in logical coordinates and mirrored with the
/// grid. Buttons are Tab stops (level buttons first), Space presses; AccessKit `Button`s named by
/// [`OutlineBandLabels`] ("Level 2", "Hide Detail, rows 4–9", numbers through `LocaleNumbers`).
pub struct OutlineBand<'a> {
    id_salt: IdSalt,
    rows: &'a OutlineAxis,
    columns: &'a OutlineAxis,
    geometry: &'a SheetGeometry,
    zoom: f32,
    labels: Option<OutlineBandLabels>,
}

impl<'a> OutlineBand<'a> {
    pub fn new(id_salt: impl AsIdSalt, rows: &'a OutlineAxis, columns: &'a OutlineAxis, geometry: &'a SheetGeometry, zoom: f32) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rows, columns, geometry, zoom, labels: None }
    }

    pub fn labels(mut self, labels: OutlineBandLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The band's thickness for an outline `levels` deep: `(levels + 1)` lanes ([`lane`] of
    /// `style.outline_lane`) plus [`BAND_SLACK_LANES`] of a lane; 0 without an outline.
    // gridcraft: crates/ui-egui/src/outline.rs:50 `band` (`(levels + 1) × lane + 3`).
    pub fn thickness(levels: u8, zoom: f32, style: &super::SheetStyle) -> f32 {
        todo!()
    }

    // gridcraft: crates/ui-egui/src/outline.rs:88 `Button`, `:96` `Bracket`, `:105` `Layout`,
    // `:113` `bands` and the layout below.
    pub fn show(self, ui: &mut Ui) -> OutlineBandOutput {
        todo!()
    }
}

/// The room after the last lane, as a share of one lane: GridCraft's 3 points over its 15-point
/// lane, kept as a proportion so it follows `sheet.outline_lane`.
pub const BAND_SLACK_LANES: f32 = 0.2;

/// What the bands did this pass.
#[must_use]
pub struct OutlineBandOutput {
    pub response: Response,
    pub action: Option<OutlineAction>,
}
