//! The gallery app.
use std::collections::HashMap;
use std::sync::Arc;

use craft_platform::appearance::SystemReader;
use craft_theme::{ContrastChoice, Mode, ModeChoice, SystemAppearance, TextSize, TokenRef, builtins};
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::composites::PropertyRow;
use craft_ui::controls::{Button, Checkbox, NumberField, SearchField, Segmented, Select, TextField, ToggleButton};
use craft_ui::direction;
use craft_ui::icons::{self, IconSet};
use craft_ui::inspect;
use craft_ui::list::{List, ListEvent, ListRow, TreeState};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::Divider;
use craft_ui::story::{ArgValue, Args, Level, Story, registry};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, margin, scope};
use egui::{CentralPanel, Frame, Panel, ScrollArea, Ui};

use crate::appearance::{Appearance, MotionChoice, SCALE, builtin};
use crate::browser::Browser;
use crate::faces;
use crate::fields;
use crate::inspector::Inspector;
use crate::persist::{self, KEY, Saved};

/// The gallery's state.
pub struct Gallery {
    stories: Vec<Story>,
    appearance: Appearance,
    /// The appearance `base` and the context's direction and zoom were set for.
    applied: Option<Appearance>,
    /// The scale the shown story asks for, once applied; `None` while no story asks.
    story_ppp: Option<f32>,
    /// Whether the OS's interface face had resolved when the faces were last installed.
    system_face_applied: bool,
    os: SystemReader,
    /// The OS's answer `base` was resolved with.
    os_applied: SystemAppearance,
    base: Base,
    selected: Option<String>,
    /// The story tree's selection and keyboard position; groups can hold them too.
    tree: Selection<Node>,
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
        let tree = tree_selection(&stories, selected.as_deref());
        Self {
            stories,
            base: saved.appearance.base(SystemAppearance::default()),
            appearance: saved.appearance,
            applied: None,
            story_ppp: None,
            system_face_applied: false,
            os: SystemReader::for_egui(),
            os_applied: SystemAppearance::default(),
            selected,
            tree,
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
            self.tree = tree_selection(&self.stories, Some(id));
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
        self.show_panels(ui);
        self.apply_story_scale(ui.ctx());
    }

    /// Sets the whole gallery's scale when a story starts or stops asking for one
    /// ([`story::request_pixels_per_point`](craft_ui::story::request_pixels_per_point)): the
    /// story's request, or back to the appearance bar's. In between the zoom is left alone, so
    /// egui's Ctrl +/− zoom keeps working.
    fn apply_story_scale(&mut self, ctx: &egui::Context) {
        let requested = craft_ui::story::take_pixels_per_point_request(ctx);
        if requested == self.story_ppp {
            return;
        }
        self.story_ppp = requested;
        let zoom = match (requested, ctx.native_pixels_per_point()) {
            (Some(ppp), Some(native)) if native > 0.0 => ppp / native,
            _ => self.appearance.zoom(),
        };
        set_zoom(ctx, zoom);
    }

    fn show_panels(&mut self, ui: &mut Ui) {
        self.apply_appearance(ui.ctx());
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        Panel::top("gallery.bar").show(ui, |ui| self.appearance_bar(ui));
        let inspecting = self.inspect && !self.tokens;
        inspect::set_enabled(&ctx, inspecting);
        if self.tokens {
            self.browser.show(ui, &self.base, self.appearance.mode(shown_mode(&ctx)));
            return;
        }
        direction::start_panel(&ctx, "gallery.tree").default_size(240.0).show(ui, |ui| self.tree(ui));
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
            direction::end_panel(&ctx, "gallery.inspect").default_size(320.0).show(ui, |ui| self.inspector.panel(ui));
        } else {
            direction::end_panel(&ctx, "gallery.args").default_size(240.0).show(ui, |ui| self.args_panel(ui));
        }
        let frame = Frame::new().fill(tokens.color.surface.canvas.to_egui()).inner_margin(margin(tokens.space.pad.panel));
        CentralPanel::default().frame(frame).show(ui, |ui| self.centre(ui, inspecting));
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        let system_face = faces::system_face(ctx).is_some();
        let system = SystemAppearance::from(self.os.read_in(ctx));
        if self.applied.as_ref() != Some(&self.appearance) || system_face != self.system_face_applied || system != self.os_applied {
            let previous = self.applied.replace(self.appearance.clone());
            self.base = self.appearance.configure(ctx, previous.as_ref(), system_face != self.system_face_applied, system);
            self.os_applied = system;
            self.system_face_applied = system_face;
            if previous.as_ref().is_none_or(|p| p.icon_set != self.appearance.icon_set) {
                icons::set_active(ctx, self.appearance.icon_set());
            }
            // A story's scale stays until it stops asking.
            if self.story_ppp.is_none() {
                set_zoom(ctx, self.appearance.zoom());
            }
        }
        install(ctx, &self.base);
    }

    fn appearance_bar(&mut self, ui: &mut Ui) {
        let a = &mut self.appearance;
        let tokens = &mut self.tokens;
        let inspect = &mut self.inspect;
        let numbers = LocaleNumbers::of(ui);
        ui.horizontal_wrapped(|ui| {
            ui.add(ToggleButton::new("Tokens", tokens));
            ui.add(ToggleButton::new("Inspect", inspect).enabled(!*tokens));
            Divider::vertical().show(ui);
            ui.label("Theme");
            let current = builtin(&a.theme).map_or(a.theme.as_str(), |t| t.name.as_str()).to_owned();
            let mut theme = Select::new("gallery.theme", "Theme", &mut a.theme).placeholder(current).fit(true);
            for t in builtins() {
                theme = theme.option(t.id.as_str().to_owned(), &t.name);
            }
            let _ = theme.policy(fields::policy()).show(ui);
            segmented(ui, "Mode", &mut a.mode, [("Light", ModeChoice::Light), ("Dark", ModeChoice::Dark), ("System", ModeChoice::System)]);
            segmented(
                ui,
                "Contrast",
                &mut a.contrast,
                [("Standard", ContrastChoice::Standard), ("High", ContrastChoice::High), ("System", ContrastChoice::System)],
            );
            segmented(ui, "Size", &mut a.comfortable, [("Compact", false), ("Comfortable", true)]);
            ui.label("Text");
            let mut text = Segmented::new("gallery.text_size", "Text", &mut a.text_size);
            for (i, size) in TextSize::ALL.iter().enumerate() {
                text = text.item(i, text_size_name(*size));
            }
            text.show(ui);
            segmented(
                ui,
                "Reduced motion",
                &mut a.motion,
                [("Off", MotionChoice::Full), ("On", MotionChoice::Reduced), ("System", MotionChoice::System)],
            );
            segmented(ui, "Direction", &mut a.rtl, [("LTR", false), ("RTL", true)]);
            if faces::available() {
                let ui_shown = faces::shown_ui_face(ui.ctx(), &a.ui_face);
                face_picker(ui, "UI face", "gallery.ui_face", &mut a.ui_face, ui_shown, &faces::ui_faces(ui.ctx()));
                let mono_shown = faces::shown_mono_face(&a.mono_face);
                face_picker(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, mono_shown, faces::MONO_FACES);
            }
            ui.label("Icons");
            let mut icon_set = a.icon_set().id().to_owned();
            let mut icons = Select::new("gallery.icon_set", "Icons", &mut icon_set).fit(true);
            for set in IconSet::ALL {
                icons = icons.option(set.id().to_owned(), set.title());
            }
            if icons.policy(fields::policy()).show(ui).response.changed() {
                a.icon_set = icon_set;
            }
            ui.label("Scale");
            let scale = NumberField::new("gallery.scale", "Scale", a.scale, &numbers)
                .range(SCALE)
                .decimals(0..=0)
                .unit(Unit::new("{n}%"))
                .popup_slider("Scale slider")
                .policy(fields::policy())
                .show(ui);
            let mut percent = f64::from(a.scale);
            if fields::apply(&scale, &mut percent) {
                a.scale = percent.round() as u32;
            }
        });
    }

    /// The story tree with its filter field.
    fn tree(&mut self, ui: &mut Ui) {
        let filter = SearchField::new("gallery.filter", "Filter stories", &self.filter, "Clear filter")
            .placeholder("Filter stories")
            .policy(fields::policy())
            .show(ui);
        fields::apply(&filter, &mut self.filter);
        let filter = self.filter.to_lowercase();
        let ctx = ui.ctx().clone();
        let mut open = TreeState::load(&ctx, "gallery.tree");
        let seeded = egui::Id::NULL.with("gallery.tree.seeded");
        if !ctx.data_mut(|d| d.get_persisted::<bool>(seeded)).unwrap_or(false) {
            for level in Level::ALL {
                open.set(&Node::Level(level), true);
            }
            ctx.data_mut(|d| d.insert_persisted(seeded, true));
        }
        // A filter shows every match, whatever the groups' state.
        let filtering = !filter.is_empty();
        let mut rows: Vec<RowInfo<Node>> = Vec::new();
        let mut groups: Vec<Node> = Vec::new();
        for (level, components) in nav(&self.stories, &filter) {
            let level_open = filtering || open.is_open(&Node::Level(level));
            rows.push(RowInfo::group(Node::Level(level), level_open));
            groups.push(Node::Level(level));
            for component in components {
                let key = Node::Component(level, component);
                let component_open = filtering || open.is_open(&key);
                groups.push(key.clone());
                if !level_open {
                    continue;
                }
                rows.push(RowInfo::group(key, component_open).at_depth(1));
                if component_open {
                    for story in self.stories.iter().filter(|s| s.level == level && s.component == component && matches(s, &filter)) {
                        rows.push(RowInfo::leaf(Node::Story(story.id)).at_depth(2));
                    }
                }
            }
        }
        let stories = &self.stories;
        let out = List::new("gallery.stories", "Stories", rows.as_slice())
            .tree(true)
            .selection(&mut self.tree)
            .mode(SelectionMode::Single)
            .allow_empty(false)
            .show(ui, |_, node| ListRow::new(node.clone(), node.title(stories)));
        for event in out.events {
            match event {
                ListEvent::Selection(change) => {
                    if let Some(Node::Story(id)) = change.row {
                        self.selected = Some(id.to_owned());
                    }
                }
                ListEvent::Open(change) => open.apply(&change, groups.iter()),
                ListEvent::Activate(Node::Story(id)) => self.selected = Some(id.to_owned()),
                ListEvent::Activate(group) => {
                    let now = open.is_open(&group);
                    open.set(&group, !now);
                }
                _ => {}
            }
        }
        open.store(&ctx);
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
        if ui.add(Button::new("Reset args")).clicked() {
            self.args.insert(story.id.to_owned(), story.defaults());
            self.saved_args.remove(story.id);
        }
        Divider::horizontal().show(ui);
        let numbers = LocaleNumbers::of(ui);
        let args = self.args_of(&story);
        for arg in args.iter_mut() {
            let id = ("gallery.arg", story.id, arg.name);
            PropertyRow::word(arg.name).show(ui, |ui, _| match &mut arg.value {
                ArgValue::Bool(b) => Checkbox::bool("", b).show(ui).response,
                ArgValue::Choice { options, index } => {
                    let mut select = Select::new(id, arg.name, index).fit(true);
                    for (i, option) in options.iter().enumerate() {
                        select = select.option(i, *option);
                    }
                    select.policy(fields::policy()).show(ui).response
                }
                ArgValue::Text(t) => {
                    let edit = TextField::new(id, arg.name, t).policy(fields::policy()).show(ui);
                    fields::apply(&edit, t);
                    edit.response
                }
                ArgValue::Number { value, min, max } => {
                    let edit = NumberField::new(id, arg.name, *value, &numbers)
                        .range(*min..=*max)
                        .step((*max - *min) / 200.0)
                        .policy(fields::policy())
                        .show(ui);
                    fields::apply(&edit, value);
                    edit.response
                }
            });
        }
    }

    fn centre(&mut self, ui: &mut Ui, inspecting: bool) {
        ui.horizontal_wrapped(|ui| {
            Segmented::new("gallery.view", "View", &mut self.compare).item(false, "Canvas").item(true, "Compare").show(ui);
            if self.compare {
                ui.label("Themes");
                for theme in builtins() {
                    let id = theme.id.as_str();
                    let mut on = self.compare_themes.iter().any(|t| t == id);
                    if Checkbox::bool(&theme.name, &mut on).show(ui).response.changed() {
                        if on {
                            self.compare_themes.push(id.to_owned());
                        } else {
                            self.compare_themes.retain(|t| t != id);
                        }
                    }
                }
                let _ = Checkbox::bool("Show high contrast variants", &mut self.compare_high).show(ui);
            }
        });
        Divider::horizontal().show(ui);
        let Some(story) = self.current() else {
            ui.label("No story selected.");
            return;
        };
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let themes = self.compare_themes.clone();
        let compare = self.compare;
        let high = self.compare_high;
        let args = args_of(&mut self.args, &self.saved_args, &story);
        let inspector = inspecting.then_some(&mut self.inspector);
        ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            if compare {
                compare_grid(ui, &story, args, &appearance, &themes, high, inspector);
            } else {
                canvas(ui, &story, args, &appearance, system, inspector);
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

/// The navigation's levels in their order, each with the components that have a story matching
/// `filter` (lowercase), sorted by name ignoring case, whatever the registration order.
fn nav(stories: &[Story], filter: &str) -> Vec<(Level, Vec<&'static str>)> {
    Level::ALL
        .into_iter()
        .filter_map(|level| {
            let mut components: Vec<&'static str> = Vec::new();
            for story in stories.iter().filter(|s| s.level == level && matches(s, filter)) {
                if !components.contains(&story.component) {
                    components.push(story.component);
                }
            }
            components.sort_by_cached_key(|c| c.to_lowercase());
            (!components.is_empty()).then_some((level, components))
        })
        .collect()
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
    ui.label(label);
    let mut pick = shown.to_owned();
    let mut select = Select::new(id, label, &mut pick).fit(true);
    for option in options {
        select = select.option((*option).to_owned(), *option);
    }
    if select.policy(fields::policy()).show(ui).response.changed() {
        *value = pick;
    }
}

/// A labelled set of exclusive choices.
fn segmented<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) {
    ui.label(label);
    let mut segmented = Segmented::new(("gallery.segmented", label), label, value);
    for (name, option) in options {
        segmented = segmented.item(option, name);
    }
    segmented.show(ui);
}

/// The mode `ctx` shows, which is the OS's answer while the mode choice is System.
fn shown_mode(ctx: &egui::Context) -> Mode {
    match ctx.theme() {
        egui::Theme::Dark => Mode::Dark,
        egui::Theme::Light => Mode::Light,
    }
}

/// Sets the context's zoom factor. The gallery is the host of the craft-ui showcase: its interface
/// scale (the appearance bar's, or a story's request) is a window setting that craft-ui has no
/// token for.
fn set_zoom(ctx: &egui::Context, zoom: f32) {
    // design: allow[style-mutation](the gallery's interface scale is host configuration; craft-ui has no zoom setting)
    ctx.set_zoom_factor(zoom);
}

/// A row of the story tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Node {
    Level(Level),
    Component(Level, &'static str),
    Story(&'static str),
}

impl Node {
    fn title(&self, stories: &[Story]) -> String {
        match self {
            Node::Level(level) => level.title().to_owned(),
            Node::Component(_, component) => (*component).to_owned(),
            Node::Story(id) => stories.iter().find(|s| s.id == *id).map_or_else(|| (*id).to_owned(), |s| s.name.to_owned()),
        }
    }
}

/// The tree selection for the story `id`.
fn tree_selection(stories: &[Story], id: Option<&str>) -> Selection<Node> {
    id.and_then(|id| stories.iter().find(|s| s.id == id)).map_or_else(Selection::default, |s| Selection::only(Node::Story(s.id)))
}

/// The story on `surface.background` under the bar's appearance.
fn canvas(ui: &mut Ui, story: &Story, args: &mut Args, appearance: &Appearance, system: SystemAppearance, inspector: Option<&mut Inspector>) {
    let shown = shown_mode(ui.ctx());
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        appearance.overrides(o);
        o.mode = Some(appearance.mode(shown));
        o.contrast = Some(appearance.contrast(system));
    };
    scope(ui, overrides, |ui| framed(ui, story, args, inspector, "Canvas", appearance.rtl));
}

/// The story in its frame, with the inspector over it when there is one.
fn framed(ui: &mut Ui, story: &Story, args: &mut Args, inspector: Option<&mut Inspector>, name: &str, rtl: bool) {
    let tokens = Tokens::of(ui);
    let fill = tokens.color.surface.background.to_egui();
    let framed = Frame::new().fill(fill).inner_margin(margin(tokens.space.pad.dialog)).show(ui, |ui| {
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
        ui.add_space(Tokens::of(ui).space.gap.section);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nav_lists_each_levels_components_alphabetically() {
        let mut stories = registry();
        stories.extend(faces::stories());
        let nav = nav(&stories, "");
        assert!(!nav.is_empty());
        for (level, components) in &nav {
            let keys: Vec<String> = components.iter().map(|c| c.to_lowercase()).collect();
            assert!(keys.is_sorted(), "{level:?}: {components:?}");
        }
        let levels: Vec<Level> = nav.iter().map(|(level, _)| *level).collect();
        let order: Vec<Level> = Level::ALL.into_iter().filter(|l| levels.contains(l)).collect();
        assert_eq!(levels, order);
    }
}
