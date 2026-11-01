//! [`histogram_statistics`]: the Histogram panel's Mean, Std Dev, Median, Pixels, Level, Count
//! and Percentile.

/// The statistics of one channel's histogram. Mean, standard deviation, median and pixels describe
/// the whole histogram; level, count and percentile the inspected level or range.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistogramStatistics {
    /// The mean level.
    pub mean: f64,
    /// The standard deviation of the level (Photoshop's Std Dev), over the whole population.
    pub standard_deviation: f64,
    /// The first level at which the running count reaches half the pixels.
    pub median: u8,
    /// The pixels counted.
    pub pixels: u64,
    /// The inspected levels `lo..=hi` (one level when equal), in order; `None` when nothing is
    /// inspected.
    pub level: Option<[u8; 2]>,
    /// Pixels at the inspected levels.
    pub count: Option<u64>,
    /// For one level, percent of the pixels at or below it; for a range, percent in the range.
    pub percentile: Option<f64>,
    /// Percent of the pixels at level 0 and at the last level: shadow and highlight clipping.
    pub clipped: [f64; 2],
}

/// The statistics of `bins` (one count per level, usually 256), inspecting `levels` (a level or a
/// range, either order; clamped to the bins). `None` when nothing was counted.
///
/// Rules (PhotoCraft's `stats`): mean `Σ i·n_i / N`; variance `Σ (i − mean)²·n_i / N`; median the
/// first `i` where the running count × 2 `>= N`; count of `lo..=hi` the sum of those bins;
/// percentile `100 × sum(0..=hi) / N` when `lo == hi`, else `100 × sum(lo..=hi) / N`; clipped
/// `100 × n_0 / N` and `100 × n_last / N`.
#[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub fn histogram_statistics(bins: &[u32], levels: Option<[u8; 2]>) -> Option<HistogramStatistics> {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:266 `stats`.
    todo!()
}
