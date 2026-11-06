//! The result types, the JSON baseline and the printed table.

use std::path::Path;

use serde::Serialize;

/// The numbers of one scenario over the measured frames.
#[derive(Clone, Debug, Serialize)]
pub struct Measurement {
    /// Median frame time in milliseconds.
    pub frame_ms_median: f64,
    /// 95th-percentile frame time in milliseconds.
    pub frame_ms_p95: f64,
    /// Median allocations per frame.
    pub allocations_median: f64,
    /// Median bytes requested per frame.
    pub bytes_median: f64,
    /// Mean discarded passes per frame.
    pub discard_passes_per_frame: f64,
    /// Frames that discarded at least one pass.
    pub frames_with_discard: usize,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let at = ((sorted.len() as f64 * p).ceil() as usize).clamp(1, sorted.len()) - 1;
    sorted[at]
}

fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
}

impl Measurement {
    pub(crate) fn from_samples(frame_ms: Vec<f64>, allocations: Vec<f64>, bytes: Vec<f64>, discards: Vec<usize>) -> Self {
        let (frame_ms, allocations, bytes) = (sorted(frame_ms), sorted(allocations), sorted(bytes));
        Measurement {
            frame_ms_median: round(percentile(&frame_ms, 0.5)),
            frame_ms_p95: round(percentile(&frame_ms, 0.95)),
            allocations_median: percentile(&allocations, 0.5),
            bytes_median: percentile(&bytes, 0.5),
            discard_passes_per_frame: round(discards.iter().sum::<usize>() as f64 / discards.len() as f64),
            frames_with_discard: discards.iter().filter(|&&d| d > 0).count(),
        }
    }
}

/// One scenario's result.
#[derive(Clone, Debug, Serialize)]
pub struct ScenarioResult {
    /// The scenario's name.
    pub name: String,
    /// The spec's text for it.
    pub description: String,
    /// Whether the spec asserts it runs no discard pass.
    pub no_discard: bool,
    /// What the library used for it; `None` when it has no equivalent.
    pub equivalent: Option<String>,
    /// Why the library has no equivalent.
    pub no_equivalent_reason: Option<String>,
    /// The numbers; `None` when it has no equivalent.
    pub measurement: Option<Measurement>,
}

/// A library's whole run.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// The library's name.
    pub library: String,
    /// Frames run before measuring.
    pub warmup_frames: u64,
    /// Frames measured per scenario.
    pub measured_frames: u64,
    /// The scenarios, in [`SCENARIOS`](crate::SCENARIOS) order.
    pub scenarios: Vec<ScenarioResult>,
}

/// Writes `report` as pretty JSON to `path`.
pub fn write_json(report: &Report, path: &Path) -> std::io::Result<()> {
    let mut text = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    text.push('\n');
    std::fs::write(path, text)
}

/// Prints the report as a table.
pub fn print_table(report: &Report) {
    println!("{} ({} warm-up, {} measured frames)", report.library, report.warmup_frames, report.measured_frames);
    println!("{:<34} {:>10} {:>10} {:>12} {:>12} {:>9}", "scenario", "median ms", "p95 ms", "allocs/frame", "KiB/frame", "discards");
    for scenario in &report.scenarios {
        let Some(m) = &scenario.measurement else {
            println!("{:<34} no equivalent: {}", scenario.name, scenario.no_equivalent_reason.as_deref().unwrap_or(""));
            continue;
        };
        let discards =
            if scenario.no_discard { format!("{:.3}", m.discard_passes_per_frame) } else { format!("({:.3})", m.discard_passes_per_frame) };
        println!(
            "{:<34} {:>10.3} {:>10.3} {:>12.0} {:>12.1} {:>9}",
            scenario.name,
            m.frame_ms_median,
            m.frame_ms_p95,
            m.allocations_median,
            m.bytes_median / 1024.0,
            discards
        );
    }
    println!("discards: passes discarded per frame; in parentheses where the spec does not assert it");
}
