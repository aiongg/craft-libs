//! `FindingsList` in a harness.

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

fn finding(level: FindingLevel, message: &str, page: Option<&str>, target: Option<u64>) -> Finding {
    Finding { level, message: message.into(), page: page.map(Into::into), target }
}

fn sample() -> Vec<Finding> {
    vec![
        finding(FindingLevel::Error, "Missing font: Minion Pro Bold", Some("3"), Some(7)),
        finding(FindingLevel::Warning, "photo.jpg: effective 96 ppi (< 150)", Some("A-2"), Some(9)),
        finding(FindingLevel::Warning, "Overset text", None, None),
    ]
}

// The summary reads "No errors" for an empty list and both counts otherwise.
#[test]
fn summary_counts_errors_and_warnings() {
    let labels = FindingsLabels::default();
    assert_eq!(findings_summary(&[], &labels), "No errors");
    assert_eq!(findings_summary(&sample(), &labels), "1 error, 2 warnings");
    let warnings = &sample()[1..];
    assert_eq!(findings_summary(warnings, &labels), "0 errors, 2 warnings");
    assert_eq!(findings_summary(&sample()[..1], &labels), "1 error, 0 warnings");
}

// A click or Enter on a row reports its index; arrows move between rows from one Tab stop.
#[test]
fn activating_a_row_reports_it() {
    let mut harness = Harness::<(Vec<Finding>, Vec<usize>)>::builder()
        .with_size([320.0, 300.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (findings, activated)| {
                if let Some(at) = FindingsList::new("findings", findings).show(ui).activated {
                    activated.push(at);
                }
            },
            (sample(), Vec::new()),
        );
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "Error: Missing font: Minion Pro Bold, page 3");
    harness.get_by_role_and_label(Role::ListBoxOption, "Warning: photo.jpg: effective 96 ppi (< 150), page A-2").click();
    harness.run();
    assert_eq!(harness.state().1, [1], "a click reports the row's index");

    // The list is one Tab stop: the arrow moves the lead to the third row, which has no item and
    // no page, so Enter on it reports nothing; Up and Enter on the second reports it.
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().1, [1], "a finding without item and page does nothing");
    harness.key_press(Key::ArrowUp);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().1, [1, 1]);
}
