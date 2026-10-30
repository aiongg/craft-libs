//! The gallery app.
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use craft_platform::appearance::SystemReader;
use craft_theme::{ButtonVariant, ContrastChoice, ControlSize, Mode, ModeChoice, MotionChoice, SystemAppearance, TextSize, TypeRole, builtins};
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::composites::{EmptyState, FormGrid, PropertyRow};
use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, SearchField, Segmented, Select, TextField, ToggleButton, TooltipExt as _};
use craft_ui::direction;
use craft_ui::docking::{Action, DockArea, DockConfig, DockContent, DockHeights, DockLook, PanelState, Permissions, Workspace};
use craft_ui::icons::{self, Icon, IconSet};
use craft_ui::inspect;
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::popover::{self, Popover, PopoverAnchor};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{ArgValue, Args, Level, Story, Surface, registry};
use craft_ui::theme::{Base, StrokeExt as _, ToEgui, Tokens, TokensOf, install, margin, scale, scope};
use egui::epaint::RectShape;
use egui::{
    Align, CentralPanel, Color32, Frame, Margin, Panel, Rect, Role, ScrollArea, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2,
};

use crate::appearance::{Appearance, SCALE, builtin};
use crate::browser::{self, Browser};
use crate::faces;
use crate::fields;
use crate::inspector::Inspector;
use crate::layout::{GalleryWorkspace, PanelId, PreviewView, VariantAxis, default_for, reconciled};
use crate::persist::{self, KEY, Saved};
use crate::preview::AppPreview;
use crate::problems::Problems;

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
    /// The component the Overview has selected: its tile has the ring, and the side panels follow
    /// it while the Overview is the front centre tab.
    overview_selected: Option<&'static str>,
    /// The story id each Overview tile shows, by component; a component without an entry shows
    /// its default story.
    overview_story: HashMap<&'static str, &'static str>,
    /// The selected story id of each component tab; a tab without an entry has its default story
    /// selected.
    tab_story: HashMap<&'static str, &'static str>,
    filter: String,
    /// The axis the Variants view repeats the story over; not saved.
    variants: Option<VariantAxis>,
    /// The centre tab Variants shows in; it closes when another centre tab comes to the front.
    variants_tab: PanelId,
    /// The More popover of the options bar is open.
    more_open: bool,
    /// The tile to scroll to, until it has been drawn: a component in the Overview, a story id in
    /// a component tab.
    scroll_to: Option<&'static str>,
    browser: Browser,
    problems: Problems,
    /// The mock app the Theme workspace previews.
    app: AppPreview,
    inspector: Inspector,
    /// The workspace shown.
    workspace: GalleryWorkspace,
    /// What the Theme workspace's preview shows.
    preview: PreviewView,
    /// The Components workspace's dock: where each panel is, and which tab is shown.
    components_dock: Workspace<PanelId>,
    /// The Theme workspace's dock.
    theme_dock: Workspace<PanelId>,
    /// Reset layout was chosen in this frame.
    reset_layout: bool,
    /// Edited args by story id; a story gets its entry when first shown.
    args: HashMap<String, Args>,
    saved_args: HashMap<String, Vec<persist::SavedArg>>,
}

impl Gallery {
    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        let saved = Saved::load(storage);
        let stories = stories();
        let component = |name: &str| stories.iter().find(|s| s.component == name).map(|s| s.component);
        let story = |component: &str, id: &str| stories.iter().find(|s| s.component == component && s.id == id).map(|s| (s.component, s.id));
        let mut overview_selected = saved.overview_selected.as_deref().and_then(component);
        let mut overview_story: HashMap<&'static str, &'static str> = saved.overview_story.iter().filter_map(|(c, id)| story(c, id)).collect();
        // A state from before the Overview kept its own selection holds one selected story.
        if overview_selected.is_none()
            && let Some(old) = saved.selected.as_deref().and_then(|id| stories.iter().find(|s| s.id == id))
        {
            overview_selected = Some(old.component);
            overview_story.entry(old.component).or_insert(old.id);
        }
        let tab_story: HashMap<&'static str, &'static str> = saved.tab_story.iter().filter_map(|(c, id)| story(c, id)).collect();
        let components_dock = load_dock(persist::load_layout(storage), GalleryWorkspace::Components);
        let tab_story = tab_story.into_iter().filter(|(c, _)| components_dock.layout.contains(&PanelId::Component(c))).collect();
        Self {
            stories,
            base: saved.appearance.base(SystemAppearance::default()),
            appearance: saved.appearance,
            applied: None,
            story_ppp: None,
            system_face_applied: false,
            os: SystemReader::for_egui(),
            os_applied: SystemAppearance::default(),
            overview_selected,
            overview_story,
            tab_story,
            filter: saved.filter,
            variants: None,
            variants_tab: PanelId::View,
            more_open: false,
            scroll_to: None,
            browser: Browser::new(saved.tokens_split),
            problems: Problems::default(),
            app: AppPreview::default(),
            inspector: Inspector::default(),
            workspace: saved.workspace,
            preview: saved.preview,
            components_dock,
            theme_dock: load_dock(persist::load_theme_layout(storage), GalleryWorkspace::Theme),
            reset_layout: false,
            args: HashMap::new(),
            saved_args: saved.args,
        }
    }

    /// Switches to `workspace`.
    pub fn set_workspace(&mut self, workspace: GalleryWorkspace) {
        self.workspace = workspace;
    }

    pub fn workspace(&self) -> GalleryWorkspace {
        self.workspace
    }

    /// Sets what the Theme workspace's preview shows.
    pub fn set_preview(&mut self, preview: PreviewView) {
        self.preview = preview;
    }

    /// Selects a story by id in the Overview, and in the component's tab when it is open; unknown
    /// ids are ignored.
    pub fn select(&mut self, id: &str) {
        let Some(story) = self.stories.iter().find(|s| s.id == id).copied() else { return };
        self.overview_selected = Some(story.component);
        self.overview_story.insert(story.component, story.id);
        if self.components_dock.layout.contains(&PanelId::Component(story.component)) {
            self.tab_story.insert(story.component, story.id);
        }
    }

    /// The component the Overview has selected.
    pub fn overview_selected(&self) -> Option<&'static str> {
        self.overview_selected
    }

    /// The id of the story the side panels follow: the selected tile's in the Overview, the
    /// selected story's in a component tab.
    pub fn selected(&self) -> Option<&'static str> {
        self.target().map(|s| s.id)
    }

    /// The centre tab in front: the Overview, or a component's tab. With several centre tabs in
    /// front (a floated one), the Overview and then the first in the layout.
    pub fn front_tab(&self) -> PanelId {
        let layout = &self.components_dock.layout;
        if layout.panel_state(&PanelId::View) == PanelState::Front {
            return PanelId::View;
        }
        layout
            .panels()
            .into_iter()
            .copied()
            .find(|p| matches!(p, PanelId::Component(_)) && layout.panel_state(p) == PanelState::Front)
            .unwrap_or(PanelId::View)
    }

    /// The args of the story with id `id`, which the Properties panel edits.
    pub fn args_of_story(&mut self, id: &str) -> Option<&mut Args> {
        let story = self.stories.iter().find(|s| s.id == id).copied()?;
        Some(self.args_of(&story))
    }

    /// The components that have a tab, in the layout's order.
    pub fn open_tabs(&self) -> Vec<&'static str> {
        self.components_dock
            .layout
            .panels()
            .into_iter()
            .filter_map(|p| match p {
                PanelId::Component(c) => Some(*c),
                _ => None,
            })
            .collect()
    }

    /// Opens `component`'s tab in the Components workspace and fronts it. A tab that is open comes
    /// to the front; a new one joins the group of the Overview.
    pub fn open_component(&mut self, component: &'static str) {
        self.workspace = GalleryWorkspace::Components;
        self.variants = None;
        let panel = PanelId::Component(component);
        let _ = self.components_dock.set_visible(panel, true, Some(PanelId::View), PanelId::permissions);
    }

    /// Shows the Variants view over `axis` in the front centre tab, or closes it. Without a
    /// selected component there is nothing to repeat, so it stays closed.
    pub fn set_variants(&mut self, axis: Option<VariantAxis>) {
        self.variants = axis.filter(|_| self.target().is_some());
        self.variants_tab = self.front_tab();
    }

    pub fn variants(&self) -> Option<VariantAxis> {
        self.variants
    }

    /// Opens or closes the options bar's More popover.
    pub fn set_more_open(&mut self, open: bool) {
        self.more_open = open;
    }

    /// Shows the inspector tab, which turns inspect mode on, instead of the properties tab.
    pub fn set_inspect(&mut self, inspect: bool) {
        self.activate(if inspect { PanelId::Inspect } else { PanelId::Properties });
    }

    /// Brings `panel` to the front of its tab group.
    pub fn activate(&mut self, panel: PanelId) {
        let _ = self.dock_of_mut(panel.workspace()).apply(Action::Activate { panel }, PanelId::permissions);
    }

    /// Whether `panel` is the shown tab of its group in its workspace's dock.
    pub fn is_front(&self, panel: PanelId) -> bool {
        let dock = match panel.workspace() {
            GalleryWorkspace::Components => &self.components_dock,
            GalleryWorkspace::Theme => &self.theme_dock,
        };
        dock.layout.panel_state(&panel) == PanelState::Front
    }

    fn dock_of_mut(&mut self, workspace: GalleryWorkspace) -> &mut Workspace<PanelId> {
        match workspace {
            GalleryWorkspace::Components => &mut self.components_dock,
            GalleryWorkspace::Theme => &mut self.theme_dock,
        }
    }

    /// The labels of the focus-order badges of each canvas the inspector drew in the last frame.
    pub fn focus_badges(&self) -> Vec<Vec<String>> {
        self.inspector.cells().iter().map(|c| c.badges.clone()).collect()
    }

    /// Selects a row of the token browser.
    pub fn select_token(&mut self, token: craft_theme::TokenRef) {
        self.browser.select(token);
    }

    /// The row selected in the token browser, which the Tokens tab and the Theme panel share.
    pub fn selected_token(&self) -> Option<craft_theme::TokenRef> {
        self.browser.selected()
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
        // Esc closes Variants unless it closes a popup first; the bar draws before the centre, so
        // the popups are read now.
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !egui::Popup::is_any_open(&ctx) && !popover::any_open(&ctx);
        Panel::top("gallery.bar").frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| self.options_bar(ui));
        if (escape && self.workspace == GalleryWorkspace::Components) || self.front_tab() != self.variants_tab {
            self.variants = None;
        }
        let inspecting = self.workspace == GalleryWorkspace::Components
            && self.variants.is_none()
            && self.target().is_some()
            && self.components_dock.layout.panel_state(&PanelId::Inspect) == PanelState::Front;
        inspect::set_enabled(&ctx, inspecting);
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
        }
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| self.dock(ui, inspecting));
        if std::mem::take(&mut self.reset_layout) {
            let shown = self.workspace;
            *self.dock_of_mut(shown) = default_for(shown);
        }
        self.forget_closed_tabs();
    }

    /// Drops what a component tab that is no longer in the layout kept: its selected story, and
    /// the place the dock recorded for it. A closed tab does not come back from `hidden`.
    fn forget_closed_tabs(&mut self) {
        let layout = &self.components_dock.layout;
        self.tab_story.retain(|c, _| layout.contains(&PanelId::Component(c)));
        self.components_dock.hidden.retain(|(panel, _)| !matches!(panel, PanelId::Component(_)));
    }

    /// The shown workspace's panels in their dock. The panels draw from a copy of the layout,
    /// because they borrow the gallery and may change its dock (opening a tab); the dock's own
    /// actions of this frame are applied to the gallery's layout afterwards.
    fn dock(&mut self, ui: &mut Ui, inspecting: bool) {
        let shown = self.workspace;
        let front = self.front_tab();
        let workspace = self.dock_of_mut(shown).clone();
        let id = match shown {
            GalleryWorkspace::Components => "gallery.dock.components",
            GalleryWorkspace::Theme => "gallery.dock.theme",
        };
        let tokens = Tokens::of(ui);
        let mut config = DockConfig::default();
        config.right_to_left = direction::is_rtl(ui);
        let mut panels = Panels { gallery: self, inspecting, control: tokens.size.control.sm };
        let output = DockArea::new(ui.make_persistent_id(id)).locked(workspace.locked).show_content(ui, &workspace.layout, &config, &mut panels);
        for action in output.actions {
            let _ = self.dock_of_mut(shown).apply(action, PanelId::permissions);
        }
        // Closing the tab in front shows the Overview.
        if shown == GalleryWorkspace::Components && !self.components_dock.layout.contains(&front) {
            self.activate(PanelId::View);
        }
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
        let has_target = self.target().is_some();
        let front = self.front_tab();
        let a = &mut self.appearance;
        let variants_tab = &mut self.variants_tab;
        let workspace = &mut self.workspace;
        let current = *workspace;
        let preview = &mut self.preview;
        let variants = &mut self.variants;
        let more_open = &mut self.more_open;
        let reset_layout = &mut self.reset_layout;
        let numbers = LocaleNumbers::of(ui);
        let faces_shown =
            faces::available().then(|| (faces::shown_ui_face(ui.ctx(), &a.ui_face), faces::shown_mono_face(&a.mono_face), faces::ui_faces(ui.ctx())));
        let icon_set_id = a.icon_set().id().to_owned();
        OptionsBar::new("gallery.options").label("Appearance").show(ui, |bar| {
            bar.row(|ui| {
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
                let more = ui.add(IconButton::new(Icon::More, "More appearance settings").size(ControlSize::Sm));
                if more.clicked() {
                    *more_open = !*more_open;
                }
                Popover::new("gallery.more", PopoverAnchor::Control(more.clone())).toggle(more.id).label("More appearance settings").show(
                    ui,
                    more_open,
                    |ui, _| {
                        FormGrid::new("gallery.more.form").show(ui, |form| {
                            form.row("Reduced motion", |ui, _| {
                                choice(
                                    ui,
                                    "Reduced motion",
                                    &mut a.motion,
                                    [("Off", MotionChoice::Full), ("On", MotionChoice::Reduced), ("System", MotionChoice::System)],
                                )
                            });
                            form.row("Direction", |ui, _| choice(ui, "Direction", &mut a.rtl, [("LTR", false), ("RTL", true)]));
                            form.row("Text", |ui, _| {
                                let mut text = Segmented::new("gallery.text_size", "Text", &mut a.text_size).size(ControlSize::Sm);
                                for (i, size) in TextSize::ALL.iter().enumerate() {
                                    text = text.item(i, text_size_name(*size));
                                }
                                text.show(ui)
                            });
                            if let Some((ui_shown, mono_shown, ui_faces)) = &faces_shown {
                                form.row("UI face", |ui, _| face_select(ui, "UI face", "gallery.ui_face", &mut a.ui_face, ui_shown, ui_faces));
                                form.row("Mono face", |ui, _| {
                                    face_select(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, mono_shown, faces::MONO_FACES)
                                });
                            }
                            form.row("Icons", |ui, _| {
                                let mut icon_set = icon_set_id;
                                let mut icons = Select::new("gallery.icon_set", "Icons", &mut icon_set).fit(true).size(ControlSize::Sm);
                                for set in IconSet::ALL {
                                    icons = icons.option(set.id().to_owned(), set.title());
                                }
                                let shown = icons.policy(fields::policy()).show(ui);
                                if shown.response.changed() {
                                    a.icon_set = icon_set;
                                }
                                shown.response
                            });
                            form.row("Scale", |ui, label| {
                                let mut field = NumberField::new("gallery.scale", "Scale", a.scale, &numbers)
                                    .range(SCALE)
                                    .decimals(0..=0)
                                    .unit(Unit::new("{n}%"))
                                    .popup_slider("Scale slider")
                                    .size(ControlSize::Sm)
                                    .policy(fields::policy());
                                if let Some(label) = label.scrub() {
                                    field = field.scrub_by(label);
                                }
                                let scale = field.show(ui);
                                let mut percent = f64::from(a.scale);
                                if fields::apply(&scale, &mut percent) {
                                    a.scale = percent.round() as u32;
                                }
                                scale.response
                            });
                        });
                    },
                );
                OptionsBar::divider(ui);
                match current {
                    GalleryWorkspace::Components => {
                        // The toggle only shows the state; the menu opens on the press.
                        let mut shown = variants.is_some();
                        let button = ui.add(ToggleButton::new("Variants", &mut shown).trailing_icon(Icon::ChevronDown).size(ControlSize::Sm));
                        let button = if has_target { button } else { button.tooltip("Select a component first") };
                        let entries: Vec<MenuEntry<VariantAxis>> =
                            VariantAxis::ALL.into_iter().map(|axis| MenuItem::new(axis.menu_label(), axis).enabled(has_target).into()).collect();
                        if let Some(axis) = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen {
                            *variants = Some(axis);
                            *variants_tab = front;
                        }
                    }
                    GalleryWorkspace::Theme => {
                        segmented(ui, "Preview", preview, [("App", PreviewView::App), ("Components", PreviewView::Components)]);
                        let mut picking = false;
                        ui.add(ToggleButton::new("Pick from preview", &mut picking).icon(Icon::Eyedropper).enabled(false).size(ControlSize::Sm))
                            .tooltip("Comes with token picking");
                    }
                }
            });
            bar.trailing(|ui| {
                if current == GalleryWorkspace::Theme {
                    ui.add(Button::new("Revert all").enabled(false).size(ControlSize::Sm)).tooltip("Comes with theme editing");
                    ui.add(Button::new("Save as…").variant(ButtonVariant::Primary).enabled(false).size(ControlSize::Sm))
                        .tooltip("Comes with theme editing");
                    OptionsBar::divider(ui);
                }
                let reset = ui.add(IconButton::new(Icon::Undo, "Reset layout").show_tooltip(false).size(ControlSize::Sm));
                if reset.tooltip("Reset this workspace's layout").clicked() {
                    *reset_layout = true;
                }
                OptionsBar::cell(ui, "Workspace:", CellWidth::Content, |ui, _| {
                    let mut segmented = Segmented::new(("gallery.segmented", "Workspace"), "Workspace", workspace).size(ControlSize::Sm);
                    for option in GalleryWorkspace::ALL {
                        segmented = segmented.item(option, option.title());
                    }
                    segmented.show(ui);
                });
            });
        });
    }

    /// The story of `component` that `picks` names, or its default story.
    fn story_in(&self, picks: &HashMap<&'static str, &'static str>, component: &str) -> Option<Story> {
        picks.get(component).and_then(|id| self.stories.iter().find(|s| s.id == *id)).or_else(|| default_story_of(&self.stories, component)).copied()
    }

    /// The story the side panels follow: the Overview's selected tile's, or the front component
    /// tab's selected story. None while the Overview has no selection.
    fn target(&self) -> Option<Story> {
        match self.front_tab() {
            PanelId::Component(component) => self.story_in(&self.tab_story, component),
            _ => self.overview_selected.and_then(|component| self.story_in(&self.overview_story, component)),
        }
    }

    /// The components that have a story matching the filter, under a label for each level. A click
    /// selects a component in the Overview; Open, Enter and a double click open its tab.
    fn components_panel(&mut self, ui: &mut Ui) {
        let filter = SearchField::new("gallery.filter", "Filter components", &self.filter, "Clear filter")
            .placeholder("Filter components")
            .policy(fields::policy())
            .show(ui);
        fields::apply(&filter, &mut self.filter);
        let filter = self.filter.to_lowercase();
        let current = self.target().map(|s| s.component);
        let mut rows: Vec<RowInfo<Entry>> = Vec::new();
        for (level, components) in nav(&self.stories, &filter) {
            rows.push(RowInfo::section(Entry::Level(level)));
            rows.extend(components.into_iter().map(|c| RowInfo::leaf(Entry::Component(c))));
        }
        let mut selection =
            current.map(Entry::Component).filter(|e| rows.iter().any(|r| r.key == *e)).map_or_else(Selection::default, Selection::only);
        let open_button = Tokens::of(ui).size.control.sm;
        let opened = Cell::new(None);
        let report = &opened;
        let out = List::new("gallery.components", "Components", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .type_ahead(true)
            .show(ui, |_, entry| match entry {
                Entry::Level(level) => ListRow::new(*entry, level.title()),
                Entry::Component(component) => {
                    let (component, on) = (*component, current == Some(*component));
                    ListRow::new(*entry, component).trailing_item(open_button, move |ui| {
                        // The item is as tall as the row; the list's clip is as wide.
                        let row = Rect::from_x_y_ranges(ui.clip_rect().x_range(), ui.max_rect().y_range());
                        if (on || ui.rect_contains_pointer(row))
                            && ui.add(IconButton::new(Icon::Open, format!("Open {component}")).size(ControlSize::Sm)).clicked()
                        {
                            report.set(Some(component));
                        }
                    })
                }
            });
        let mut picked = None;
        let mut activated = opened.get();
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(Entry::Component(c)), .. }) => picked = Some(c),
                ListEvent::Activate(Entry::Component(c)) => activated = Some(c),
                _ => {}
            }
        }
        if let Some(component) = picked {
            self.select_component(component);
        }
        if let Some(component) = activated {
            self.open_component(component);
        }
    }

    /// Selects `component` in the Overview, which comes to the front and scrolls to its tile.
    fn select_component(&mut self, component: &'static str) {
        self.overview_selected = Some(component);
        self.scroll_to = Some(component);
        self.activate(PanelId::View);
    }

    /// The target component's stories. In the Overview a pick changes what the selected tile
    /// shows; in a component tab it selects that story's tile.
    fn stories_panel(&mut self, ui: &mut Ui) {
        let Some(target) = self.target() else {
            select_a_component(ui);
            return;
        };
        let stories: Vec<Story> = self.stories.iter().filter(|s| s.component == target.component).copied().collect();
        let rows: Vec<RowInfo<&'static str>> = stories.iter().map(|s| RowInfo::leaf(s.id)).collect();
        let mut selection = Selection::only(target.id);
        let out = List::new("gallery.stories", "Stories", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .show(ui, |_, id| ListRow::new(*id, stories.iter().find(|s| s.id == *id).map_or(*id, |s| s.name)));
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(id), .. }) | ListEvent::Activate(id) => {
                    if matches!(self.front_tab(), PanelId::Component(_)) {
                        self.tab_story.insert(target.component, id);
                        self.scroll_to = Some(id);
                    } else {
                        self.overview_story.insert(target.component, id);
                    }
                }
                _ => {}
            }
        }
    }

    /// The args of `story`, from the saved values on first use.
    fn args_of(&mut self, story: &Story) -> &mut Args {
        args_of(&mut self.args, &self.saved_args, story)
    }

    /// Puts the args of the story with id `id` back to its defaults.
    fn reset_story(&mut self, id: &str) {
        if let Some(story) = self.stories.iter().find(|s| s.id == id) {
            self.args.insert(story.id.to_owned(), story.defaults());
            self.saved_args.remove(story.id);
        }
    }

    fn reset_args(&mut self) {
        if let Some(story) = self.target() {
            self.reset_story(story.id);
        }
    }

    /// The target story's args, under a header with its name and Reset.
    fn properties_panel(&mut self, ui: &mut Ui) {
        let Some(story) = self.target() else {
            select_a_component(ui);
            return;
        };
        let changed = *self.args_of(&story) != story.defaults();
        let mut reset = false;
        direction::horizontal(ui, |ui| {
            ui.add(Text::new(story.component).role(TypeRole::Heading));
            ui.add(Text::new(story.name).content(Content::Muted));
            if changed {
                at_trailing_end(ui, |ui| reset = ui.add(IconButton::new(Icon::Undo, "Reset args").size(ControlSize::Sm)).clicked());
            }
        });
        if reset {
            self.reset_story(story.id);
        }
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

    /// The Variants view of the target story, in the front centre tab.
    fn variants_view(&mut self, ui: &mut Ui, axis: VariantAxis) {
        let Some(story) = self.target() else {
            self.variants = None;
            return;
        };
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let args = args_of(&mut self.args, &self.saved_args, &story);
        let mut applied = None;
        let mut closed = false;
        canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            closed = variants_header(ui, &story, axis);
            ScrollArea::vertical().id_salt("gallery.variants").auto_shrink([false, false]).show(ui, |ui| {
                applied = variants_grid(ui, &story, args, &appearance, system, axis);
            });
        });
        if let Some(from) = applied {
            copy_axis(axis, &from, &mut self.appearance);
            closed = true;
        }
        if closed {
            self.variants = None;
        }
    }

    /// The Variants view when it shows in `tab`.
    fn variants_in(&self, tab: PanelId) -> Option<VariantAxis> {
        self.variants.filter(|_| self.variants_tab == tab)
    }

    /// Applies what a set of tiles reported, other than a pick, which the tab interprets.
    fn apply_tile_events(&mut self, events: &TileEvents) {
        if let Some(id) = events.reset {
            self.reset_story(id);
        }
        if let Some(component) = events.opened {
            self.open_component(component);
        }
    }

    /// The Overview tab: every component of one level, each tile on its story's surface. The
    /// level follows the selected component; each tile draws the story it was given.
    fn overview_tab(&mut self, ui: &mut Ui, inspecting: bool) {
        if let Some(axis) = self.variants_in(PanelId::View) {
            self.variants_view(ui, axis);
            return;
        }
        let nav = nav(&self.stories, &self.filter.to_lowercase());
        let selected = self.overview_selected;
        let shown =
            selected.and_then(|c| default_story_of(&self.stories, c)).and_then(|s| nav.iter().position(|(level, _)| *level == s.level)).unwrap_or(0);
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let mut level = shown;
        let mut events = TileEvents::default();
        let tiles = nav.get(shown).map(|(_, components)| self.component_tiles(components)).unwrap_or_default();
        canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let Some((shown_level, _)) = nav.get(shown) else {
                ui.label("No components match the filter.");
                return;
            };
            ui.horizontal(|ui| {
                ui.label("Level:");
                let mut select = Select::new("gallery.overview.level", "Level", &mut level).fit(true).size(ControlSize::Sm);
                for (i, (level, _)) in nav.iter().enumerate() {
                    select = select.option(i, level.title());
                }
                let _ = select.policy(fields::policy()).show(ui);
            });
            let inspector = selected.filter(|_| inspecting).map(|c| (c, &mut self.inspector));
            let config = Tiles {
                one_column: false,
                selectable: true,
                selected,
                buttons: true,
                scale_of_selected: false,
                scroll_to: &mut self.scroll_to,
                inspector,
            };
            ScrollArea::vertical().id_salt(shown_level.slug()).auto_shrink([false, false]).show(ui, |ui| {
                events = tiles_view(ui, &tiles, config, &mut self.args, &self.saved_args, &appearance, system);
            });
        });
        if let Some(first) = nav.get(level).and_then(|(_, components)| components.first()).filter(|_| level != shown) {
            self.overview_selected = Some(*first);
        }
        if let Some(component) = events.picked {
            self.overview_selected = Some(component);
        }
        self.apply_tile_events(&events);
    }

    /// One tile per component: the story the Overview shows for it.
    fn component_tiles(&self, components: &[&'static str]) -> Vec<Tile> {
        components
            .iter()
            .filter_map(|c| {
                let story = self.story_in(&self.overview_story, c)?;
                let default = default_story_of(&self.stories, c)?;
                Some(Tile { key: c, story, title: c, note: (story.id != default.id).then_some(story.name), open: true })
            })
            .collect()
    }

    /// A component's tab: every story of the component as a tile in one column.
    fn component_tab(&mut self, ui: &mut Ui, component: &'static str, inspecting: bool) {
        if let Some(axis) = self.variants_in(PanelId::Component(component)) {
            self.variants_view(ui, axis);
            return;
        }
        let tiles: Vec<Tile> = self
            .stories
            .iter()
            .filter(|s| s.component == component)
            .map(|s| Tile { key: s.id, story: *s, title: s.name, note: None, open: false })
            .collect();
        let selected = self.story_in(&self.tab_story, component).map(|s| s.id);
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let mut events = TileEvents::default();
        canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let inspector = selected.filter(|_| inspecting).map(|id| (id, &mut self.inspector));
            let config = Tiles {
                one_column: true,
                selectable: true,
                selected,
                buttons: true,
                scale_of_selected: true,
                scroll_to: &mut self.scroll_to,
                inspector,
            };
            ScrollArea::vertical().id_salt(("gallery.tab", component)).auto_shrink([false, false]).show(ui, |ui| {
                events = tiles_view(ui, &tiles, config, &mut self.args, &self.saved_args, &appearance, system);
            });
        });
        if let Some(id) = events.picked {
            self.tab_story.insert(component, id);
        }
        self.apply_tile_events(&events);
    }

    /// The Theme workspace's centre: the mock app, or the Controls level's tiles.
    fn preview_panel(&mut self, ui: &mut Ui) {
        match self.preview {
            PreviewView::App => self.app.show(ui),
            PreviewView::Components => {
                let components: Vec<&'static str> =
                    nav(&self.stories, "").into_iter().find(|(level, _)| *level == Level::Controls).map_or_else(Vec::new, |(_, c)| c);
                let tiles: Vec<Tile> = components
                    .iter()
                    .filter_map(|c| default_story_of(&self.stories, c).map(|story| Tile { key: c, story: *story, title: c, note: None, open: true }))
                    .collect();
                let appearance = self.appearance.clone();
                let system = self.os_applied;
                let mut events = TileEvents::default();
                canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    let mut scroll_to = None;
                    let config = Tiles {
                        one_column: false,
                        selectable: false,
                        selected: None,
                        buttons: true,
                        scale_of_selected: false,
                        scroll_to: &mut scroll_to,
                        inspector: None,
                    };
                    ScrollArea::vertical().id_salt("gallery.preview").auto_shrink([false, false]).show(ui, |ui| {
                        events = tiles_view(ui, &tiles, config, &mut self.args, &self.saved_args, &appearance, system);
                    });
                });
                self.apply_tile_events(&events);
            }
        }
    }

    fn tokens_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        self.browser.show(ui, &self.base, mode);
    }

    /// The Theme workspace's Theme panel: the theme's basics above the token browser, which is
    /// the one the Tokens tab draws.
    fn theme_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let theme = self.base.inputs.theme(mode).clone();
        let max = (ui.available_height() * BASICS_SHARE).round();
        ScrollArea::vertical().id_salt("gallery.theme.basics.scroll").max_height(max).auto_shrink([false, true]).show(ui, |ui| {
            browser::basics(ui, &theme, mode);
        });
        self.browser.show(ui, &self.base, mode);
    }

    /// The Problems panel; a click on a row selects its foreground token in the Theme panel.
    fn problems_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let explained = self.browser.explained(&self.base, mode);
        if let Some(token) = self.problems.show(ui, &self.base, mode, &explained) {
            self.browser.select(token);
            self.activate(PanelId::Theme);
        }
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

/// What the side panels say while nothing is selected.
fn select_a_component(ui: &mut Ui) {
    EmptyState::new("Select a component").show(ui);
}

/// Runs `add` at the trailing end of the rest of the row.
fn at_trailing_end(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let layout = if direction::is_rtl(ui) { egui::Layout::left_to_right(Align::Center) } else { egui::Layout::right_to_left(Align::Center) };
    ui.with_layout(layout, add);
}

/// A panel that is not built yet: one muted line.
fn placeholder(ui: &mut Ui, text: &str) {
    padded(ui, |ui| {
        EmptyState::new(text).show(ui);
    });
}

/// The share of the Theme panel's height the Basics section may take.
const BASICS_SHARE: f32 = 0.4;

/// The body height of the Theme workspace's Problems and Changes group.
const PROBLEMS_HEIGHT: f32 = 210.0;

/// The dock layout `saved` holds, made valid for `workspace`, or the default.
fn load_dock(saved: Option<Workspace<PanelId>>, workspace: GalleryWorkspace) -> Workspace<PanelId> {
    saved.map_or_else(|| default_for(workspace), |saved| reconciled(saved, workspace))
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
            PanelId::Inspect if self.gallery.target().is_none() => padded(ui, select_a_component),
            PanelId::Inspect if self.gallery.variants.is_some() => placeholder(ui, "Inspect is off while Variants shows."),
            PanelId::Inspect => padded(ui, |ui| self.gallery.inspector.panel(ui)),
            PanelId::View => self.gallery.overview_tab(ui, self.inspecting),
            PanelId::Component(component) => self.gallery.component_tab(ui, component, self.inspecting),
            PanelId::Tokens => padded(ui, |ui| self.gallery.tokens_panel(ui)),
            PanelId::Preview => self.gallery.preview_panel(ui),
            PanelId::Theme => padded(ui, |ui| self.gallery.theme_panel(ui)),
            PanelId::Problems => padded(ui, |ui| self.gallery.problems_panel(ui)),
            PanelId::Changes => placeholder(ui, "Changes since the last save come here."),
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
            PanelId::Stories => Some(DockHeights::new(80.0).compact(60.0)),
            PanelId::Properties | PanelId::Inspect => Some(DockHeights::new(240.0).compact(120.0).fill(300.0)),
            PanelId::Theme => Some(DockHeights::new(0.0).fill(300.0)),
            PanelId::Problems | PanelId::Changes => Some(DockHeights::new(PROBLEMS_HEIGHT)),
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
            selected: None,
            overview_selected: self.overview_selected.map(str::to_owned),
            overview_story: self.overview_story.iter().map(|(c, id)| ((*c).to_owned(), (*id).to_owned())).collect(),
            tab_story: self.tab_story.iter().map(|(c, id)| ((*c).to_owned(), (*id).to_owned())).collect(),
            filter: self.filter.clone(),
            workspace: self.workspace,
            preview: self.preview,
            tokens_split: Some(self.browser.share()),
            args: saved_args,
        };
        eframe::set_value(storage, KEY, &saved);
        eframe::set_value(storage, persist::LAYOUT_KEY, &self.components_dock);
        eframe::set_value(storage, persist::THEME_LAYOUT_KEY, &self.theme_dock);
    }
}

/// The args of `story` in `args`, from the saved values on first use.
pub(crate) fn args_of<'a>(args: &'a mut HashMap<String, Args>, saved: &HashMap<String, Vec<persist::SavedArg>>, story: &Story) -> &'a mut Args {
    args.entry(story.id.to_owned()).or_insert_with(|| match saved.get(story.id) {
        Some(values) => persist::restore_args(story.defaults(), values),
        None => story.defaults(),
    })
}

/// Every story the gallery shows: the library's and the gallery's own.
pub(crate) fn stories() -> Vec<Story> {
    let mut stories = registry();
    stories.extend(faces::stories());
    stories.extend(crate::preview::stories());
    stories
}

/// The story the Overview draws and the Components panel opens for `component`: its first story,
/// which shows the component at rest.
pub(crate) fn default_story_of<'a>(stories: &'a [Story], component: &str) -> Option<&'a Story> {
    stories.iter().find(|s| s.component == component)
}

/// The navigation's levels in their order, each with the components that have a story matching
/// `filter` (lowercase), sorted by name ignoring case, whatever the registration order.
pub(crate) fn nav(stories: &[Story], filter: &str) -> Vec<(Level, Vec<&'static str>)> {
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
fn face_select(ui: &mut Ui, label: &str, id: &str, value: &mut String, shown: &str, options: &[&str]) -> egui::Response {
    let mut pick = shown.to_owned();
    let mut select = Select::new(id, label, &mut pick).fit(true).size(ControlSize::Sm);
    for option in options {
        select = select.option((*option).to_owned(), *option);
    }
    let out = select.policy(fields::policy()).show(ui);
    if out.response.changed() {
        *value = pick;
    }
    out.response
}

/// A captioned set of exclusive choices in the options bar.
fn segmented<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) {
    OptionsBar::cell(ui, &format!("{label}:"), CellWidth::Content, |ui, _| {
        choice(ui, label, value, options);
    });
}

/// Exclusive choices as a segmented control.
fn choice<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) -> egui::Response {
    let mut segmented = Segmented::new(("gallery.segmented", label), label, value).size(ControlSize::Sm);
    for (name, option) in options {
        segmented = segmented.item(option, name);
    }
    segmented.show(ui)
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

/// The dock colour with the panel padding around what a view holds.
pub(crate) fn canvas_frame(tokens: &Tokens) -> Frame {
    Frame::new().fill(tokens.color.surface.background.to_egui()).inner_margin(margin(tokens.space.pad.panel))
}

/// How a story is framed: the fill and edge of its surface, and the margin round the story.
pub(crate) struct StoryFrame {
    fill: Option<Color32>,
    edge: Stroke,
    margin: Margin,
    /// The gap between stacked controls, when the surface is a panel body.
    body_gap: Option<f32>,
}

impl StoryFrame {
    /// The frame of `surface` under `ui`'s theme. A panel is a dock panel body with its margin and
    /// item spacing, edge and fill from [`DockLook`]; canvas and chrome are their surface colours
    /// with the dock's edge; an own surface adds nothing.
    pub(crate) fn of(ui: &Ui, surface: Surface) -> Self {
        let tokens = Tokens::of(ui);
        let look = DockLook::of(ui);
        let surfaces = &tokens.color.surface;
        let bar = |fill: Color32| Self { fill: Some(fill), edge: look.border, margin: margin(tokens.space.pad.dialog), body_gap: None };
        match surface {
            Surface::Panel => {
                Self { fill: Some(look.background), edge: look.border, margin: margin(tokens.space.pad.panel), body_gap: Some(look.body_gap) }
            }
            Surface::Canvas => bar(surfaces.canvas.to_egui()),
            Surface::Chrome => bar(surfaces.chrome.to_egui()),
            Surface::Own => Self { fill: None, edge: Stroke::NONE, margin: margin(tokens.space.pad.dialog), body_gap: None },
        }
    }

    /// The frame to show the story in.
    fn frame(&self) -> Frame {
        let frame = Frame::new().inner_margin(self.margin);
        match self.fill {
            Some(fill) => frame.fill(fill).stroke(self.edge),
            None => frame,
        }
    }

    /// Applies the surface's item spacing to the story's `ui`.
    fn space(&self, ui: &mut Ui) {
        if let Some(gap) = self.body_gap {
            ui.spacing_mut().item_spacing.y = gap;
        }
    }
}

/// The story on its surface under the bar's appearance.
pub(crate) fn canvas(
    ui: &mut Ui,
    story: &Story,
    args: &mut Args,
    appearance: &Appearance,
    system: SystemAppearance,
    inspector: Option<&mut Inspector>,
) {
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
    let surface = StoryFrame::of(ui, story.surface);
    let framed = surface.frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        surface.space(ui);
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

/// One tile of an Overview or a component tab.
#[derive(Clone, Copy)]
pub(crate) struct Tile {
    /// What a pick and a scroll request name: the component in the Overview, the story's id in a
    /// component tab.
    pub key: &'static str,
    pub story: Story,
    /// The header's heading: the component, or the story.
    pub title: &'static str,
    /// The story's name after the heading, when the tile shows other than the default story.
    pub note: Option<&'static str>,
    /// Whether the header has an Open button (for the component).
    pub open: bool,
}

/// How a set of tiles looks and what it reports.
pub(crate) struct Tiles<'a> {
    /// One full-width column; otherwise one or two, by the width.
    pub one_column: bool,
    /// Whether a click on a tile selects it. Without it no tile has the ring.
    pub selectable: bool,
    /// The key of the tile with the ring.
    pub selected: Option<&'static str>,
    /// Whether the headers have the Reset and Open buttons.
    pub buttons: bool,
    /// The key of the tile to scroll to; cleared once its tile has been drawn.
    pub scroll_to: &'a mut Option<&'static str>,
    /// Whether the selected tile's request for a scale ([`craft_ui::story::request_pixels_per_point`])
    /// stands; every other tile's is dropped, as a tile does not change the gallery's scale.
    pub scale_of_selected: bool,
    /// The inspector, and the key of the one tile it draws over.
    pub inspector: Option<(&'static str, &'a mut Inspector)>,
}

/// What a set of tiles reported in a frame.
#[derive(Default)]
pub(crate) struct TileEvents {
    /// The key of the tile that was clicked.
    pub picked: Option<&'static str>,
    /// The component whose Open was pressed.
    pub opened: Option<&'static str>,
    /// The id of the story whose Reset was pressed.
    pub reset: Option<&'static str>,
}

/// The narrowest a tile may be: PhotoCraft's dock panels are 290 pt wide, and a story drawn in
/// less than a panel's width is no longer what the apps show.
const TILE_MIN_WIDTH: f32 = 290.0;

/// The tiles in one or two columns: two once each column keeps [`TILE_MIN_WIDTH`]. A row's tiles
/// share its height. A tile has a header (the heading, Reset while the story's args differ from
/// its defaults, Open) above its story; a click on the tile that none of its controls takes picks
/// it.
pub(crate) fn tiles_view(
    ui: &mut Ui,
    tiles: &[Tile],
    mut config: Tiles<'_>,
    args: &mut HashMap<String, Args>,
    saved: &HashMap<String, Vec<persist::SavedArg>>,
    appearance: &Appearance,
    system: SystemAppearance,
) -> TileEvents {
    let shown = shown_mode(ui.ctx());
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        appearance.overrides(o);
        o.mode = Some(appearance.mode(shown));
        o.contrast = Some(appearance.contrast(system));
    };
    // The ring belongs to the gallery, not to the appearance the stories are shown in.
    let ring = Tokens::of(ui).color.accent.border.to_egui();
    let mut events = TileEvents::default();
    scope(ui, overrides, |ui| {
        let tokens = Tokens::of(ui);
        let gap = tokens.space.gap.section;
        let rtl = direction::is_rtl(ui);
        let width = ui.available_width();
        let columns = if config.one_column || width < 2.0 * TILE_MIN_WIDTH + gap { 1 } else { 2 };
        let tile_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let area = ui.available_rect_before_wrap();
        // A tile is as tall as its story; a story that fills the height it is given gets a viewport's.
        let reach = area.height();
        for row in tiles.chunks(columns) {
            let top = ui.cursor().min.y;
            let mut bottom = top;
            let mut placed = Vec::new();
            for (i, tile) in row.iter().enumerate() {
                let story = &tile.story;
                let x = if rtl { area.max.x - (i + 1) as f32 * tile_width - i as f32 * gap } else { area.min.x + i as f32 * (tile_width + gap) };
                let slot = ui.painter().add(Shape::Noop);
                let sense = if config.selectable { Sense::CLICK } else { Sense::hover() };
                let mut cell = ui.new_child(
                    UiBuilder::new().id_salt(tile.key).sense(sense).max_rect(Rect::from_min_max(pos2(x, top), pos2(x + tile_width, top + reach))),
                );
                let surface = StoryFrame::of(&cell, story.surface);
                let used = Frame::new().inner_margin(surface.margin).show(&mut cell, |ui| {
                    surface.space(ui);
                    ui.set_min_width(ui.available_width());
                    let args = args_of(args, saved, story);
                    let changed = *args != story.defaults();
                    direction::horizontal(ui, |ui| {
                        ui.add(Text::new(tile.title).role(TypeRole::Heading));
                        if let Some(note) = tile.note {
                            ui.add(Text::new(format!("· {note}")).content(Content::Muted));
                        }
                        if config.buttons {
                            at_trailing_end(ui, |ui| {
                                if tile.open && ui.add(IconButton::new(Icon::Open, format!("Open {}", tile.title)).size(ControlSize::Sm)).clicked() {
                                    events.opened = Some(story.component);
                                }
                                if changed && ui.add(IconButton::new(Icon::Undo, format!("Reset {}", tile.title)).size(ControlSize::Sm)).clicked() {
                                    events.reset = Some(story.id);
                                }
                            });
                        }
                    });
                    // Only the selected tile of a component tab sets the gallery's scale; the request
                    // pending before this tile drew stays.
                    let sets_scale = config.scale_of_selected && config.selected == Some(tile.key);
                    let pending = if sets_scale { None } else { craft_ui::story::take_pixels_per_point_request(ui.ctx()) };
                    let body = ui.push_id(story.id, |ui| {
                        (story.show)(ui, args);
                        ui.unique_id()
                    });
                    if !sets_scale {
                        let _ = craft_ui::story::take_pixels_per_point_request(ui.ctx());
                        if let Some(ppp) = pending {
                            craft_ui::story::request_pixels_per_point(ui.ctx(), ppp);
                        }
                    }
                    if let Some((key, inspector)) = config.inspector.as_mut()
                        && *key == tile.key
                    {
                        inspector.overlay(ui, "Canvas".to_owned(), body.response.rect, body.inner.accesskit_id(), tokens.size.control.sm, rtl);
                    }
                });
                if config.scroll_to.is_some_and(|key| key == tile.key) {
                    cell.scroll_to_rect(used.response.rect, Some(Align::TOP));
                    *config.scroll_to = None;
                }
                let selected = config.selected == Some(tile.key);
                if config.selectable {
                    let response = cell.response();
                    response.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, format!("Select {}", tile.title)));
                    if response.clicked() {
                        events.picked = Some(tile.key);
                    }
                }
                bottom = bottom.max(used.response.rect.bottom());
                placed.push((slot, x, surface, selected));
            }
            for (slot, x, surface, selected) in placed {
                let rect = Rect::from_min_max(pos2(x, top), pos2(x + tile_width, bottom));
                let mut shapes = Vec::new();
                if let Some(fill) = surface.fill {
                    shapes.push(Shape::Rect(RectShape::new(rect, 0.0, fill, surface.edge, StrokeKind::Inside)));
                }
                if selected {
                    shapes.push(Shape::Rect(RectShape::stroke(rect, 0.0, scale::Stroke::W2.egui(ui.ctx(), ring), StrokeKind::Inside)));
                }
                ui.painter().set(slot, Shape::Vec(shapes));
            }
            ui.allocate_rect(Rect::from_min_max(pos2(area.min.x, top), pos2(area.max.x, bottom)), Sense::hover());
            ui.add_space(gap);
        }
    });
    events
}

/// The cards of a Variants view: the appearance with each value of `axis` set, named by the value.
fn variant_values(axis: VariantAxis, appearance: &Appearance) -> Vec<(String, Appearance)> {
    let with = |name: &str, set: &dyn Fn(&mut Appearance)| {
        let mut a = appearance.clone();
        set(&mut a);
        (name.to_owned(), a)
    };
    match axis {
        VariantAxis::Theme => builtins().iter().map(|t| with(&t.name, &|a| a.theme = t.id.as_str().to_owned())).collect(),
        VariantAxis::Mode => vec![with("Light", &|a| a.mode = ModeChoice::Light), with("Dark", &|a| a.mode = ModeChoice::Dark)],
        VariantAxis::Contrast => {
            vec![with("Standard", &|a| a.contrast = ContrastChoice::Standard), with("High", &|a| a.contrast = ContrastChoice::High)]
        }
        VariantAxis::Size => vec![with("Compact", &|a| a.comfortable = false), with("Comfortable", &|a| a.comfortable = true)],
    }
}

/// Sets `to`'s value on `axis` to `from`'s.
fn copy_axis(axis: VariantAxis, from: &Appearance, to: &mut Appearance) {
    match axis {
        VariantAxis::Theme => to.theme.clone_from(&from.theme),
        VariantAxis::Mode => to.mode = from.mode,
        VariantAxis::Contrast => to.contrast = from.contrast,
        VariantAxis::Size => to.comfortable = from.comfortable,
    }
}

/// Whether `a` and `b` show the same value on `axis`; the System choices count as what they show.
fn same_on(axis: VariantAxis, a: &Appearance, b: &Appearance, shown: Mode, system: SystemAppearance) -> bool {
    match axis {
        VariantAxis::Theme => a.theme == b.theme,
        VariantAxis::Mode => a.mode(shown) == b.mode(shown),
        VariantAxis::Contrast => a.contrast(system) == b.contrast(system),
        VariantAxis::Size => a.comfortable == b.comfortable,
    }
}

/// The Variants view's header: what is shown and the hint, with Close at the trailing end. Returns
/// whether Close was clicked.
fn variants_header(ui: &mut Ui, story: &Story, axis: VariantAxis) -> bool {
    let mut closed = false;
    let gap = Tokens::of(ui).space.gap.section;
    direction::horizontal(ui, |ui| {
        ui.add(Text::new(format!("{} · {} in every {}", story.component, story.name, axis.noun())).role(TypeRole::Heading));
        ui.add(Text::new("Click one to use it").content(Content::Muted));
        // The strip's leftover width goes before Close, so it ends at the trailing edge.
        let rtl = direction::is_rtl(ui);
        let layout = if rtl { egui::Layout::left_to_right(Align::Center) } else { egui::Layout::right_to_left(Align::Center) };
        ui.with_layout(layout, |ui| closed = ui.add(Button::new("Close").size(ControlSize::Sm)).clicked());
    });
    ui.add_space(gap);
    closed
}

/// One card per value of `axis`, in one or two columns like the Overview's tiles. Each draws the
/// story as the Component view does, under the gallery's appearance with the value set; the story
/// stays live. Returns the appearance of the card whose header was clicked.
fn variants_grid(
    ui: &mut Ui,
    story: &Story,
    args: &mut Args,
    appearance: &Appearance,
    system: SystemAppearance,
    axis: VariantAxis,
) -> Option<Appearance> {
    let shown = shown_mode(ui.ctx());
    let gap = Tokens::of(ui).space.gap.section;
    let columns = if ui.available_width() >= 2.0 * TILE_MIN_WIDTH + gap { 2 } else { 1 };
    let cards = variant_values(axis, appearance);
    let mut picked = None;
    for row in cards.chunks(columns) {
        ui.columns(columns, |uis| {
            for (ui, (name, card)) in uis.iter_mut().zip(row) {
                // The header sits on the gallery's background, so it keeps the gallery's colours.
                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new(name.as_str()).variant(ButtonVariant::Ghost).size(ControlSize::Sm).id_salt(("gallery.variant", name)))
                        .clicked()
                    {
                        picked = Some(card.clone());
                    }
                    if same_on(axis, card, appearance, shown, system) {
                        ui.add(Text::new("shown now").content(Content::Muted));
                    }
                });
                ui.push_id((axis.noun(), name), |ui| canvas(ui, story, args, card, system, None));
                // A card does not change the gallery's scale.
                let _ = craft_ui::story::take_pixels_per_point_request(ui.ctx());
            }
        });
        ui.add_space(gap);
    }
    picked
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::layout::PanelId;
    use egui::Rect;

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

    /// Frames of the gallery in a 1280 x 800 window.
    fn run_frames(ctx: &egui::Context, gallery: &mut Gallery, frames: usize) {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
        for _ in 0..frames {
            ctx.run_ui(input.clone(), |ui| gallery.show(ui)).drop_without_applying_deltas();
        }
    }

    /// A component with several stories, and its second story.
    fn component_with_stories(gallery: &Gallery) -> (&'static str, Story) {
        let multi = gallery.stories.iter().find(|s| gallery.stories.iter().filter(|o| o.component == s.component).count() > 1).unwrap().component;
        (multi, *gallery.stories.iter().filter(|s| s.component == multi).nth(1).unwrap())
    }

    #[test]
    fn a_fresh_gallery_shows_the_overview_with_nothing_selected() {
        let gallery = Gallery::new(None);
        assert_eq!(gallery.front_tab(), PanelId::View);
        assert_eq!(gallery.overview_selected(), None);
        assert_eq!(gallery.selected(), None);
        assert!(gallery.open_tabs().is_empty());
    }

    #[test]
    fn picking_a_component_scrolls_the_overview_to_it_and_opens_no_tab() {
        let ctx = egui::Context::default();
        let mut gallery = Gallery::new(None);
        // The last component of another level, so the pick changes the level shown.
        let story = gallery.stories.iter().rev().find(|s| s.level != Level::Specimens).copied().unwrap();
        gallery.select_component(story.component);
        assert_eq!(gallery.scroll_to, Some(story.component));
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.scroll_to, None, "the tile was drawn and scrolled to");
        assert_eq!(gallery.front_tab(), PanelId::View);
        assert!(gallery.open_tabs().is_empty());
        assert_eq!(gallery.selected(), default_story_of(&gallery.stories, story.component).map(|s| s.id));
    }

    #[test]
    fn opening_a_component_twice_makes_one_tab_and_closing_it_forgets_its_story() {
        let ctx = egui::Context::default();
        let mut gallery = Gallery::new(None);
        let (component, second) = component_with_stories(&gallery);
        gallery.open_component(component);
        gallery.select(second.id);
        gallery.open_component(component);
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.open_tabs(), [component]);
        assert_eq!(gallery.front_tab(), PanelId::Component(component));
        assert_eq!(gallery.selected(), Some(second.id));
        gallery.components_dock.apply(Action::Close { panel: PanelId::Component(component) }, PanelId::permissions).unwrap();
        run_frames(&ctx, &mut gallery, 2);
        assert!(gallery.open_tabs().is_empty());
        assert_eq!(gallery.front_tab(), PanelId::View);
        assert!(gallery.tab_story.is_empty());
        assert!(gallery.components_dock.hidden.iter().all(|(panel, _)| !matches!(panel, PanelId::Component(_))), "a closed tab does not come back");
    }

    #[test]
    fn open_tabs_and_the_selections_survive_a_save_and_load() {
        let ctx = egui::Context::default();
        let mut gallery = Gallery::new(None);
        let (component, second) = component_with_stories(&gallery);
        let other = gallery.stories.iter().find(|s| s.component != component).unwrap().component;
        gallery.open_component(component);
        run_frames(&ctx, &mut gallery, 2);
        gallery.select(second.id);
        gallery.open_component(other);
        run_frames(&ctx, &mut gallery, 2);
        let mut storage = Memory::default();
        eframe::App::save(&mut gallery, &mut storage);
        let restored = Gallery::new(Some(&storage));
        assert_eq!(restored.open_tabs(), gallery.open_tabs());
        assert_eq!(restored.front_tab(), PanelId::Component(other));
        assert_eq!(restored.overview_selected(), Some(component));
        assert_eq!(restored.overview_story, gallery.overview_story);
        assert_eq!(restored.tab_story, gallery.tab_story);
        assert_eq!(restored.tab_story.get(component), Some(&second.id));
    }

    #[test]
    fn a_component_the_registry_lacks_is_dropped_from_the_saved_layout_and_selections() {
        use craft_ui::docking::{Layout, Node};
        let centre = Node::Tabs { panels: vec![PanelId::View, PanelId::Component("No such component")], active: 1 };
        let mut storage = Memory::default();
        eframe::set_value(&mut storage, persist::LAYOUT_KEY, &Workspace::new(Layout { root: Some(centre), floating: Vec::new() }));
        let saved = Saved {
            overview_selected: Some("No such component".to_owned()),
            tab_story: HashMap::from([("No such component".to_owned(), "nope/nope/nope".to_owned())]),
            ..Saved::default()
        };
        eframe::set_value(&mut storage, KEY, &saved);
        let gallery = Gallery::new(Some(&storage));
        assert!(gallery.open_tabs().is_empty());
        assert!(gallery.components_dock.hidden.iter().all(|(panel, _)| panel.is_registered()));
        assert_eq!(gallery.components_dock.validate(), Ok(()));
        assert_eq!(gallery.overview_selected(), None);
        assert!(gallery.tab_story.is_empty());
    }

    #[test]
    fn state_saved_with_a_view_and_a_selected_story_loads() {
        let ctx = egui::Context::default();
        let gallery = Gallery::new(None);
        let (component, second) = component_with_stories(&gallery);
        let mut storage = Memory::default();
        eframe::Storage::set_string(&mut storage, KEY, format!("(selected:Some(\"{}\"),view:Component,filter:\"x\")", second.id));
        let mut gallery = Gallery::new(Some(&storage));
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.filter, "x");
        assert_eq!(gallery.overview_selected(), Some(component));
        assert_eq!(gallery.selected(), Some(second.id));
    }

    #[test]
    fn the_dock_layout_round_trips_through_storage() {
        let mut gallery = Gallery::new(None);
        gallery.activate(PanelId::Tokens);
        gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
        let mut storage = Memory::default();
        eframe::App::save(&mut gallery, &mut storage);
        let restored = Gallery::new(Some(&storage));
        assert_eq!(restored.components_dock, gallery.components_dock);
        assert_ne!(restored.components_dock, crate::layout::default_workspace());
    }

    #[test]
    fn an_invalid_saved_layout_falls_back_to_the_default() {
        use craft_ui::docking::{Layout, Node};
        let broken =
            Workspace::new(Layout { root: Some(Node::Tabs { panels: vec![PanelId::View, PanelId::View], active: 0 }), floating: Vec::new() });
        let mut storage = Memory::default();
        eframe::set_value(&mut storage, persist::LAYOUT_KEY, &broken);
        assert_eq!(Gallery::new(Some(&storage)).components_dock, crate::layout::default_workspace());
    }

    #[test]
    fn a_layout_saved_before_the_views_frame_loads_as_the_default_and_keeps_the_rest() {
        use craft_ui::docking::{Layout, Node};
        // The earlier layout held the token browser as a tab beside the canvas in the centre.
        let centre = Node::Tabs { panels: vec![PanelId::View, PanelId::Tokens], active: 1 };
        let old = Workspace::new(Layout { root: Some(centre), floating: Vec::new() });
        let mut storage = Memory::default();
        eframe::set_value(&mut storage, KEY, &Saved { filter: "button".to_owned(), ..Saved::default() });
        eframe::set_value(&mut storage, "craft-ui-gallery.layout", &old);
        let gallery = Gallery::new(Some(&storage));
        assert_eq!(gallery.components_dock, crate::layout::default_workspace());
        assert_eq!(gallery.filter, "button");
    }

    #[test]
    fn a_fresh_gallery_opens_in_the_components_workspace() {
        assert_eq!(Gallery::new(None).workspace(), GalleryWorkspace::Components);
    }

    #[test]
    fn switching_workspaces_keeps_each_layout_as_it_was() {
        let ctx = egui::Context::default();
        let mut gallery = Gallery::new(None);
        gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
        let moved = gallery.components_dock.clone();
        gallery.set_workspace(GalleryWorkspace::Theme);
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.theme_dock, crate::layout::default_theme_workspace());
        gallery.set_workspace(GalleryWorkspace::Components);
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.components_dock, moved);
    }

    #[test]
    fn each_layout_saves_under_its_own_key_and_a_broken_one_costs_only_itself() {
        let mut gallery = Gallery::new(None);
        gallery.components_dock.apply(Action::Float { panel: PanelId::Stories, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
        gallery.theme_dock.apply(Action::Float { panel: PanelId::Changes, rect: [40.0, 40.0, 240.0, 200.0] }, PanelId::permissions).unwrap();
        let mut storage = Memory::default();
        eframe::App::save(&mut gallery, &mut storage);
        let restored = Gallery::new(Some(&storage));
        assert_eq!(restored.components_dock, gallery.components_dock);
        assert_eq!(restored.theme_dock, gallery.theme_dock);
        assert_ne!(restored.theme_dock, crate::layout::default_theme_workspace());

        eframe::Storage::set_string(&mut storage, persist::THEME_LAYOUT_KEY, "(version:9,layout:(root:Some(Frobnicate(".to_owned());
        let restored = Gallery::new(Some(&storage));
        assert_eq!(restored.theme_dock, crate::layout::default_theme_workspace());
        assert_eq!(restored.components_dock, gallery.components_dock);
    }

    #[test]
    fn an_unreadable_saved_layout_costs_only_the_layout() {
        let mut storage = Memory::default();
        eframe::set_value(&mut storage, KEY, &Saved { filter: "button".to_owned(), ..Saved::default() });
        eframe::Storage::set_string(&mut storage, persist::LAYOUT_KEY, "(version:9,layout:(root:Some(Frobnicate(".to_owned());
        let gallery = Gallery::new(Some(&storage));
        assert_eq!(gallery.components_dock, crate::layout::default_workspace());
        assert_eq!(gallery.filter, "button", "the rest of the saved state is kept");
    }

    #[test]
    fn state_saved_with_the_removed_compare_fields_loads() {
        let mut storage = Memory::default();
        eframe::Storage::set_string(
            &mut storage,
            KEY,
            "(filter:\"button\",compare:true,compare_themes:[\"craft.pro-light-gray\"],compare_high:true)".to_owned(),
        );
        let gallery = Gallery::new(Some(&storage));
        assert_eq!(gallery.filter, "button");
        assert_eq!(gallery.variants, None);
    }
}
