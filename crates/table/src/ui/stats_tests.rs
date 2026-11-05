//! Selection statistics.

use super::stats::{SelectionStats, StatsLabels, StatsShown, stats_parts};

fn texts(stats: &SelectionStats, shown: &StatsShown, format: impl Fn(f64) -> String) -> Vec<String> {
    stats_parts(stats, shown, &StatsLabels::default(), format).into_iter().map(|part| part.text).collect()
}

fn all() -> StatsShown {
    StatsShown { average: true, count: true, numerical_count: true, minimum: true, maximum: true, sum: true }
}

// stats_parts lists the shown statistics in order, numbers only with numbers.
#[test]
fn parts_follow_shown_and_values() {
    let format = |v: f64| format!("{v:.1}");
    let stats = SelectionStats { count: 4, numeric: 2, sum: 7.0, min: 2.0, max: 5.0 };
    assert_eq!(texts(&stats, &all(), format), ["Average: 3.5", "Count: 4", "Numerical Count: 2", "Minimum: 2.0", "Maximum: 5.0", "Sum: 7.0"]);
    assert_eq!(texts(&stats, &StatsShown::default(), format), ["Average: 3.5", "Count: 4", "Sum: 7.0"]);
    let text_only = SelectionStats { count: 3, ..SelectionStats::default() };
    assert_eq!(texts(&text_only, &StatsShown::default(), format), ["Count: 3"]);
    assert!(texts(&SelectionStats::default(), &all(), format).is_empty());
}
