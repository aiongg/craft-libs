//! The viewer's window: the options bar over the panels over the status bar, and the settings it keeps.

use super::options::{options_bar, status_bar};
use super::panels::Panels;
use crate::viewer::{Browser, Catalog, GalleryCommands, GallerySettings, Viewer};
use craft_settings::{Location, SettingsStore};
use craft_ui::contract::{Bind, CommandSet as _, ShowContainer as _};
use craft_ui::docking::{SplitArea, group_rects};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::frame::{FrameInput, FrameOutput, request_repaint_after};
use craft_ui::shell::app::{CloseDecision, ShellApp};
use egui::{Context, Ui};

/// Passes without a change after which the settings are written.
const SETTLED_PASSES: u64 = 30;
/// Seconds until the pass that writes settings a change left unwritten.
const SAVE_WAKE: f64 = 0.75;

/// The viewer as the root window's app.
pub struct Gallery {
    viewer: Viewer,
    store: SettingsStore<GallerySettings>,
    /// The pass of the latest change to the settings.
    changed_pass: u64,
    /// Why the settings could not be written, when they could not.
    save_error: Option<String>,
}

impl Gallery {
    /// The viewer of the stories linked into the binary, in the state the gallery's `config.toml` holds.
    pub fn open() -> Self {
        let (store, _) = SettingsStore::<GallerySettings>::open(Location::Auto);
        let viewer = Viewer::new(Catalog::linked(), store.get());
        Gallery { viewer, store, changed_pass: 0, save_error: None }
    }

    /// Writes the settings now.
    fn save(&mut self) {
        self.save_error = self.store.flush().err().map(|error| error.to_string());
    }

    fn status(&self) -> String {
        let browser = &self.viewer.browser;
        let shown = browser.selected_stories().len();
        let total = browser.catalog.entries.len();
        match &self.save_error {
            Some(error) => format!("{shown} stories shown of {total}. Settings were not saved: {error}"),
            None => format!("{shown} stories shown of {total}"),
        }
    }
}

impl ShellApp for Gallery {
    fn frame_input(&mut self) -> FrameInput {
        FrameInput::new(self.viewer.appearance.root_env())
    }

    fn output(&mut self, output: FrameOutput) {
        for fired in output.fired {
            if let Some(command) = GalleryCommands::from_ref(&fired.command) {
                self.viewer.run(command);
            }
        }
    }

    fn logic(&mut self, ctx: &Context) {
        if self.store.is_dirty() && ctx.cumulative_pass_nr() >= self.changed_pass + SETTLED_PASSES {
            self.save();
        }
    }

    fn close_requested(&mut self) -> CloseDecision {
        self.save();
        CloseDecision::Close
    }

    fn ui(&mut self, ui: &mut Ui) {
        let status = self.status();
        let Viewer { appearance, more_open, browser } = &mut self.viewer;
        let shown_appearance = *appearance;
        let Browser { catalog, filter, selection, open, dock, generation, cells } = browser;
        let layout = dock.clone();
        let mut panels = Panels { catalog, filter, selection, open, cells, generation: *generation, appearance: shown_appearance };
        Stack::new().fill().header(|ui, _| options_bar(ui, appearance, more_open)).footer(|ui, _| status_bar(ui, &status)).show(ui, |ui, _| {
            SplitArea::new("Panels", Bind::bound(dock)).show(ui, |ui, body| {
                for (group, rect) in group_rects(&layout, body.rect, body.env.direction().is_rtl()) {
                    panels.show(ui, group, rect);
                }
            });
        });
        if self.store.set(GallerySettings::of(&self.viewer)) {
            self.changed_pass = ui.ctx().cumulative_pass_nr();
        }
        if self.store.is_dirty() {
            request_repaint_after(ui.ctx(), SAVE_WAKE);
        }
    }
}
