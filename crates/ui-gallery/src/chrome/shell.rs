//! The viewer's window: the options bar over the panels over the status bar, and the settings it keeps.

use super::options::{options_bar, status_bar};
use super::panels::Panels;
use super::persist::Persisted;
use crate::theme::{ThemeSubject, parse_sizes};
use crate::viewer::{Browser, Catalog, GalleryCommands, GalleryPanel, GallerySettings, Viewer, Workspace, theme_of};
use craft_theme::AppSizes;
use craft_ui::contract::SizeLimits;
use craft_ui::contract::{Bind, CommandSet as _, ShowContainer as _};
use craft_ui::docking::SplitArea;
use craft_ui::docking::tree::FrameRules;
use craft_ui::layout_components::Stack;
use craft_ui::popover::DropVeil;
use craft_ui::runtime::drag::OsFile;
use craft_ui::runtime::frame::{FrameInput, FrameOutput};
use craft_ui::shell::app::{CloseDecision, ShellApp};
use egui::{Ui, Vec2};
use web_time::Instant;

/// The viewer as the root window's app.
pub struct Gallery {
    viewer: Viewer,
    persisted: Persisted,
    /// What came of the files last dropped on the theme workspace.
    drop_note: Option<String>,
    /// A command changed the viewer since the settings were last compared.
    command_ran: bool,
}

impl Gallery {
    /// The viewer of the stories linked into the binary, in the state the gallery's settings hold.
    pub fn open() -> Self {
        Gallery::new(Catalog::linked(), Persisted::open())
    }

    /// The viewer of `catalog` over `persisted`.
    pub fn new(catalog: Catalog, persisted: Persisted) -> Self {
        let viewer = Viewer::new(catalog, persisted.get());
        Gallery { viewer, persisted, drop_note: None, command_ran: false }
    }

    fn status(&self) -> String {
        let browser = &self.viewer.browser;
        let shown = browser.selected_stories().len();
        let total = browser.catalog.entries.len();
        let mut parts = vec![format!("{shown} stories shown of {total}")];
        parts.extend(self.persisted.notes());
        parts.extend(self.drop_note.iter().cloned());
        parts.join(". ")
    }
}

/// Whether an input event can have changed what a control edits: hover, focus and pointer motion alone cannot, the
/// release or key that ends an edit can.
fn is_edit(event: &egui::Event) -> bool {
    !matches!(event, egui::Event::PointerMoved(_) | egui::Event::MouseMoved(_) | egui::Event::PointerGone | egui::Event::WindowFocused(_))
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
                self.command_ran = true;
                self.viewer.run(command);
            }
        }
    }

    fn tick(&mut self, now: Instant) -> Option<Instant> {
        let (deadline, merged) = self.persisted.tick(now);
        if let Some(settings) = merged {
            self.viewer.restore(&settings);
        }
        deadline
    }

    fn close_requested(&mut self) -> CloseDecision {
        self.persisted.save();
        CloseDecision::Close
    }

    fn ui(&mut self, ui: &mut Ui) {
        let status = self.status();
        let Viewer { env, cell_filter, workspace, more_open, browser, themes, theme_dock, .. } = &mut self.viewer;
        let in_theme = *workspace == Workspace::Theme;
        let subject = ThemeSubject::new(theme_of(&env.theme), env.appearance());
        if in_theme {
            themes.refresh(&subject);
        }
        let shown_filter = *cell_filter;
        let Browser { catalog, filter, selection, open, dock, generation, cells } = browser;
        let dock = if in_theme { theme_dock } else { dock };
        let limits = |_: &GalleryPanel| SizeLimits::at_least(Vec2::ZERO);
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
                    themes.edit_sizes(|layer| {
                        *layer = sizes;
                        true
                    });
                    format!("Loaded {}", file.name)
                }
                Err(message) => message,
            });
        }
        self.viewer.sync_sizes();
        // The controls change the viewer only on input, and a command only in `output`: a pass with neither has
        // nothing new to save.
        let edited = std::mem::take(&mut self.command_ran) || ui.ctx().input(|input| input.events.iter().any(is_edit));
        if edited && self.persisted.set(GallerySettings::of(&self.viewer)) {
            // The pass after a change ticks the store, which names when the save is due.
            ui.ctx().request_repaint();
        }
    }
}
