//! The gallery app.
#![allow(deprecated, reason = "the gallery's own pickers use craft_ui::ComboBox until they move to craft_ui::controls::Select")]

use std::collections::HashMap;
use std::sync::Arc;

use craft_theme::{Mode, TextSize, TokenRef, builtins};
use craft_ui::ComboBox;
use craft_ui::direction;
use craft_ui::icons::{self, IconSet};
use craft_ui::inspect;
use craft_ui::story::{ArgValue, Args, Level, Story, registry};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, scope};
use egui::containers::menu::{MenuButton, MenuConfig};
use egui::{CentralPanel, Frame, Margin, Panel, PopupCloseBehavior, ScrollArea, Ui};

use crate::appearance::{Appearance, SCALE, builtin};
use crate::browser::Browser;
use crate::faces;
use crate::inspector::Inspector;
use crate::persist::{self, KEY, Saved};

/// The gallery's state.
pub struct Gallery {
    stories: Vec<Story>,
    appearance: Appearance,
    /// The appearance `base` and the context's direction and zoom were set for.
    applied: Option<Appearance>,
    /// Whether the OS's interface face had resolved when the faces were last installed.
    system_face_applied: bool,
    base: Base,
    selected: Option<String>,
    filter: String,
    compare: bool,
    compare_themes: Vec<String>,
    compare_high: bool,
    /// The token browser instead of the stories.
    tokens: bool,
    browser: Browser,
    /// The inspector over the story.
    inspect: bool,
    inspector: Inspector,
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
            system_face_applied: false,
            selected,
            filter: saved.filter,
            compare: saved.compare,
            compare_themes,
            compare_high: saved.compare_high,
            tokens: saved.tokens,
            browser: Browser::default(),
            inspect: saved.inspect,
            inspector: Inspector::default(),
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

    pub fn set_compare_high(&mut self, high: bool) {
        self.compare_high = high;
    }

    pub fn set_compare_themes(&mut self, ids: &[&str]) {
        self.compare_themes = ids.iter().filter(|id| builtin(id).is_some()).map(|id| (*id).to_owned()).collect();
    }

    /// Shows the token browser instead of the stories.
    pub fn set_tokens(&mut self, tokens: bool) {
        self.tokens = tokens;
    }

    /// Draws the inspector over the story.
    pub fn set_inspect(&mut self, inspect: bool) {
        self.inspect = inspect;
    }

    /// The labels of the focus-order badges of each canvas the inspector drew in the last frame.
    pub fn focus_badges(&self) -> Vec<Vec<String>> {
        self.inspector.cells().iter().map(|c| c.badges.clone()).collect()
    }

    /// Selects a row of the token browser.
    pub fn select_token(&mut self, token: TokenRef) {
        self.browser.select(token);
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
        let inspecting = self.inspect && !self.tokens;
        inspect::set_enabled(&ctx, inspecting);
        if self.tokens {
            self.browser.show(ui, &self.base, self.appearance.mode());
            return;
        }
        direction::start_panel(&ctx, "gallery.tree").default_size(240.0).show(ui, |ui| self.tree(ui));
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
            direction::end_panel(&ctx, "gallery.inspect").default_size(320.0).show(ui, |ui| self.inspector.panel(ui));
        } else {
            direction::end_panel(&ctx, "gallery.args").default_size(240.0).show(ui, |ui| self.args_panel(ui));
        }
        let frame = Frame::new().fill(tokens.color.surface.canvas.to_egui()).inner_margin(Margin::same(12));
        CentralPanel::default().frame(frame).show(ui, |ui| self.centre(ui, inspecting));
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        let system_face = faces::system_face(ctx).is_some();
        if self.applied.as_ref() != Some(&self.appearance) || system_face != self.system_face_applied {
            let previous = self.applied.replace(self.appearance.clone());
            if previous.as_ref().is_none_or(|p| p.rtl != self.appearance.rtl) {
                direction::apply(ctx, self.appearance.direction());
            }
            let faces_changed = previous.as_ref().is_none_or(|p| p.ui_face != self.appearance.ui_face || p.mono_face != self.appearance.mono_face);
            if faces::available() && (faces_changed || system_face != self.system_face_applied) {
                faces::install(ctx, &self.appearance.ui_face, &self.appearance.mono_face);
            }
            self.system_face_applied = system_face;
            if previous.as_ref().is_none_or(|p| p.icon_set != self.appearance.icon_set) {
                icons::set_active(ctx, self.appearance.icon_set());
            }
            ctx.set_zoom_factor(self.appearance.zoom());
            self.base = self.appearance.base();
        }
        install(ctx, &self.base);
    }

    fn appearance_bar(&mut self, ui: &mut Ui) {
        let a = &mut self.appearance;
        let tokens = &mut self.tokens;
        let inspect = &mut self.inspect;
        ui.horizontal_wrapped(|ui| {
            ui.toggle_value(tokens, "Tokens");
            ui.add_enabled_ui(!*tokens, |ui| ui.toggle_value(inspect, "Inspect"));
            ui.separator();
            let themes = builtins();
            let current = builtin(&a.theme).map_or(a.theme.as_str(), |t| t.name.as_str()).to_owned();
            let label = ui.label("Theme");
            ComboBox::from_id_salt("gallery.theme")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for theme in themes {
                        ui.selectable_value(&mut a.theme, theme.id.as_str().to_owned(), &theme.name);
                    }
                })
                .response
                .labelled_by(label.id);
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
                let ui_shown = faces::shown_ui_face(ui.ctx(), &a.ui_face);
                face_picker(ui, "UI face", "gallery.ui_face", &mut a.ui_face, ui_shown, &faces::ui_faces(ui.ctx()));
                let mono_shown = faces::shown_mono_face(&a.mono_face);
                face_picker(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, mono_shown, faces::MONO_FACES);
            }
            let label = ui.label("Icons");
            ComboBox::from_id_salt("gallery.icon_set")
                .selected_text(a.icon_set().title())
                .show_ui(ui, |ui| {
                    for set in IconSet::ALL {
                        ui.selectable_value(&mut a.icon_set, set.id().to_owned(), set.title());
                    }
                })
                .response
                .labelled_by(label.id);
            let label = ui.label("Scale");
            ui.add(egui::Slider::new(&mut a.scale, SCALE).suffix("%")).labelled_by(label.id);
        });
    }

    fn tree(&mut self, ui: &mut Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter stories"));
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
        args_of(&mut self.args, &self.saved_args, story)
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
                let label = ui.label(arg.name);
                let control = match &mut arg.value {
                    ArgValue::Bool(b) => ui.checkbox(b, ""),
                    ArgValue::Choice { options, index } => {
                        let shown = options.get(*index).copied().unwrap_or("");
                        ComboBox::from_id_salt(arg.name)
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for (i, option) in options.iter().enumerate() {
                                    ui.selectable_value(index, i, *option);
                                }
                            })
                            .response
                    }
                    ArgValue::Text(t) => ui.text_edit_singleline(t),
                    ArgValue::Number { value, min, max } => ui.add(egui::DragValue::new(value).range(*min..=*max).speed((*max - *min) / 200.0)),
                };
                control.labelled_by(label.id);
                ui.end_row();
            }
        });
    }

    fn centre(&mut self, ui: &mut Ui, inspecting: bool) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.compare, false, "Canvas");
            ui.selectable_value(&mut self.compare, true, "Compare");
            if self.compare {
                // Stays open while themes are ticked; a click outside closes it.
                let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
                MenuButton::new("Themes").config(config).ui(ui, |ui| {
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
                ui.checkbox(&mut self.compare_high, "Show high contrast variants");
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
        let high = self.compare_high;
        let args = args_of(&mut self.args, &self.saved_args, &story);
        let inspector = inspecting.then_some(&mut self.inspector);
        ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            if compare {
                compare_grid(ui, &story, args, &appearance, &themes, high, inspector);
            } else {
                canvas(ui, &story, args, &appearance, inspector);
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
            compare_high: self.compare_high,
            tokens: self.tokens,
            inspect: self.inspect,
            args: saved_args,
        };
        eframe::set_value(storage, KEY, &saved);
    }
}

/// The args of `story` in `args`, from the saved values on first use.
fn args_of<'a>(args: &'a mut HashMap<String, Args>, saved: &HashMap<String, Vec<persist::SavedArg>>, story: &Story) -> &'a mut Args {
    args.entry(story.id.to_owned()).or_insert_with(|| match saved.get(story.id) {
        Some(values) => persist::restore_args(story.defaults(), values),
        None => story.defaults(),
    })
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

/// A face picker showing `shown`, the face in use for the stored pick `value`; choosing an option
/// stores it.
fn face_picker(ui: &mut Ui, label: &str, id: &str, value: &mut String, shown: &str, options: &[&str]) {
    let label = ui.label(label);
    ComboBox::from_id_salt(id)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for option in options {
                if ui.selectable_label(*option == shown, *option).clicked() {
                    (*option).clone_into(value);
                }
            }
        })
        .response
        .labelled_by(label.id);
}

/// A labelled pair of exclusive choices.
fn segmented(ui: &mut Ui, label: &str, value: &mut bool, options: [(&str, bool); 2]) {
    ui.label(label);
    for (name, option) in options {
        ui.selectable_value(value, option, name);
    }
}

/// The story on `surface.background` under the bar's appearance.
fn canvas(ui: &mut Ui, story: &Story, args: &mut Args, appearance: &Appearance, inspector: Option<&mut Inspector>) {
    scope(ui, |o| appearance.overrides(o), |ui| framed(ui, story, args, inspector, "Canvas", appearance.rtl));
}

/// The story in its frame, with the inspector over it when there is one.
fn framed(ui: &mut Ui, story: &Story, args: &mut Args, inspector: Option<&mut Inspector>, name: &str, rtl: bool) {
    let tokens = Tokens::of(ui);
    let fill = tokens.color.surface.background.to_egui();
    let framed = Frame::new().fill(fill).inner_margin(Margin::same(16)).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.push_id(story.id, |ui| {
            (story.show)(ui, args);
            ui.unique_id()
        })
        .inner
    });
    if let Some(inspector) = inspector {
        inspector.overlay(ui, name.to_owned(), framed.response.rect, framed.inner.accesskit_id(), tokens.size.control.sm, rtl);
    }
}

/// Rows: the chosen themes. Columns: light and dark in standard contrast, each followed by its
/// high-contrast column when `high` is set.
/// A mode the theme doesn't have leaves its column empty.
fn compare_grid(
    ui: &mut Ui,
    story: &Story,
    args: &mut Args,
    appearance: &Appearance,
    themes: &[String],
    high: bool,
    mut inspector: Option<&mut Inspector>,
) {
    const ALL: [(Mode, bool); 4] = [(Mode::Light, false), (Mode::Light, true), (Mode::Dark, false), (Mode::Dark, true)];
    const STANDARD: [(Mode, bool); 2] = [(Mode::Light, false), (Mode::Dark, false)];
    let columns: &[(Mode, bool)] = if high { &ALL } else { &STANDARD };
    for id in themes {
        let Some(theme) = builtin(id) else {
            continue;
        };
        ui.heading(&theme.name);
        ui.columns(columns.len(), |uis| {
            for (ui, &(mode, high)) in uis.iter_mut().zip(columns) {
                let mode_name = if mode == Mode::Dark { "Dark" } else { "Light" };
                let name = format!("{mode_name} {}", if high { "high contrast" } else { "standard" });
                // The label sits on the gallery's background, so it keeps the gallery's colours.
                ui.label(name.as_str());
                // A one-variant theme shows its other mode in this column's place; leave it empty
                // so each column shows what its label says.
                if theme.variants.mode_for(mode) != mode {
                    ui.weak(format!("No {} variant", mode_name.to_lowercase()));
                    continue;
                }
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
                            framed(ui, story, args, inspector.as_deref_mut(), &format!("{} · {name}", theme.name), appearance.rtl);
                        },
                    );
                });
            }
        });
        ui.add_space(12.0);
    }
}
