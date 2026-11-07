//! Story viewer over the registration list of craft-ui's stories.
//!
//! Every registered story is shown in each direction and control size at once, under the chosen theme. The
//! viewer holds no per-story code: a story registered by `#[story]` appears here when its crate is linked.

use craft_i18n::Direction;
use craft_theme::{ControlSize, Mode};
use craft_ui::contract::{Bind, Props as _, Scroll, Show as _, ShowContainer as _};
use craft_ui::controls::radio::{Radio, RadioGroup};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::runtime::env::{RootEnv, with_env};
use craft_ui::runtime::frame::FrameInput;
use craft_ui::shell::app::{AppSpec, ShellApp, run};
use craft_ui::testing::{STORIES, StoryData, StoryRegistration, registered};
use craft_window::restore::Defaults;
use egui::{Ui, Vec2};
use std::collections::HashMap;

const WINDOW: [f32; 2] = [1100.0, 800.0];
const WINDOW_MIN: [f32; 2] = [640.0, 480.0];
const SIDE_WIDTH: f32 = 220.0;
const CELL_WIDTH: f32 = 260.0;
const DIRECTIONS: [Direction; 2] = [Direction::Ltr, Direction::Rtl];
const SIZES: [ControlSize; 2] = [ControlSize::Sm, ControlSize::Md];

/// One registered story with the facts the list shows.
struct Entry {
    registration: &'static StoryRegistration,
    type_name: &'static str,
    module: &'static str,
}

impl Entry {
    /// The type's name without its module path, type arguments kept.
    fn short_type(&self) -> String {
        let (path, arguments) = self.type_name.split_once('<').map_or((self.type_name, ""), |(path, rest)| (path, rest));
        let name = path.rsplit("::").next().unwrap_or(path);
        if arguments.is_empty() { name.to_owned() } else { format!("{name}<{arguments}") }
    }

    fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        filter.is_empty() || [self.type_name, self.module, self.registration.story.name].iter().any(|text| text.to_lowercase().contains(&filter))
    }
}

/// Every story linked into the viewer, ordered by type then story name.
fn entries() -> Vec<Entry> {
    let modules: HashMap<&'static str, &'static str> =
        registered().iter().filter_map(|registration| registration.probe()).map(|probe| (probe.type_name(), probe.module)).collect();
    let mut entries: Vec<_> = STORIES
        .iter()
        .map(|registration| {
            let type_name = (registration.type_name)();
            Entry { registration, type_name, module: modules.get(type_name).copied().unwrap_or("") }
        })
        .collect();
    entries.sort_by_key(|entry| (entry.type_name, entry.registration.story.name));
    entries
}

/// A group of radio buttons that sets `value` to the chosen option.
fn choose<T: Clone + PartialEq + Send + Sync + 'static>(ui: &mut Ui, label: &'static str, value: &mut T, options: &'static [(T, &'static str)]) {
    RadioGroup::new(label, Bind::bound(value)).show(ui, |ui, _| {
        for (option, name) in options {
            Radio::new(option.clone()).label(*name).show(ui);
        }
    });
}

fn label_of(direction: Direction, size: ControlSize) -> String {
    let direction = if direction.is_rtl() { "RTL" } else { "LTR" };
    format!("{direction}, {size:?}")
}

struct Viewer {
    entries: Vec<Entry>,
    filter: String,
    mode: Mode,
    /// `None` shows both.
    direction: Option<Direction>,
    size: Option<ControlSize>,
    /// The values each shown cell keeps between frames, by entry index, direction and size.
    data: HashMap<(usize, Direction, ControlSize), StoryData>,
}

impl Viewer {
    fn new() -> Self {
        Viewer { entries: entries(), filter: String::new(), mode: Mode::Light, direction: None, size: None, data: HashMap::new() }
    }

    fn side(&mut self, ui: &mut Ui) {
        ui.label("Filter");
        ui.text_edit_singleline(&mut self.filter);
        choose(ui, "Theme", &mut self.mode, &[(Mode::Light, "Light"), (Mode::Dark, "Dark")]);
        choose(ui, "Direction", &mut self.direction, &[(None, "Both"), (Some(Direction::Ltr), "LTR"), (Some(Direction::Rtl), "RTL")]);
        choose(ui, "Control size", &mut self.size, &[(None, "Both"), (Some(ControlSize::Sm), "Sm"), (Some(ControlSize::Md), "Md")]);
        let shown = self.entries.iter().filter(|entry| entry.matches(&self.filter)).count();
        ui.label(format!("{shown} of {} stories", self.entries.len()));
    }

    fn stories(&mut self, ui: &mut Ui) {
        if self.entries.is_empty() {
            ui.label("No stories are registered.");
            return;
        }
        let directions: Vec<_> = DIRECTIONS.into_iter().filter(|d| self.direction.is_none_or(|only| only == *d)).collect();
        let sizes: Vec<_> = SIZES.into_iter().filter(|s| self.size.is_none_or(|only| only == *s)).collect();
        let (entries, filter, data) = (&self.entries, &self.filter, &mut self.data);
        Stack::new().scroll(Scroll::vertical()).fill().show(ui, |ui, _| {
            for (index, entry) in entries.iter().enumerate().filter(|(_, entry)| entry.matches(filter)) {
                ui.heading(format!("{}: {}", entry.short_type(), entry.registration.story.name));
                if !entry.module.is_empty() {
                    ui.weak(entry.module);
                }
                ui.horizontal_wrapped(|ui| {
                    for direction in &directions {
                        for size in &sizes {
                            let cell = data.entry((index, *direction, *size)).or_default();
                            Self::cell(ui, entry, *direction, *size, cell);
                        }
                    }
                });
            }
        });
    }

    fn cell(ui: &mut Ui, entry: &Entry, direction: Direction, size: ControlSize, data: &mut StoryData) {
        ui.vertical(|ui| {
            ui.set_width(CELL_WIDTH);
            ui.small(label_of(direction, size));
            with_env(
                ui,
                |env| {
                    env.direction = Some(direction);
                    env.size = Some(size);
                },
                |ui| {
                    ui.set_min_size(Vec2::ZERO);
                    (entry.registration.story.show)(ui, data);
                },
            );
        });
    }
}

struct App {
    viewer: Viewer,
}

impl ShellApp for App {
    fn frame_input(&mut self) -> FrameInput {
        FrameInput::new(RootEnv::new().mode(self.viewer.mode))
    }

    fn ui(&mut self, ui: &mut Ui) {
        let viewer = &mut self.viewer;
        Row::new().show(ui, |ui, _| {
            ui.vertical(|ui| {
                ui.set_width(SIDE_WIDTH);
                viewer.side(ui);
            });
            viewer.stories(ui);
        });
    }
}

fn main() -> eframe::Result {
    let spec = AppSpec::new("craft-ui-gallery", "craft-ui stories", Defaults::new(WINDOW, WINDOW_MIN));
    run(spec, |_| App { viewer: Viewer::new() })
}
