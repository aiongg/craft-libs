//! The gallery app.
use std::collections::HashMap;
use std::sync::Arc;

use craft_platform::appearance::SystemReader;
use craft_theme::{ContrastChoice, ControlSize, Mode, ModeChoice, MotionChoice, SystemAppearance, TextSize, builtins};
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::composites::{PropertyRow, SectionHeader};
use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, SearchField, Segmented, Select, TextField};
use craft_ui::direction;
use craft_ui::docking::{Action, DockArea, DockConfig, DockContent, DockHeights, PanelState, Permissions, Workspace};
use craft_ui::icons::{self, Icon, IconSet};
use craft_ui::inspect;
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::story::{ArgValue, Args, Level, Story, registry};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, margin, scope};
use egui::{CentralPanel, Frame, Panel, ScrollArea, Ui, UiBuilder};

use crate::appearance::{Appearance, SCALE, builtin};
use crate::browser::Browser;
use crate::faces;
use crate::fields;
use crate::inspector::Inspector;
use crate::layout::{PanelId, default_workspace, reconciled};
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
    filter: String,
    compare: bool,
    compare_themes: Vec<String>,
    compare_high: bool,
    browser: Browser,
    inspector: Inspector,
    /// The dock: where each panel is, and which tab is shown.
    workspace: Workspace<PanelId>,
    /// Reset layout was chosen in this frame.
    reset_layout: bool,
    /// Edited args by story id; a story gets its entry when first shown.
    args: HashMap<String, Args>,
    saved_args: HashMap<String, Vec<persist::SavedArg>>,
}

impl Gallery {
    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        let saved = Saved::load(storage);
        let mut stories = registry();
        stories.extend(faces::stories());
        let selected = saved.selected.filter(|id| stories.iter().any(|s| s.id == id)).or_else(|| first_story(&stories).map(str::to_owned));
        let mut compare_themes: Vec<String> = saved.compare_themes.into_iter().filter(|id| builtin(id).is_some()).collect();
        if compare_themes.is_empty() {
            compare_themes.push(saved.appearance.theme.clone());
        }
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
            filter: saved.filter,
            compare: saved.compare,
            compare_themes,
            compare_high: saved.compare_high,
            browser: Browser::default(),
            inspector: Inspector::default(),
            workspace: saved.layout.map_or_else(default_workspace, reconciled),
            reset_layout: false,
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

    /// Shows the token browser tab instead of the canvas tab.
    pub fn set_tokens(&mut self, tokens: bool) {
        self.activate(if tokens { PanelId::Tokens } else { PanelId::Canvas });
    }

    /// Shows the inspector tab, which turns inspect mode on, instead of the properties tab.
    pub fn set_inspect(&mut self, inspect: bool) {
        self.activate(if inspect { PanelId::Inspect } else { PanelId::Properties });
    }

    fn activate(&mut self, panel: PanelId) {
        let _ = self.workspace.apply(Action::Activate { panel }, PanelId::permissions);
    }

    /// The labels of the focus-order badges of each canvas the inspector drew in the last frame.
    pub fn focus_badges(&self) -> Vec<Vec<String>> {
        self.inspector.cells().iter().map(|c| c.badges.clone()).collect()
    }

    /// Selects a row of the token browser.
    pub fn select_token(&mut self, token: craft_theme::TokenRef) {
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
    /// story's request, or back to the options bar's. In between the zoom is left alone, so
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
        Panel::top("gallery.bar").frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| self.options_bar(ui));
        let layout = &self.workspace.layout;
        let inspecting = layout.panel_state(&PanelId::Inspect) == PanelState::Front && layout.panel_state(&PanelId::Canvas) == PanelState::Front;
        inspect::set_enabled(&ctx, inspecting);
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
        }
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| self.dock(ui, inspecting));
        if std::mem::take(&mut self.reset_layout) {
            self.workspace = default_workspace();
        }
    }

    /// The panels in their dock. The workspace is taken out while the panels draw, because they
    /// borrow the gallery.
    fn dock(&mut self, ui: &mut Ui, inspecting: bool) {
        let mut workspace = std::mem::take(&mut self.workspace);
        let tokens = Tokens::of(ui);
        let mut config = DockConfig::default();
        config.right_to_left = direction::is_rtl(ui);
        let mut panels = Panels { gallery: self, inspecting, control: tokens.size.control.sm };
        let output =
            DockArea::new(ui.make_persistent_id("gallery.dock")).locked(workspace.locked).show_content(ui, &workspace.layout, &config, &mut panels);
        for action in output.actions {
            let _ = workspace.apply(action, PanelId::permissions);
        }
        self.workspace = workspace;
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

    /// The appearance options and the view, with Reset layout pinned at the end.
    fn options_bar(&mut self, ui: &mut Ui) {
        let a = &mut self.appearance;
        let compare = &mut self.compare;
        let reset_layout = &mut self.reset_layout;
        let numbers = LocaleNumbers::of(ui);
        let faces_shown =
            faces::available().then(|| (faces::shown_ui_face(ui.ctx(), &a.ui_face), faces::shown_mono_face(&a.mono_face), faces::ui_faces(ui.ctx())));
        OptionsBar::new("gallery.options").label("Appearance").show(ui, |bar| {
            bar.body(|ui| {
                OptionsBar::cell(ui, "Theme:", CellWidth::Content, |ui, _| {
                    let current = builtin(&a.theme).map_or(a.theme.as_str(), |t| t.name.as_str()).to_owned();
                    let mut theme = Select::new("gallery.theme", "Theme", &mut a.theme).placeholder(current).fit(true).size(ControlSize::Sm);
                    for t in builtins() {
                        theme = theme.option(t.id.as_str().to_owned(), &t.name);
                    }
                    let _ = theme.policy(fields::policy()).show(ui);
                });
                segmented(ui, "Mode", &mut a.mode, [("Light", ModeChoice::Light), ("Dark", ModeChoice::Dark), ("System", ModeChoice::System)]);
                segmented(
                    ui,
                    "Contrast",
                    &mut a.contrast,
                    [("Standard", ContrastChoice::Standard), ("High", ContrastChoice::High), ("System", ContrastChoice::System)],
                );
                segmented(ui, "Size", &mut a.comfortable, [("Compact", false), ("Comfortable", true)]);
                segmented(
                    ui,
                    "Reduced motion",
                    &mut a.motion,
                    [("Off", MotionChoice::Full), ("On", MotionChoice::Reduced), ("System", MotionChoice::System)],
                );
                segmented(ui, "Direction", &mut a.rtl, [("LTR", false), ("RTL", true)]);
                OptionsBar::divider(ui);
                OptionsBar::cell(ui, "Text:", CellWidth::Content, |ui, _| {
                    let mut text = Segmented::new("gallery.text_size", "Text", &mut a.text_size).size(ControlSize::Sm);
                    for (i, size) in TextSize::ALL.iter().enumerate() {
                        text = text.item(i, text_size_name(*size));
                    }
                    text.show(ui);
                });
                if let Some((ui_shown, mono_shown, ui_faces)) = &faces_shown {
                    face_picker(ui, "UI face", "gallery.ui_face", &mut a.ui_face, ui_shown, ui_faces);
                    face_picker(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, mono_shown, faces::MONO_FACES);
                }
                OptionsBar::cell(ui, "Icons:", CellWidth::Content, |ui, _| {
                    let mut icon_set = a.icon_set().id().to_owned();
                    let mut icons = Select::new("gallery.icon_set", "Icons", &mut icon_set).fit(true).size(ControlSize::Sm);
                    for set in IconSet::ALL {
                        icons = icons.option(set.id().to_owned(), set.title());
                    }
                    if icons.policy(fields::policy()).show(ui).response.changed() {
                        a.icon_set = icon_set;
                    }
                });
                OptionsBar::cell(ui, "Scale:", CellWidth::Md, |ui, caption| {
                    let scale = NumberField::new("gallery.scale", "Scale", a.scale, &numbers)
                        .range(SCALE)
                        .decimals(0..=0)
                        .unit(Unit::new("{n}%"))
                        .popup_slider("Scale slider")
                        .size(ControlSize::Sm)
                        .scrub_by(caption)
                        .policy(fields::policy())
                        .show(ui);
                    let mut percent = f64::from(a.scale);
                    if fields::apply(&scale, &mut percent) {
                        a.scale = percent.round() as u32;
                    }
                });
                OptionsBar::divider(ui);
                segmented(ui, "View", compare, [("Canvas", false), ("Compare", true)]);
            });
            bar.trailing(|ui| {
                if ui.add(Button::new("Reset layout").size(ControlSize::Sm)).clicked() {
                    *reset_layout = true;
                }
            });
        });
    }

    /// The components that have a story matching the filter, under a label for each level.
    fn components_panel(&mut self, ui: &mut Ui) {
        let filter = SearchField::new("gallery.filter", "Filter components", &self.filter, "Clear filter")
            .placeholder("Filter components")
            .policy(fields::policy())
            .show(ui);
        fields::apply(&filter, &mut self.filter);
        let filter = self.filter.to_lowercase();
        let current = self.current().map(|s| s.component);
        let mut rows: Vec<RowInfo<Entry>> = Vec::new();
        for (level, components) in nav(&self.stories, &filter) {
            rows.push(RowInfo::section(Entry::Level(level)));
            rows.extend(components.into_iter().map(|c| RowInfo::leaf(Entry::Component(c))));
        }
        let mut selection =
            current.map(Entry::Component).filter(|e| rows.iter().any(|r| r.key == *e)).map_or_else(Selection::default, Selection::only);
        let out = List::new("gallery.components", "Components", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .type_ahead(true)
            .show(ui, |_, entry| match entry {
                Entry::Level(level) => ListRow::new(*entry, level.title()),
                Entry::Component(component) => ListRow::new(*entry, *component),
            });
        let mut picked = None;
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(Entry::Component(c)), .. }) | ListEvent::Activate(Entry::Component(c)) => {
                    picked = Some(c)
                }
                _ => {}
            }
        }
        if let Some(component) = picked.filter(|c| Some(*c) != current) {
            self.select_component(component);
        }
    }

    /// Opens the first story of `component`.
    fn select_component(&mut self, component: &str) {
        if let Some(story) = self.stories.iter().find(|s| s.component == component) {
            self.selected = Some(story.id.to_owned());
        }
    }

    /// The selected component's stories.
    fn stories_panel(&mut self, ui: &mut Ui) {
        let Some(component) = self.current().map(|s| s.component) else {
            return;
        };
        let stories: Vec<Story> = self.stories.iter().filter(|s| s.component == component).copied().collect();
        let rows: Vec<RowInfo<&'static str>> = stories.iter().map(|s| RowInfo::leaf(s.id)).collect();
        let mut selection =
            self.selected.as_deref().and_then(|id| stories.iter().find(|s| s.id == id)).map_or_else(Selection::default, |s| Selection::only(s.id));
        let out = List::new("gallery.stories", "Stories", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .show(ui, |_, id| ListRow::new(*id, stories.iter().find(|s| s.id == *id).map_or(*id, |s| s.name)));
        for event in out.events {
            match event {
                ListEvent::Selection(change) => {
                    if let Some(id) = change.row {
                        self.selected = Some(id.to_owned());
                    }
                }
                ListEvent::Activate(id) => self.selected = Some(id.to_owned()),
                _ => {}
            }
        }
    }

    fn current(&self) -> Option<Story> {
        let id = self.selected.as_deref()?;
        self.stories.iter().find(|s| s.id == id).copied()
    }

    /// The args of `story`, from the saved values on first use.
    fn args_of(&mut self, story: &Story) -> &mut Args {
        args_of(&mut self.args, &self.saved_args, story)
    }

    fn reset_args(&mut self) {
        if let Some(story) = self.current() {
            self.args.insert(story.id.to_owned(), story.defaults());
            self.saved_args.remove(story.id);
        }
    }

    /// The selected story's args, and the compare options while the compare view is on.
    fn properties_panel(&mut self, ui: &mut Ui) {
        if self.compare {
            SectionHeader::new("gallery.compare", "Compare").collapsible(false).show(ui, |ui| {
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
            });
        }
        let Some(story) = self.current() else {
            ui.label("No story selected.");
            return;
        };
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

    /// The story, or the grid of appearances in the compare view, on the canvas colour.
    fn canvas_panel(&mut self, ui: &mut Ui, inspecting: bool) {
        let tokens = Tokens::of(ui);
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, tokens.color.surface.canvas.to_egui());
        let mut inner = ui.new_child(UiBuilder::new().max_rect(rect.shrink(tokens.space.pad.panel)));
        let ui = &mut inner;
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

    fn tokens_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        self.browser.show(ui, &self.base, mode);
    }
}

/// A row of the component list: a level's label, or a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Entry {
    Level(Level),
    Component(&'static str),
}

/// The panel menu's commands.
#[derive(Clone, Copy)]
enum PanelCommand {
    ResetArgs,
    ResetLayout,
}

/// The gallery's panels as the dock's contents.
struct Panels<'a> {
    gallery: &'a mut Gallery,
    inspecting: bool,
    /// `size.control.sm`: the panel menu button's width.
    control: f32,
}

/// Draws `add` inside the panel's padding.
fn padded(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let pad = Tokens::of(ui).space.pad.panel;
    Frame::new().inner_margin(margin(pad)).show(ui, add);
}

/// Draws `add` in a vertical scroll area inside the panel's padding.
fn scrolled(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| padded(ui, add));
}

impl DockContent<PanelId> for Panels<'_> {
    fn body(&mut self, ui: &mut Ui, panel: &PanelId) {
        match panel {
            PanelId::Components => padded(ui, |ui| self.gallery.components_panel(ui)),
            PanelId::Stories => padded(ui, |ui| self.gallery.stories_panel(ui)),
            PanelId::Properties => scrolled(ui, |ui| self.gallery.properties_panel(ui)),
            PanelId::Inspect => padded(ui, |ui| self.gallery.inspector.panel(ui)),
            PanelId::Canvas => self.gallery.canvas_panel(ui, self.inspecting),
            PanelId::Tokens => self.gallery.tokens_panel(ui),
        }
    }

    fn logical_label(&self, panel: &PanelId) -> String {
        panel.title().to_owned()
    }

    fn permissions(&self, panel: &PanelId) -> Permissions {
        panel.permissions()
    }

    fn dock_heights(&self, panel: &PanelId) -> Option<DockHeights> {
        match panel {
            PanelId::Stories => Some(DockHeights::new(170.0).compact(90.0)),
            PanelId::Properties | PanelId::Inspect => Some(DockHeights::new(240.0).compact(120.0).fill(300.0)),
            _ => None,
        }
    }

    fn group_header_width(&self, panels: &[PanelId], _: &PanelId, _: bool, _: f32) -> f32 {
        if panels.contains(&PanelId::Properties) { self.control } else { 0.0 }
    }

    fn group_header(&mut self, ui: &mut Ui, panels: &[PanelId], _: &PanelId, _: bool) {
        if !panels.contains(&PanelId::Properties) {
            return;
        }
        let entries: Vec<MenuEntry<PanelCommand>> =
            vec![MenuItem::new("Reset args", PanelCommand::ResetArgs).into(), MenuItem::new("Reset layout", PanelCommand::ResetLayout).into()];
        let button = ui.add(IconButton::new(Icon::More, "Panel menu").size(ControlSize::Sm));
        match PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen {
            Some(PanelCommand::ResetArgs) => self.gallery.reset_args(),
            Some(PanelCommand::ResetLayout) => self.gallery.reset_layout = true,
            None => {}
        }
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
            layout: Some(self.workspace.clone()),
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

/// The story the navigation lists first: the first story of its first level's first component.
fn first_story(stories: &[Story]) -> Option<&'static str> {
    let (level, components) = nav(stories, "").into_iter().next()?;
    let component = *components.first()?;
    stories.iter().find(|s| s.level == level && s.component == component).map(|s| s.id)
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
    OptionsBar::cell(ui, &format!("{label}:"), CellWidth::Content, |ui, _| {
        let mut pick = shown.to_owned();
        let mut select = Select::new(id, label, &mut pick).fit(true).size(ControlSize::Sm);
        for option in options {
            select = select.option((*option).to_owned(), *option);
        }
        if select.policy(fields::policy()).show(ui).response.changed() {
            *value = pick;
        }
    });
}

/// A captioned set of exclusive choices in the options bar.
fn segmented<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) {
    OptionsBar::cell(ui, &format!("{label}:"), CellWidth::Content, |ui, _| {
        let mut segmented = Segmented::new(("gallery.segmented", label), label, value).size(ControlSize::Sm);
        for (name, option) in options {
            segmented = segmented.item(option, name);
        }
        segmented.show(ui);
    });
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
    use std::collections::HashMap;

    use super::*;
    use crate::layout::PanelId;

    /// An in-memory eframe storage.
    #[derive(Default)]
    struct Memory(HashMap<String, String>);

    impl eframe::Storage for Memory {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }

        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }

        fn flush(&mut self) {}
    }

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

    #[test]
    fn a_fresh_gallery_opens_the_navigations_first_story() {
        let gallery = Gallery::new(None);
        let nav = nav(&gallery.stories, "");
        let (level, components) = &nav[0];
        let first = gallery.stories.iter().find(|s| s.level == *level && s.component == components[0]).expect("the nav lists stories").id;
        assert_eq!(gallery.selected.as_deref(), Some(first));
        assert_eq!(first_story(&gallery.stories), Some(first));
    }

    #[test]
    fn picking_a_component_opens_its_first_story() {
        let mut gallery = Gallery::new(None);
        let (component, first) = {
            let story = gallery.stories.iter().rev().find(|s| s.component != gallery.current().unwrap().component).unwrap();
            let first = gallery.stories.iter().find(|s| s.component == story.component).unwrap().id;
            (story.component, first)
        };
        gallery.select_component(component);
        assert_eq!(gallery.selected.as_deref(), Some(first));
    }

    #[test]
    fn the_dock_layout_round_trips_through_storage() {
        let mut gallery = Gallery::new(None);
        gallery.set_tokens(true);
        gallery.workspace.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
        let mut storage = Memory::default();
        eframe::App::save(&mut gallery, &mut storage);
        let restored = Gallery::new(Some(&storage));
        assert_eq!(restored.workspace, gallery.workspace);
        assert_ne!(restored.workspace, default_workspace());
    }

    #[test]
    fn an_invalid_saved_layout_falls_back_to_the_default() {
        use craft_ui::docking::{Layout, Node};
        let broken =
            Workspace::new(Layout { root: Some(Node::Tabs { panels: vec![PanelId::Canvas, PanelId::Canvas], active: 0 }), floating: Vec::new() });
        let mut storage = Memory::default();
        eframe::set_value(&mut storage, KEY, &Saved { layout: Some(broken), ..Saved::default() });
        assert_eq!(Gallery::new(Some(&storage)).workspace, default_workspace());
    }
}
