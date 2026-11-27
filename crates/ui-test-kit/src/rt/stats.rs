//! Median and 95th percentile of a column.

/// Median and nearest-rank 95th percentile.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stat {
    /// The middle value.
    pub median: f64,
    /// The nearest-rank 95th percentile.
    pub p95: f64,
}

/// Empty input gives zeros.
pub fn summarize(values: &[f64]) -> Stat {
    if values.is_empty() {
        return Stat::default();
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let median = if n % 2 == 1 { v[n / 2] } else { f64::midpoint(v[n / 2 - 1], v[n / 2]) };
    let rank = ((0.95 * n as f64).ceil() as usize).clamp(1, n);
    Stat { median, p95: v[rank - 1] }
}
