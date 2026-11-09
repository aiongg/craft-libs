//! The selection statistics.

use craft_table::ui::stats::{SelectionStats, StatsShown, StatsWords, stats_parts};
use craft_ui::testing::trial;

fn texts(stats: &SelectionStats, shown: &StatsShown, format: impl Fn(f64) -> String) -> Vec<String> {
    stats_parts(stats, shown, &StatsWords::default(), format).into_iter().map(|part| part.text).collect()
}

/// The parts list the shown statistics in order, numbers only with numbers.
#[trial]
fn parts_follow_shown_and_values() {
    let format = |v: f64| format!("{v:.1}");
    let stats = SelectionStats::new(4, 2, 7.0, 2.0, 5.0);
    assert_eq!(
        texts(&stats, &StatsShown::all(), format),
        ["Average: 3.5", "Count: 4", "Numerical Count: 2", "Minimum: 2.0", "Maximum: 5.0", "Sum: 7.0"]
    );
    assert_eq!(texts(&stats, &StatsShown::default(), format), ["Average: 3.5", "Count: 4", "Sum: 7.0"]);
    let text_only = SelectionStats::new(3, 0, 0.0, 0.0, 0.0);
    assert_eq!(texts(&text_only, &StatsShown::default(), format), ["Count: 3"]);
    assert!(texts(&SelectionStats::default(), &StatsShown::all(), format).is_empty());
}
