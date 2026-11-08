//! The viewer's window: the options bar over the panels over the status bar, and the settings it keeps.

use super::options::{options_bar, status_bar};
use super::panels::Panels;
use crate::theme::{ThemeSubject, parse_sizes};
use crate::viewer::{Browser, Catalog, GalleryCommands, GalleryPanel, GallerySettings, Viewer, Workspace, theme_of};
use craft_settings::{LoadReport, Location, SaveStatus, SettingsStore, Source};
use craft_theme::AppSizes;
use craft_ui::contract::{Bind, CommandSet as _, ShowContainer as _};
use craft_ui::docking::tree::FrameRules;
use craft_ui::docking::{PanelLimits, SplitArea};
use craft_ui::layout_components::Stack;
use craft_ui::popover::DropVeil;
use craft_ui::runtime::drag::OsFile;
use craft_ui::runtime::frame::{FrameInput, FrameOutput};
use craft_ui::shell::app::{CloseDecision, ShellApp};
use egui::{Ui, Vec2};
use std::time::Instant;

/// The viewer as the root window's app.
pub struct Gallery {
    viewer: Viewer,
    store: SettingsStore<GallerySettings>,
    /// What loading and saving the settings told that the user should know.
    notes: Vec<String>,
    /// Why the settings could not be written, when they could not.
    save_error: Option<String>,
    /// What came of the files last dropped on the theme workspace.
    drop_note: Option<String>,
}

impl Gallery {
    /// The viewer of the stories linked into the binary, in the state the gallery's `config.toml` holds.
    pub fn open() -> Self {
        let (store, report) = SettingsStore::<GallerySettings>::open(Location::Auto);
        Gallery::new(Catalog::linked(), store, &report)
    }

    /// The viewer of `catalog` over `store`, which `report` describes.
    pub fn new(catalog: Catalog, store: SettingsStore<GallerySettings>, report: &LoadReport) -> Self {
        let viewer = Viewer::new(catalog, store.get());
        Gallery { viewer, store, notes: load_notes(report), save_error: None, drop_note: None }
    }

    /// Writes the settings now.
    fn save(&mut self) {
        self.save_error = self.store.flush().err().map(|error| error.to_string());
    }

    fn status(&self) -> String {
        let browser = &self.viewer.browser;
        let shown = browser.selected_stories().len();
        let total = browser.catalog.entries.len();
        let mut parts = vec![format!("{shown} stories shown of {total}")];
        parts.extend(self.notes.iter().cloned());
        parts.extend(self.save_error.iter().map(|error| format!("Settings were not saved: {error}")));
        parts.extend(self.drop_note.iter().cloned());
        parts.join(". ")
    }
}

/// What opening the settings told that the user should know.
fn load_notes(report: &LoadReport) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(moved) = &report.broken_moved_to {
        notes.push(format!("A damaged settings file was moved aside to {}", moved.display()));
    }
    if let Some(reason) = &report.read_only {
        notes.push(format!("Settings are read-only: {reason}"));
    }
    if let Source::Legacy(from) = &report.source {
        notes.push(format!("Settings were imported from {}", from.display()));
    }
    if !report.section_errors.is_empty() {
        notes.push(format!("{} settings sections were reset to their defaults", report.section_errors.len()));
    }
    notes.extend(report.warning.iter().cloned());
    notes
}

/// The app layer a dropped `sizes.json` holds.
fn read_dropped(file: &OsFile) -> Result<AppSizes, String> {
    let bytes = match (&file.bytes, &file.path) {
        (Some(bytes), _) => bytes.to_vec(),
        (None, Some(path)) => std::fs::read(path).map_err(|error| format!("{}: {error}", file.name))?,
        (None, None) => return Err(format!("{}: the file has no content", file.name)),
    };
    parse_sizes(&file.name, &bytes)
}

impl ShellApp for Gallery {
    fn frame_input(&mut self) -> FrameInput {
        FrameInput::new(self.viewer.env.clone())
    }

    fn output(&mut self, output: FrameOutput) {
        for fired in output.fired {
            if let Some(command) = GalleryCommands::from_ref(&fired.command) {
                self.viewer.run(command);
            }
        }
    }

    fn tick(&mut self, now: Instant) -> Option<Instant> {
        let tick = self.store.tick(now);
        match tick.status {
            SaveStatus::Failed(error) => self.save_error = Some(error.to_string()),
            SaveStatus::Saved => self.save_error = None,
            _ => {}
        }
        for moved in self.store.take_moved_aside() {
            self.notes.push(format!("A damaged settings file was moved aside to {}", moved.display()));
        }
        if self.store.take_merged_change() {
            self.viewer.restore(self.store.get());
        }
        tick.next_deadline
    }

    fn close_requested(&mut self) -> CloseDecision {
        self.save();
        CloseDecision::Close
    }

    fn ui(&mut self, ui: &mut Ui) {
        let status = self.status();
        let Viewer { env, cell_filter, workspace, more_open, browser, themes, theme_dock } = &mut self.viewer;
        let in_theme = *workspace == Workspace::Theme;
        let subject = ThemeSubject::new(theme_of(&env.theme), env.appearance());
        if in_theme {
            themes.refresh(&subject);
        }
        let shown_filter = *cell_filter;
        let Browser { catalog, filter, selection, open, dock, generation, cells } = browser;
        let dock = if in_theme { theme_dock } else { dock };
        let limits = |_: &GalleryPanel| PanelLimits::at_least(Vec2::ZERO);
        let rules = FrameRules::new(&limits);
        let mut panels = Panels {
            catalog,
            filter,
            selection,
            open,
            cells,
            generation: *generation,
            cell_filter: shown_filter,
            themes: &mut *themes,
            subject,
            revealed: None,
        };
        let mut content = |ui: &mut Ui| {
            Stack::new().fill().header(|ui, _| options_bar(ui, env, cell_filter, workspace, more_open)).footer(|ui, _| status_bar(ui, &status)).show(
                ui,
                |ui, _| {
                    SplitArea::new("Panels", Bind::bound(dock), rules).show(ui, |ui, body| {
                        for (group, rect) in body.group_frames::<GalleryPanel>() {
                            panels.show(ui, group, rect);
                        }
                    });
                },
            );
        };
        let dropped = if in_theme {
            DropVeil::new("Drop an app's sizes.json to load it").show(ui, |ui, _| content(ui)).dropped.clone()
        } else {
            content(ui);
            None
        };
        if let Some(file) = dropped.as_ref().and_then(|dropped| dropped.files.first()) {
            self.drop_note = Some(match read_dropped(file) {
                Ok(sizes) => {
                    themes.sizes = sizes;
                    format!("Loaded {}", file.name)
                }
                Err(message) => message,
            });
        }
        self.viewer.sync_sizes();
        if self.store.set(GallerySettings::of(&self.viewer)) {
            // The pass after a change ticks the store, which names when the save is due.
            ui.ctx().request_repaint();
        }
    }
}
