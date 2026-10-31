//! Selection statistics in the status bar (N6.08).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{IdSalt, Response, Ui};

craft_ui::words! {
    /// The statistics' names, English by default (GridCraft's, Excel's).
    #[non_exhaustive]
    pub struct StatsLabels {
        average: "stats.average" = "Average: {v}";
        count: "stats.count" = "Count: {v}";
        numerical_count: "stats.numericalCount" = "Numerical Count: {v}";
        minimum: "stats.minimum" = "Minimum: {v}";
        maximum: "stats.maximum" = "Maximum: {v}";
        sum: "stats.sum" = "Sum: {v}";
        average_item: "stats.averageItem" = "Average";
        count_item: "stats.countItem" = "Count";
        numerical_count_item: "stats.numericalCountItem" = "Numerical Count";
        minimum_item: "stats.minimumItem" = "Minimum";
        maximum_item: "stats.maximumItem" = "Maximum";
        sum_item: "stats.sumItem" = "Sum";
    }
}

/// A selection's statistics, worked out by the app over its cells.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectionStats {
    /// Non-empty cells.
    pub count: u64,
    /// Cells holding numbers.
    pub numeric: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl SelectionStats {
    /// `None` without numbers.
    pub fn average(&self) -> Option<f64> {
        (self.numeric > 0).then(|| self.sum / self.numeric as f64)
    }
}

/// Which statistics show; one settings section (Excel's status bar menu is app-wide).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatsShown {
    pub average: bool,
    pub count: bool,
    pub numerical_count: bool,
    pub minimum: bool,
    pub maximum: bool,
    pub sum: bool,
}

impl Default for StatsShown {
    /// Excel's: Average, Count and Sum.
    fn default() -> Self {
        Self { average: true, count: true, numerical_count: false, minimum: false, maximum: false, sum: true }
    }
}

/// The status bar's texts, left to right: Average, Count, Numerical Count, Minimum, Maximum,
/// Sum, each when shown and the selection has a value for it (Count with any cell, the others
/// with numbers). `format` formats a number in the active cell's number format.
// gridcraft: crates/ui-egui/src/tabs.rs:441 `selection_statistics`.
pub fn stats_parts(stats: &SelectionStats, shown: &StatsShown, labels: &StatsLabels, format: impl Fn(f64) -> String) -> Vec<String> {
    todo!()
}

/// The statistics in the status bar for a selection of more than one cell, separated by wide
/// spaces; a secondary click opens the menu of check items (one per statistic), which changes
/// `shown` directly. Text (AccessKit labels); the menu items are the Tab stops.
pub struct SelectionStatsReadout<'a> {
    id_salt: IdSalt,
    parts: &'a [String],
    shown: &'a mut StatsShown,
    labels: Option<StatsLabels>,
}

impl<'a> SelectionStatsReadout<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, parts: &'a [String], shown: &'a mut StatsShown) -> Self {
        Self { id_salt: IdSalt::new(id_salt), parts, shown, labels: None }
    }

    pub fn labels(mut self, labels: StatsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // gridcraft: crates/ui-egui/src/tabs.rs:323 `status_bar`, `:479` `status_bar_menu`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
