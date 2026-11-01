//! Selection statistics.

use super::stats::{SelectionStats, StatsLabels, StatsShown, stats_parts};

fn all() -> StatsShown {
    StatsShown { average: true, count: true, numerical_count: true, minimum: true, maximum: true, sum: true }
}

// stats_parts lists the shown statistics in order, numbers only with numbers.
#[test]
fn parts_follow_shown_and_values() {
    let labels = StatsLabels::default();
    let format = |v: f64| format!("{v:.1}");
    let stats = SelectionStats { count: 4, numeric: 2, sum: 7.0, min: 2.0, max: 5.0 };
    assert_eq!(
        stats_parts(&stats, &all(), &labels, format),
        ["Average: 3.5", "Count: 4", "Numerical Count: 2", "Minimum: 2.0", "Maximum: 5.0", "Sum: 7.0"]
    );
    assert_eq!(stats_parts(&stats, &StatsShown::default(), &labels, format), ["Average: 3.5", "Count: 4", "Sum: 7.0"]);
    let text_only = SelectionStats { count: 3, ..SelectionStats::default() };
    assert_eq!(stats_parts(&text_only, &StatsShown::default(), &labels, format), ["Count: 3"]);
    assert!(stats_parts(&SelectionStats::default(), &all(), &labels, format).is_empty());
}
