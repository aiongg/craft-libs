//! The gallery app.
use std::collections::HashMap;
use std::sync::Arc;

use craft_platform::appearance::SystemReader;
use craft_theme::{ButtonVariant, ContrastChoice, ControlSize, Mode, ModeChoice, MotionChoice, SystemAppearance, TextSize, TypeRole, builtins};
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::composites::{EmptyState, FormGrid, PropertyRow};
use craft_ui::controls::{Button, Checkbox, IconButton, Link, NumberField, SearchField, Segmented, Select, TextField, ToggleButton, TooltipExt as _};
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
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install, margin, scope};
use egui::epaint::RectShape;
use egui::{
    Align, CentralPanel, Color32, Frame, Margin, Panel, Rect, ScrollArea, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Widget as _, pos2,
};

use crate::appearance::{Appearance, SCALE, builtin};
use crate::browser::{self, Browser};
use crate::faces;
use crate::fields;
use crate::inspector::Inspector;
use crate::layout::{CentreView, GalleryWorkspace, PanelId, PreviewView, VariantAxis, default_for, reconciled};
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
    selected: Option<String>,
    filter: String,
    view: CentreView,
    /// The axis the Variants view repeats the story over; not saved.
    variants: Option<VariantAxis>,
    /// The More popover of the options bar is open.
    more_open: bool,
    /// The component the Overview scrolls to, until its tile has been drawn.
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
        let selected = saved.selected.filter(|id| stories.iter().any(|s| s.id == id)).or_else(|| first_story(&stories).map(str::to_owned));
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
            view: saved.view,
            variants: None,
            more_open: false,
            scroll_to: None,
            browser: Browser::new(saved.tokens_split),
            problems: Problems::default(),
            app: AppPreview::default(),
            inspector: Inspector::default(),
            workspace: saved.workspace,
            preview: saved.preview,
            components_dock: load_dock(persist::load_layout(storage), GalleryWorkspace::Components),
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

    /// Selects a story by id; unknown ids are ignored.
    pub fn select(&mut self, id: &str) {
        if self.stories.iter().any(|s| s.id == id) {
            self.selected = Some(id.to_owned());
        }
    }

    pub fn set_view(&mut self, view: CentreView) {
        self.view = view;
    }

    pub fn view(&self) -> CentreView {
        self.view
    }

    /// The id of the selected story.
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Shows the Variants view over `axis` in the Component view, or closes it.
    pub fn set_variants(&mut self, axis: Option<VariantAxis>) {
        self.variants = axis;
        if axis.is_some() {
            self.view = CentreView::Component;
        }
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
        if (escape && self.workspace == GalleryWorkspace::Components) || self.view == CentreView::Overview {
            self.variants = None;
        }
        let layout = &self.components_dock.layout;
        let inspecting = self.workspace == GalleryWorkspace::Components
            && self.view == CentreView::Component
            && self.variants.is_none()
            && layout.panel_state(&PanelId::Inspect) == PanelState::Front
            && layout.panel_state(&PanelId::View) == PanelState::Front;
        inspect::set_enabled(&ctx, inspecting);
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
        }
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| self.dock(ui, inspecting));
        if std::mem::take(&mut self.reset_layout) {
            let shown = self.workspace;
            *self.dock_of_mut(shown) = default_for(shown);
        }
    }

    /// The shown workspace's panels in their dock. The dock layout is taken out while the panels
    /// draw, because they borrow the gallery.
    fn dock(&mut self, ui: &mut Ui, inspecting: bool) {
        let shown = self.workspace;
        let mut workspace = std::mem::take(self.dock_of_mut(shown));
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
            let _ = workspace.apply(action, PanelId::permissions);
        }
        *self.dock_of_mut(shown) = workspace;
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
        let view = &mut self.view;
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
                        segmented(ui, "View", view, [("Overview", CentreView::Overview), ("Component", CentreView::Component)]);
                        // The toggle only shows the state; the menu opens on the press.
                        let mut shown = variants.is_some();
                        let button = ui.add(ToggleButton::new("Variants", &mut shown).trailing_icon(Icon::ChevronDown).size(ControlSize::Sm));
                        let entries: Vec<MenuEntry<VariantAxis>> =
                            VariantAxis::ALL.into_iter().map(|axis| MenuItem::new(axis.menu_label(), axis).into()).collect();
                        if let Some(axis) = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen {
                            *variants = Some(axis);
                            *view = CentreView::Component;
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
        let mut activated = false;
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(Entry::Component(c)), .. }) => picked = Some(c),
                ListEvent::Activate(Entry::Component(c)) => {
                    picked = Some(c);
                    activated = true;
                }
                _ => {}
            }
        }
        // Enter on the current component scrolls the Overview back to it.
        if let Some(component) = picked.filter(|c| activated || Some(*c) != current) {
            self.select_component(component);
        }
    }

    /// Opens the default story of `component`; the Overview scrolls to its tile.
    fn select_component(&mut self, component: &'static str) {
        if let Some(story) = default_story_of(&self.stories, component) {
            self.selected = Some(story.id.to_owned());
            self.scroll_to = Some(component);
        }
    }

    /// Opens `component` in the Component view, on its default story.
    fn open_component(&mut self, component: &'static str) {
        self.select_component(component);
        self.scroll_to = None;
        self.view = CentreView::Component;
        self.workspace = GalleryWorkspace::Components;
        self.variants = None;
        self.activate(PanelId::View);
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

    /// The selected story's args.
    fn properties_panel(&mut self, ui: &mut Ui) {
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

    /// The centre: the current view.
    fn view_panel(&mut self, ui: &mut Ui, inspecting: bool) {
        match self.view {
            CentreView::Overview => self.overview_view(ui),
            CentreView::Component => self.component_view(ui, inspecting),
        }
    }

    /// The story, or its variants while Variants is shown, each on its surface in the dock colour.
    fn component_view(&mut self, ui: &mut Ui, inspecting: bool) {
        let Some(story) = self.current() else {
            canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui.label("No story selected.");
            });
            return;
        };
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let args = args_of(&mut self.args, &self.saved_args, &story);
        let inspector = inspecting.then_some(&mut self.inspector);
        let mut applied = None;
        let mut closed = false;
        canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            if let Some(axis) = self.variants {
                closed = variants_header(ui, &story, axis);
                ScrollArea::vertical().id_salt("gallery.variants").auto_shrink([false, false]).show(ui, |ui| {
                    applied = variants_grid(ui, &story, args, &appearance, system, axis);
                });
            } else {
                ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| canvas(ui, &story, args, &appearance, system, inspector));
            }
        });
        if let Some(from) = applied {
            if let Some(axis) = self.variants {
                copy_axis(axis, &from, &mut self.appearance);
            }
            closed = true;
        }
        if closed {
            self.variants = None;
        }
    }

    /// Every component of one level, each tile on its story's surface: the level follows the selected
    /// story. Each tile draws the component's default story; its name opens the Component view.
    fn overview_view(&mut self, ui: &mut Ui) {
        let nav = nav(&self.stories, &self.filter.to_lowercase());
        let current = self.current();
        let shown = current.and_then(|s| nav.iter().position(|(level, _)| *level == s.level)).unwrap_or(0);
        let appearance = self.appearance.clone();
        let system = self.os_applied;
        let mut level = shown;
        let mut opened = None;
        canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let Some((shown_level, components)) = nav.get(shown) else {
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
            let tiles = overview_tiles(&self.stories, components, current);
            let scroll_to = &mut self.scroll_to;
            ScrollArea::vertical().id_salt(shown_level.slug()).auto_shrink([false, false]).show(ui, |ui| {
                opened = overview(ui, &tiles, &mut self.args, &self.saved_args, &appearance, system, scroll_to);
            });
        });
        let first = nav.get(level).and_then(|(_, components)| components.first()).and_then(|c| default_story_of(&self.stories, c));
        if let Some(story) = first.filter(|_| level != shown) {
            self.selected = Some(story.id.to_owned());
        }
        if let Some(component) = opened {
            self.open_component(component);
        }
    }

    /// The Theme workspace's centre: the mock app, or the Controls level's tiles.
    fn preview_panel(&mut self, ui: &mut Ui) {
        match self.preview {
            PreviewView::App => self.app.show(ui),
            PreviewView::Components => {
                let components: Vec<&'static str> =
                    nav(&self.stories, "").into_iter().find(|(level, _)| *level == Level::Controls).map_or_else(Vec::new, |(_, c)| c);
                let tiles = overview_tiles(&self.stories, &components, None);
                let appearance = self.appearance.clone();
                let system = self.os_applied;
                let mut opened = None;
                canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    let mut scroll_to = None;
                    ScrollArea::vertical().id_salt("gallery.preview").auto_shrink([false, false]).show(ui, |ui| {
                        opened = overview(ui, &tiles, &mut self.args, &self.saved_args, &appearance, system, &mut scroll_to);
                    });
                });
                if let Some(component) = opened {
                    self.open_component(component);
                }
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
            PanelId::Inspect if self.gallery.view == CentreView::Overview || self.gallery.variants.is_some() => padded(ui, |ui| {
                ui.label("Inspect works on the Component view.");
            }),
            PanelId::Inspect => padded(ui, |ui| self.gallery.inspector.panel(ui)),
            PanelId::View => self.gallery.view_panel(ui, self.inspecting),
            PanelId::Tokens => padded(ui, |ui| self.gallery.tokens_panel(ui)),
            PanelId::Preview => self.gallery.preview_panel(ui),
            PanelId::Theme => padded(ui, |ui| self.gallery.theme_panel(ui)),
            PanelId::Problems => padded(ui, |ui| self.gallery.problems_panel(ui)),
            PanelId::Changes => placeholder(ui, "Changes since the last save come here."),
        }
    }

    fn logical_label(&self, panel: &PanelId) -> String {
        match panel {
            PanelId::View => self.gallery.view.title(),
            _ => panel.title(),
        }
        .to_owned()
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
            selected: self.selected.clone(),
            filter: self.filter.clone(),
            view: self.view,
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

/// The story the navigation lists first: the first story of its first level's first component.
fn first_story(stories: &[Story]) -> Option<&'static str> {
    let (level, components) = nav(stories, "").into_iter().next()?;
    let component = *components.first()?;
    stories.iter().find(|s| s.level == level && s.component == component).map(|s| s.id)
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

/// The story each of `components` draws in the Overview: its default story, except the selected
/// component's, which draws the selected story so Stories and Properties change what is seen.
fn overview_tiles(stories: &[Story], components: &[&'static str], selected: Option<Story>) -> Vec<Story> {
    components.iter().filter_map(|c| selected.filter(|s| s.component == *c).or_else(|| default_story_of(stories, c).copied())).collect()
}

/// The narrowest a tile may be: PhotoCraft's dock panels are 290 pt wide, and a story drawn in
/// less than a panel's width is no longer what the apps show.
const TILE_MIN_WIDTH: f32 = 290.0;

/// The tiles of one level in one or two columns: two once each column keeps [`TILE_MIN_WIDTH`].
/// A row's tiles share its height. Returns the component whose name was clicked.
pub(crate) fn overview(
    ui: &mut Ui,
    tiles: &[Story],
    args: &mut HashMap<String, Args>,
    saved: &HashMap<String, Vec<persist::SavedArg>>,
    appearance: &Appearance,
    system: SystemAppearance,
    scroll_to: &mut Option<&'static str>,
) -> Option<&'static str> {
    let shown = shown_mode(ui.ctx());
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        appearance.overrides(o);
        o.mode = Some(appearance.mode(shown));
        o.contrast = Some(appearance.contrast(system));
    };
    let mut opened = None;
    scope(ui, overrides, |ui| {
        let tokens = Tokens::of(ui);
        let gap = tokens.space.gap.section;
        let rtl = direction::is_rtl(ui);
        let width = ui.available_width();
        let columns = if width >= 2.0 * TILE_MIN_WIDTH + gap { 2 } else { 1 };
        let tile_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let area = ui.available_rect_before_wrap();
        // A tile is as tall as its story; a story that fills the height it is given gets a viewport's.
        let reach = area.height();
        for row in tiles.chunks(columns) {
            let top = ui.cursor().min.y;
            let mut bottom = top;
            let mut placed = Vec::new();
            for (i, story) in row.iter().enumerate() {
                let x = if rtl { area.max.x - (i + 1) as f32 * tile_width - i as f32 * gap } else { area.min.x + i as f32 * (tile_width + gap) };
                let slot = ui.painter().add(Shape::Noop);
                let mut cell =
                    ui.new_child(UiBuilder::new().id_salt(story.id).max_rect(Rect::from_min_max(pos2(x, top), pos2(x + tile_width, top + reach))));
                let surface = StoryFrame::of(&cell, story.surface);
                let used = Frame::new().inner_margin(surface.margin).show(&mut cell, |ui| {
                    surface.space(ui);
                    ui.set_min_width(ui.available_width());
                    if Link::new(story.component).role(TypeRole::Heading).id_salt(("tile", story.id)).ui(ui).clicked() {
                        opened = Some(story.component);
                    }
                    let args = args_of(args, saved, story);
                    ui.push_id(story.id, |ui| (story.show)(ui, args));
                    // A tile does not change the gallery's scale.
                    let _ = craft_ui::story::take_pixels_per_point_request(ui.ctx());
                });
                if *scroll_to == Some(story.component) {
                    cell.scroll_to_rect(used.response.rect, Some(Align::TOP));
                    *scroll_to = None;
                }
                bottom = bottom.max(used.response.rect.bottom());
                placed.push((slot, x, surface));
            }
            for (slot, x, surface) in placed {
                if let Some(fill) = surface.fill {
                    let rect = Rect::from_min_max(pos2(x, top), pos2(x + tile_width, bottom));
                    ui.painter().set(slot, Shape::Rect(RectShape::new(rect, 0.0, fill, surface.edge, StrokeKind::Inside)));
                }
            }
            ui.allocate_rect(Rect::from_min_max(pos2(area.min.x, top), pos2(area.max.x, bottom)), Sense::hover());
            ui.add_space(gap);
        }
    });
    opened
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
    fn a_fresh_gallery_opens_in_the_overview() {
        assert_eq!(Gallery::new(None).view, CentreView::Overview);
    }

    /// Frames of the gallery in a 1280 x 800 window.
    fn run_frames(ctx: &egui::Context, gallery: &mut Gallery, frames: usize) {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
        for _ in 0..frames {
            ctx.run_ui(input.clone(), |ui| gallery.show(ui)).drop_without_applying_deltas();
        }
    }

    #[test]
    fn picking_a_component_in_the_overview_scrolls_to_it_and_keeps_the_overview() {
        let ctx = egui::Context::default();
        let mut gallery = Gallery::new(None);
        let current = gallery.current().unwrap();
        // The last component of another level, so the pick changes the level shown.
        let story = gallery.stories.iter().rev().find(|s| s.level != current.level && s.level != Level::Specimens).copied().unwrap();
        gallery.select_component(story.component);
        assert_eq!(gallery.scroll_to, Some(story.component));
        assert_eq!(gallery.view, CentreView::Overview);
        run_frames(&ctx, &mut gallery, 2);
        assert_eq!(gallery.scroll_to, None, "the tile was drawn and scrolled to");
        assert_eq!(gallery.view, CentreView::Overview);
        assert_eq!(gallery.current().unwrap().component, story.component);
    }

    #[test]
    fn the_selected_components_tile_draws_the_selected_story() {
        let stories = stories();
        let multi = stories.iter().find(|s| stories.iter().filter(|o| o.component == s.component).count() > 1).unwrap().component;
        let other = stories.iter().find(|s| s.component != multi && s.level == stories.iter().find(|m| m.component == multi).unwrap().level).unwrap();
        let second = *stories.iter().filter(|s| s.component == multi).nth(1).unwrap();
        let components = [multi, other.component];
        let tiles = overview_tiles(&stories, &components, Some(second));
        assert_eq!(tiles[0].id, second.id);
        assert_eq!(tiles[1].id, default_story_of(&stories, other.component).unwrap().id);
        assert_eq!(overview_tiles(&stories, &components, None)[0].id, default_story_of(&stories, multi).unwrap().id);
    }

    #[test]
    fn a_tiles_link_opens_the_component_view_on_the_default_story() {
        let mut gallery = Gallery::new(None);
        let story = gallery.stories.iter().rev().find(|s| s.component != gallery.current().unwrap().component).unwrap();
        let (component, first) = (story.component, gallery.stories.iter().find(|s| s.component == story.component).unwrap().id);
        gallery.open_component(component);
        assert_eq!(gallery.view, CentreView::Component);
        assert_eq!(gallery.selected.as_deref(), Some(first));
        assert_eq!(gallery.scroll_to, None);
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
