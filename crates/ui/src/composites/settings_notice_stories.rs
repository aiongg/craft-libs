//! Story of [`SettingsNotices`]: the load and save problems of one app run.

use std::sync::{Arc, Mutex, PoisonError};

use craft_settings::{LoadReport, ReadOnlyReason, SaveStatus, SectionError, SettingsError, Source, TickResult};
use egui::{Context, Ui};

use super::{Notice, SettingsNotices, StatusLine};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Scenario {
    ValuesReset,
    FileMovedAside,
    NewerVersionsFile,
    FirstSaveFails,
    SavesKeepFailing,
    SaveRecovers,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct NoticesArgs {
    scenario: Scenario,
    #[arg(range = 240.0..=640.0)]
    width: f32,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/settings-notices/notices",
        Level::Composites,
        "SettingsNotices",
        "Notices",
        NoticesArgs { scenario: Scenario::SavesKeepFailing, width: 420.0 },
        show_notices
    )]
}

fn report() -> LoadReport {
    LoadReport {
        dir: None,
        rule: "story".to_owned(),
        source: Source::File,
        broken_moved_to: None,
        section_errors: Vec::new(),
        migrated_from: None,
        read_only: None,
    }
}

fn failure(message: &str) -> TickResult {
    TickResult { status: SaveStatus::Failed(SettingsError::Io(std::io::Error::other(message.to_owned()))), next_deadline: None }
}

fn saved() -> TickResult {
    TickResult { status: SaveStatus::Saved, next_deadline: None }
}

/// The notices after `scenario` happened, and the status line it left for the status bar.
fn build(ctx: &Context, scenario: Scenario) -> (SettingsNotices, Option<StatusLine>) {
    let mut notices = SettingsNotices::new();
    match scenario {
        Scenario::ValuesReset => notices.loaded(
            ctx,
            &LoadReport {
                section_errors: vec![
                    SectionError { section: "interface.uiScale".to_owned(), message: "invalid type: string".to_owned() },
                    SectionError { section: "tools.brushSize".to_owned(), message: "invalid value: -4".to_owned() },
                ],
                ..report()
            },
        ),
        Scenario::FileMovedAside => {
            notices.loaded(ctx, &LoadReport { broken_moved_to: Some("config.toml.broken-20261031".into()), ..report() });
        }
        Scenario::NewerVersionsFile => {
            notices.loaded(ctx, &LoadReport { read_only: Some(ReadOnlyReason::NewerSchema { file: 7, app: 5 }), ..report() });
        }
        Scenario::FirstSaveFails => notices.ticked(ctx, &failure("No space left on device")),
        Scenario::SavesKeepFailing => (0..super::SAVE_NOTICE_AFTER).for_each(|_| notices.ticked(ctx, &failure("No space left on device"))),
        Scenario::SaveRecovers => {
            notices.ticked(ctx, &failure("No space left on device"));
            notices.ticked(ctx, &saved());
        }
    }
    let status = notices.take_status();
    (notices, status)
}

type Built = Arc<Mutex<(Scenario, SettingsNotices, Option<StatusLine>)>>;

/// The notices and the status line of the chosen scenario. They are built once per scenario,
/// because building them asks for a repaint.
fn show_notices(ui: &mut Ui, args: &mut NoticesArgs) {
    let key = ui.make_persistent_id("craft_ui::composites::settings-notices-story");
    let built: Built = ui.data(|d| d.get_temp::<Built>(key)).unwrap_or_else(|| {
        let (notices, status) = build(ui.ctx(), args.scenario);
        Arc::new(Mutex::new((args.scenario, notices, status)))
    });
    let mut guard = built.lock().unwrap_or_else(PoisonError::into_inner);
    if guard.0 != args.scenario {
        let (notices, status) = build(ui.ctx(), args.scenario);
        *guard = (args.scenario, notices, status);
    }
    ui.allocate_ui(egui::vec2(args.width.min(ui.available_width()), 0.0), |ui| {
        let (_, notices, status) = &mut *guard;
        if let Some(status) = status {
            let _ = Notice::new(status.text.clone()).tone(status.tone).id_salt("settings-status-line").show(ui);
        }
        notices.show(ui);
        if !notices.has_notice() && status.is_none() {
            ui.label("Nothing to report.");
        }
    });
    drop(guard);
    ui.data_mut(|d| d.insert_temp(key, built));
}
