//! The selection statistics.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::stats::{SelectionStats, SelectionStatsReadout, StatPart, StatsShown, StatsWords, stats_parts};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::vec2;

fn texts(stats: &SelectionStats, shown: &StatsShown, format: impl Fn(f64) -> String) -> Vec<String> {
    stats_parts(stats, shown, &StatsWords::default(), format).into_iter().map(|part| part.text).collect()
}

fn all() -> StatsShown {
    StatsShown { average: true, count: true, numerical_count: true, minimum: true, maximum: true, sum: true }
}

/// The parts list the shown statistics in order, numbers only with numbers.
#[trial]
fn parts_follow_shown_and_values() {
    let format = |v: f64| format!("{v:.1}");
    let stats = SelectionStats { count: 4, numeric: 2, sum: 7.0, min: 2.0, max: 5.0 };
    assert_eq!(texts(&stats, &all(), format), ["Average: 3.5", "Count: 4", "Numerical Count: 2", "Minimum: 2.0", "Maximum: 5.0", "Sum: 7.0"]);
    assert_eq!(texts(&stats, &StatsShown::default(), format), ["Average: 3.5", "Count: 4", "Sum: 7.0"]);
    let text_only = SelectionStats { count: 3, ..SelectionStats::default() };
    assert_eq!(texts(&text_only, &StatsShown::default(), format), ["Count: 3"]);
    assert!(texts(&SelectionStats::default(), &all(), format).is_empty());
}

/// A click on a statistic copies its value without the label.
#[trial]
fn a_click_on_a_statistic_copies_its_value() {
    let parts = vec![StatPart { text: "Average: 3.5".into(), value: "3.5".into() }, StatPart { text: "Sum: 7.0".into(), value: "7.0".into() }];
    let copied = Rc::new(RefCell::new(None));
    let sink = copied.clone();
    let shown = Rc::new(RefCell::new(StatsShown::default()));
    let state = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut flags = *state.borrow();
        let out = SelectionStatsReadout::new("Statistics", &parts, Bind::bound(&mut flags)).show(ui);
        *state.borrow_mut() = flags;
        if out.copied.is_some() {
            *sink.borrow_mut() = out.copied;
        }
    });
    fixture.room(vec2(400.0, 60.0));
    fixture.settle();
    let second = fixture.by_label("Sum: 7.0").expect("the second statistic");
    fixture.click(&second);
    assert_eq!(copied.borrow().as_deref(), Some("7.0"));
}
