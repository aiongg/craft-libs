//! `LinkMonitor` and `SearchRun` over a fake `LinkFiles` and `ManualJobs`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A file changes once: one LinkChange Ok → Modified, and nothing on the following passes.
#[test]
#[ignore = "skeleton"]
fn the_monitor_reports_a_change_once() {
    todo!()
}

// `window_focused` checks every link again; unchanged links report nothing.
#[test]
#[ignore = "skeleton"]
fn a_focus_check_reports_only_changed_links() {
    todo!()
}

// Cancel during the run: `take_updates` is empty afterwards and `end` is Cancelled.
#[test]
#[ignore = "skeleton"]
fn a_cancelled_search_run_reports_nothing_more() {
    todo!()
}
