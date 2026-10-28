//! The gallery app.

use std::collections::HashMap;

use craft_theme::{Mode, TextSize, builtins};
use craft_ui::ComboBox;
use craft_ui::direction;
use craft_ui::story::{ArgValue, Args, Level, Story, registry};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, scope};
use egui::{CentralPanel, Frame, Margin, Panel, ScrollArea, Ui};

use crate::appearance::{Appearance, SCALE, builtin};
use crate::faces;
use crate::persist::{self, KEY, Saved};

/// The gallery's state.
pub struct Gallery {
    stories: Vec<Story>,
    appearance: Appearance,
    /// The appearance `base` and the context's direction and zoom were set for.
    applied: Option<Appearance>,
    base: Base,
    selected: Option<String>,
    filter: String,
    compare: bool,
    compare_themes: Vec<String>,
    /// Edited args by story id; a story gets its entry when first shown.
    args: HashMap<String, Args>,
    saved_args: HashMap<String, Vec<persist::SavedArg>>,
}

impl Gallery {
    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        let saved = Saved::load(storage);
        let mut stories = registry();
        stories.extend(faces::stories());
        let selected = saved.selected.filter(|id| stories.iter().any(|s| s.id == id)).or_else(|| stories.first().map(|s| s.id.to_owned()));
        let mut compare_themes: Vec<String> = saved.compare_themes.into_iter().filter(|id| builtin(id).is_some()).collect();
        if compare_themes.is_empty() {
            compare_themes.push(saved.appearance.theme.clone());
        }
        Self {
            stories,
            base: saved.appearance.base(),
            appearance: saved.appearance,
            applied: None,
            selected,
            filter: saved.filter,
            compare: saved.compare,
            compare_themes,
            args: HashMap::new(),
            saved_args: saved.args,
        }
    }

    /// Selects a story by id; unknown ids are ignored.
    pub fn select(&mut self, id: &str) {
        if self.stories.iter().any(|s| s.id == id) {
            self.selected = Some(id.to_owned());
        }
    }

    pub fn set_compare(&mut self, compare: bool) {
        self.compare = compare;
    }

    pub fn stories(&self) -> &[Story] {
        &self.stories
    }

    pub fn appearance_mut(&mut self) -> &mut Appearance {
        &mut self.appearance
    }

    /// Draws one frame of the gallery.
    pub fn show(&mut self, ui: &mut Ui) {
        self.apply_appearance(ui.ctx());
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        Panel::top("gallery.bar").show(ui, |ui| self.appearance_bar(ui));
        direction::start_panel(&ctx, "gallery.tree").default_size(240.0).show(ui, |ui| self.tree(ui));
        direction::end_panel(&ctx, "gallery.args").default_size(240.0).show(ui, |ui| self.args_panel(ui));
        let frame = Frame::new().fill(tokens.color.surface.canvas.to_egui()).inner_margin(Margin::same(12));
        CentralPanel::default().frame(frame).show(ui, |ui| self.centre(ui));
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        if self.applied.as_ref() != Some(&self.appearance) {
            let previous = self.applied.replace(self.appearance.clone());
            if previous.as_ref().is_none_or(|p| p.rtl != self.appearance.rtl) {
                direction::apply(ctx, self.appearance.direction());
            }
            if faces::available()
                && previous.as_ref().is_none_or(|p| p.ui_face != self.appearance.ui_face || p.mono_face != self.appearance.mono_face)
            {
                faces::install(ctx, &self.appearance.ui_face, &self.appearance.mono_face);
            }
            ctx.set_zoom_factor(self.appearance.zoom());
            self.base = self.appearance.base();
        }
        install(ctx, &self.base);
    }

    fn appearance_bar(&mut self, ui: &mut Ui) {
        let a = &mut self.appearance;
        ui.horizontal_wrapped(|ui| {
            let themes = builtins();
            let current = builtin(&a.theme).map_or(a.theme.as_str(), |t| t.name.as_str()).to_owned();
            ui.label("Theme");
            ComboBox::from_id_salt("gallery.theme").selected_text(current).show_ui(ui, |ui| {
                for theme in themes {
                    ui.selectable_value(&mut a.theme, theme.id.as_str().to_owned(), &theme.name);
                }
            });
            segmented(ui, "Mode", &mut a.dark, [("Light", false), ("Dark", true)]);
            segmented(ui, "Contrast", &mut a.high_contrast, [("Standard", false), ("High", true)]);
            segmented(ui, "Size", &mut a.comfortable, [("Compact", false), ("Comfortable", true)]);
            ui.label("Text");
            for (i, size) in TextSize::ALL.iter().enumerate() {
                ui.selectable_value(&mut a.text_size, i, text_size_name(*size));
            }
            ui.checkbox(&mut a.reduced_motion, "Reduced motion");
            segmented(ui, "Direction", &mut a.rtl, [("LTR", false), ("RTL", true)]);
            if faces::available() {
                face_picker(ui, "UI face", "gallery.ui_face", &mut a.ui_face, faces::UI_FACES);
                face_picker(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, faces::MONO_FACES);
            }
            ui.label("Scale");
            ui.add(egui::Slider::new(&mut a.scale, SCALE).suffix("%"));
        });
    }

    fn tree(&mut self, ui: &mut Ui) {
        ui.text_edit_singleline(&mut self.filter);
        let filter = self.filter.to_lowercase();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for level in Level::ALL {
                let mut components: Vec<&'static str> = Vec::new();
                for story in self.stories.iter().filter(|s| s.level == level && matches(s, &filter)) {
                    if !components.contains(&story.component) {
                        components.push(story.component);
                    }
                }
                if components.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(level.title()).id_salt(("gallery.level", level.slug())).default_open(true).show(ui, |ui| {
                    for component in components {
                        ui.collapsing(component, |ui| {
                            for story in self.stories.iter().filter(|s| s.level == level && s.component == component && matches(s, &filter)) {
                                let is_selected = self.selected.as_deref() == Some(story.id);
                                if ui.selectable_label(is_selected, story.name).clicked() {
                                    self.selected = Some(story.id.to_owned());
                                }
                            }
                        });
                    }
                });
            }
        });
    }

    fn current(&self) -> Option<Story> {
        let id = self.selected.as_deref()?;
        self.stories.iter().find(|s| s.id == id).copied()
    }

    /// The args of `story`, from the saved values on first use.
    fn args_of(&mut self, story: &Story) -> &mut Args {
        let saved = &self.saved_args;
        self.args.entry(story.id.to_owned()).or_insert_with(|| match saved.get(story.id) {
            Some(values) => persist::restore_args(story.defaults(), values),
            None => story.defaults(),
        })
    }

    fn args_panel(&mut self, ui: &mut Ui) {
        ui.heading("Args");
        let Some(story) = self.current() else {
            return;
        };
        if ui.button("Reset args").clicked() {
            self.args.insert(story.id.to_owned(), story.defaults());
            self.saved_args.remove(story.id);
        }
        ui.separator();
        let args = self.args_of(&story);
        egui::Grid::new(("gallery.args", story.id)).num_columns(2).show(ui, |ui| {
            for arg in args.iter_mut() {
                ui.label(arg.name);
                match &mut arg.value {
                    ArgValue::Bool(b) => {
                        ui.checkbox(b, "");
                    }
                    ArgValue::Choice { options, index } => {
                        let shown = options.get(*index).copied().unwrap_or("");
                        ComboBox::from_id_salt(arg.name).selected_text(shown).show_ui(ui, |ui| {
                            for (i, option) in options.iter().enumerate() {
                                ui.selectable_value(index, i, *option);
                            }
                        });
                    }
                    ArgValue::Text(t) => {
                        ui.text_edit_singleline(t);
                    }
                    ArgValue::Number { value, min, max } => {
                        ui.add(egui::DragValue::new(value).range(*min..=*max).speed((*max - *min) / 200.0));
                    }
                }
                ui.end_row();
            }
        });
    }

    fn centre(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.compare, false, "Canvas");
            ui.selectable_value(&mut self.compare, true, "Compare");
            if self.compare {
                ui.menu_button("Themes", |ui| {
                    for theme in builtins() {
                        let id = theme.id.as_str();
                        let mut on = self.compare_themes.iter().any(|t| t == id);
                        if ui.checkbox(&mut on, &theme.name).changed() {
                            if on {
                                self.compare_themes.push(id.to_owned());
                            } else {
                                self.compare_themes.retain(|t| t != id);
                            }
                        }
                    }
                });
            }
        });
        ui.separator();
        let Some(story) = self.current() else {
            ui.label("No story selected.");
            return;
        };
        let appearance = self.appearance.clone();
        let themes = self.compare_themes.clone();
        let compare = self.compare;
        let args = self.args_of(&story);
        ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            if compare {
                compare_grid(ui, &story, args, &appearance, &themes);
            } else {
                canvas(ui, &story, args, &appearance);
            }
        });
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut saved_args = self.saved_args.clone();
        for (id, args) in &self.args {
            saved_args.insert(id.clone(), persist::save_args(args));
        }
        let saved = Saved {
            appearance: self.appearance.clone(),
            selected: self.selected.clone(),
            filter: self.filter.clone(),
            compare: self.compare,
            compare_themes: self.compare_themes.clone(),
            args: saved_args,
        };
        eframe::set_value(storage, KEY, &saved);
    }
}

fn matches(story: &Story, filter: &str) -> bool {
    filter.is_empty() || story.id.contains(filter) || story.component.to_lowercase().contains(filter) || story.name.to_lowercase().contains(filter)
}

fn text_size_name(size: TextSize) -> &'static str {
    match size {
        TextSize::Small => "Small",
        TextSize::Default => "Default",
        TextSize::Large => "Large",
        TextSize::Larger => "Larger",
    }
}

fn face_picker(ui: &mut Ui, label: &str, id: &str, value: &mut String, options: &[&str]) {
    ui.label(label);
    ComboBox::from_id_salt(id).selected_text(value.as_str()).show_ui(ui, |ui| {
        for option in options {
            ui.selectable_value(value, (*option).to_owned(), *option);
        }
    });
}

/// A labelled pair of exclusive choices.
fn segmented(ui: &mut Ui, label: &str, value: &mut bool, options: [(&str, bool); 2]) {
    ui.label(label);
    for (name, option) in options {
        ui.selectable_value(value, option, name);
    }
}

/// The story on `surface.background` under the bar's appearance.
fn canvas(ui: &mut Ui, story: &Story, args: &mut Args, appearance: &Appearance) {
    scope(ui, |o| appearance.overrides(o), |ui| framed(ui, story, args));
}

fn framed(ui: &mut Ui, story: &Story, args: &mut Args) {
    let fill = Tokens::of(ui).color.surface.background.to_egui();
    Frame::new().fill(fill).inner_margin(Margin::same(16)).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.push_id(story.id, |ui| (story.show)(ui, args));
    });
}

/// Rows: the chosen themes. Columns: light and dark, each in standard and high contrast.
fn compare_grid(ui: &mut Ui, story: &Story, args: &mut Args, appearance: &Appearance, themes: &[String]) {
    const COLUMNS: [(Mode, bool); 4] = [(Mode::Light, false), (Mode::Light, true), (Mode::Dark, false), (Mode::Dark, true)];
    for id in themes {
        let Some(theme) = builtin(id) else {
            continue;
        };
        ui.heading(&theme.name);
        ui.columns(COLUMNS.len(), |columns| {
            for (ui, (mode, high)) in columns.iter_mut().zip(COLUMNS) {
                let name = format!("{} {}", if mode == Mode::Dark { "Dark" } else { "Light" }, if high { "high contrast" } else { "standard" });
                ui.push_id((id, name.as_str()), |ui| {
                    scope(
                        ui,
                        |o| {
                            appearance.overrides(o);
                            o.theme = Some(theme.clone());
                            o.mode = Some(mode);
                            o.contrast = Some(if high { craft_theme::Contrast::High } else { craft_theme::Contrast::Standard });
                        },
                        |ui| {
                            ui.label(name.as_str());
                            framed(ui, story, args);
                        },
                    );
                });
            }
        });
        ui.add_space(12.0);
    }
}
