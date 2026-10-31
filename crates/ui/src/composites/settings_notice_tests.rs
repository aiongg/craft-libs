//! `SettingsNotices` over store results.

use std::io;
use std::time::Instant;

use craft_settings::{LoadReport, SaveStatus, SectionError, SettingsError, Source, TickResult};
use egui::Context;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

fn failed() -> TickResult {
    let error = SettingsError::Io(io::Error::other("disk full"));
    TickResult { status: SaveStatus::Failed(error), next_deadline: Some(Instant::now()) }
}

fn saved() -> TickResult {
    TickResult { status: SaveStatus::Saved, next_deadline: None }
}

fn report(sections: &[&str]) -> LoadReport {
    LoadReport {
        dir: None,
        rule: String::new(),
        source: Source::File,
        broken_moved_to: None,
        section_errors: sections.iter().map(|s| SectionError { section: (*s).to_owned(), message: "bad".into() }).collect(),
        migrated_from: None,
        read_only: None,
    }
}

fn harness(notices: SettingsNotices) -> Harness<'static, SettingsNotices> {
    Harness::builder()
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, notices: &mut SettingsNotices| notices.show(ui), notices)
}

fn has(harness: &Harness<'_, SettingsNotices>, text: &str) -> bool {
    harness.query_all_by_label_contains(text).next().is_some()
}

// The first failed save is a status line; the third in a row adds the notice; a success says
// "Preferences saved" and removes it.
#[test]
fn third_failure_raises_the_notice_and_success_clears_it() {
    let ctx = Context::default();
    let mut notices = SettingsNotices::new();
    notices.ticked(&ctx, &failed());
    let status = notices.take_status().expect("status on the first failure");
    assert_eq!(status.tone, Tone::Bad);
    assert_eq!(status.text, "Couldn't save preferences: disk full. Retrying…", "the cause alone, without the error's own lead-in");
    assert!(notices.take_status().is_none(), "the status line is given once");
    assert!(!notices.has_notice());

    notices.ticked(&ctx, &failed());
    assert!(notices.take_status().is_none() && !notices.has_notice());
    notices.ticked(&ctx, &failed());
    assert!(notices.has_notice());
    let mut harness = harness(notices);
    harness.run();
    assert!(has(&harness, "Preferences can't be saved"));
    assert!(has(&harness, "disk full"));

    harness.state_mut().ticked(&ctx, &saved());
    harness.run();
    assert!(!harness.state().has_notice());
    assert!(!has(&harness, "Preferences can't be saved"));
    assert_eq!(harness.state_mut().take_status(), Some(StatusLine { text: "Preferences saved".into(), tone: Tone::Good }));
}

// A load report with section errors lists the reset settings in one notice.
#[test]
fn reset_sections_are_listed() {
    let ctx = Context::default();
    let mut notices = SettingsNotices::new();
    notices.loaded(&ctx, &report(&["interface", "tools"]));
    assert!(notices.has_notice());
    let mut harness = harness(notices);
    harness.run();
    assert!(has(&harness, "Some preferences were reset"));
    assert!(has(&harness, "for interface, tools."));
    harness.state_mut().ticked(&ctx, &saved());
    harness.run();
    assert!(!has(&harness, "Some preferences were reset"));
}
