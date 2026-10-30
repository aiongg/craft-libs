//! The gallery app.
//!
//! `Gallery` holds the parts and the public API the tests and the capture tool use. Each part
//! owns the state it reads and writes:
//!
//! - [`applied`]: what the context has been set to for the appearance.
//! - [`picks`]: the selections of the Overview and of the component tabs.
//! - [`components`], [`stories`], [`properties`]: the side panels (the filter, the story args).
//! - [`options_bar`]: the bar at the top.
//! - [`variants`]: the Variants view and where it is open.
//! - [`dock`]: the panels in their dock, and the glue that gives each panel its parts.
//! - [`tabs`], [`tiles`], [`canvas`]: the centre tabs and how a story is drawn.
//! - [`catalog`]: the stories and their order.
mod applied;
mod canvas;
mod catalog;
mod components;
mod dock;
mod options_bar;
mod pick;
mod picks;
mod properties;
mod stories;
mod story_state;
mod tabs;
mod tiles;
mod variants;

#[cfg(test)]
mod pick_tests;
#[cfg(test)]
mod reset_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod used_by_tests;

use std::sync::Arc;

use craft_ui::docking::{Action, PanelState, Workspace};
use craft_ui::inspect;
use craft_ui::story::{Args, Story};
use craft_ui::theme::{Tokens, TokensOf};
use egui::{CentralPanel, Frame, Panel, Ui};

use self::applied::Applied;
use self::components::ComponentList;
use self::dock::load_dock;
use self::options_bar::{Bar, BarInput, BarOutput};
use self::pick::Picker;
use self::picks::Picks;
use self::properties::ArgStore;
use self::variants::Variants;
use crate::appearance::Appearance;
use crate::browser::Browser;
use crate::inspector::Inspector;
use crate::layout::{GalleryWorkspace, PanelId, PreviewView, VariantAxis, default_for};
use crate::persist::{self, KEY, Saved};
use crate::preview::AppPreview;
use crate::problems::Problems;
use crate::usage::Scan;

pub(crate) use self::canvas::bare;
#[cfg(any(feature = "capture", test))]
pub(crate) use self::canvas::canvas;
pub(crate) use self::catalog::{default_story_of, stories};
#[cfg(feature = "capture")]
pub(crate) use self::{
    canvas::canvas_frame,
    catalog::nav,
    properties::args_of,
    story_state::StoryStates,
    tiles::{Tile, Tiles, tiles_view},
};

/// The gallery's state.
pub struct Gallery {
    stories: Vec<Story>,
    appearance: Appearance,
    applied: Applied,
    picks: Picks,
    components: ComponentList,
    variants: Variants,
    bar: Bar,
    args: ArgStore,
    browser: Browser,
    problems: Problems,
    /// The mock app the Theme workspace previews.
    app: AppPreview,
    inspector: Inspector,
    /// The scan behind the token detail's Used by list, and the probes of the highlighted tiles.
    scan: Scan,
    /// The keys of the tiles that outlined the selected token's shapes in the last frame.
    highlighted: Vec<&'static str>,
    /// Pick from preview: whether it is on, and what the preview offers to pick.
    pick: Picker,
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
}

impl Gallery {
    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        let saved = Saved::load(storage);
        let stories = stories();
        let components_dock = load_dock(persist::load_layout(storage), GalleryWorkspace::Components);
        let picks = Picks::load(&saved, &stories, &components_dock.layout);
        Self {
            stories,
            applied: Applied::new(&saved.appearance),
            appearance: saved.appearance,
            picks,
            components: ComponentList::new(saved.filter),
            variants: Variants::closed(),
            bar: Bar::default(),
            browser: Browser::new(saved.tokens_split),
            problems: Problems::default(),
            app: AppPreview::default(),
            inspector: Inspector::default(),
            scan: Scan::default(),
            highlighted: Vec::new(),
            pick: Picker::default(),
            workspace: saved.workspace,
            preview: saved.preview,
            components_dock,
            theme_dock: load_dock(persist::load_theme_layout(storage), GalleryWorkspace::Theme),
            reset_layout: false,
            args: ArgStore::new(saved.args),
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
        let tab_open = self.components_dock.layout.contains(&PanelId::Component(story.component));
        self.picks.select(story, tab_open);
    }

    /// The component the Overview has selected.
    pub fn overview_selected(&self) -> Option<&'static str> {
        self.picks.overview_selected
    }

    /// The id of the story the side panels follow: the selected tile's in the Overview, the
    /// selected story's in a component tab.
    pub fn selected(&self) -> Option<&'static str> {
        self.target().map(|s| s.id)
    }

    /// The centre tab that was last brought to the front: the Overview, or a component's tab. The
    /// side panels follow it, also while a floated centre tab is in front beside another.
    pub fn front_tab(&self) -> PanelId {
        self.picks.front_tab(&self.components_dock.layout)
    }

    /// The args of the story with id `id`, which the Properties panel edits.
    pub fn args_of_story(&mut self, id: &str) -> Option<&mut Args> {
        let story = self.stories.iter().find(|s| s.id == id).copied()?;
        Some(self.args.get(&story))
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
        self.variants.close();
        let panel = PanelId::Component(component);
        let _ = self.components_dock.set_visible(panel, true, Some(PanelId::View), PanelId::permissions);
        self.picks.centre = panel;
    }

    /// Selects `component` in the Overview, which comes to the front and scrolls to its tile.
    pub fn select_component(&mut self, component: &'static str) {
        self.picks.select_component(&self.stories, component);
        self.variants.close();
        self.activate(PanelId::View);
    }

    /// Shows the Variants view over `axis` in the front centre tab, or closes it. Without a
    /// selected component there is nothing to repeat, so it stays closed.
    pub fn set_variants(&mut self, axis: Option<VariantAxis>) {
        self.variants.set(axis.filter(|_| self.target().is_some()), self.front_tab());
    }

    pub fn variants(&self) -> Option<VariantAxis> {
        self.variants.axis()
    }

    /// Opens or closes the options bar's More popover.
    pub fn set_more_open(&mut self, open: bool) {
        self.bar.set_more_open(open);
    }

    /// Shows the inspector tab, which turns inspect mode on, instead of the properties tab.
    pub fn set_inspect(&mut self, inspect: bool) {
        self.activate(if inspect { PanelId::Inspect } else { PanelId::Properties });
    }

    /// Brings `panel` to the front of its tab group.
    pub fn activate(&mut self, panel: PanelId) {
        if matches!(panel, PanelId::View | PanelId::Component(_)) {
            self.picks.centre = panel;
        }
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

    /// Whether Pick from preview is on.
    pub fn picking(&self) -> bool {
        self.pick.on
    }

    /// Turns Pick from preview on or off.
    pub fn set_picking(&mut self, on: bool) {
        self.pick.set(on);
    }

    /// The token or field painted under the pointer in the preview, in the last frame; `None` when
    /// nothing is, and the inner `None` for a colour that no token painted.
    pub fn picked_hover(&self) -> Option<Option<craft_theme::TokenRef>> {
        self.pick.hovered.map(|h| h.token)
    }

    /// The keys of the tiles that outlined the shapes painted with the selected token in the last
    /// frame.
    pub fn highlighted_tiles(&self) -> &[&'static str] {
        &self.highlighted
    }

    /// How many components the Used by scan has probed, and how many there are; `None` before a
    /// token is selected.
    pub fn scan_progress(&self) -> Option<(usize, usize)> {
        self.scan.progress()
    }

    /// The components that painted with `token` in the scan so far, with their shape counts.
    pub fn painters(&self, token: craft_theme::TokenRef) -> Vec<(&'static str, usize)> {
        self.scan.painters(token)
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
        self.applied.apply_story_scale(ui.ctx(), &self.appearance);
    }

    fn show_panels(&mut self, ui: &mut Ui) {
        self.applied.apply(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        self.scan.begin_frame();
        self.highlighted.clear();
        if self.browser.selected().is_some() {
            let mode = self.appearance.mode(applied::shown_mode(&ctx));
            self.scan.ensure(&self.stories, &self.appearance, mode);
        }
        let tokens = Tokens::of(ui);
        // Esc closes Variants unless it closes a popup first; the bar draws before the centre, so
        // the popups are read now.
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !egui::Popup::is_any_open(&ctx) && !craft_ui::popover::any_open(&ctx);
        let input = BarInput { has_target: self.target().is_some(), variants_shown: self.variants.axis().is_some() };
        let front = self.front_tab();
        let mut bar = BarOutput::default();
        Panel::top("gallery.bar").frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| {
            let mut picking = self.pick.on;
            bar = self.bar.show(ui, &mut self.appearance, &mut self.workspace, &mut self.preview, &mut picking, &input);
            self.pick.set(picking);
        });
        // Picking belongs to the Theme workspace's preview; Escape ends it.
        self.pick.tiles.clear();
        if self.workspace != GalleryWorkspace::Theme || escape {
            self.pick.set(false);
        }
        if let Some(axis) = bar.variants {
            self.variants.set(Some(axis), front);
        }
        self.reset_layout |= bar.reset_layout;
        if (escape && self.workspace == GalleryWorkspace::Components) || self.front_tab() != self.variants.tab() {
            self.variants.close();
        }
        let inspecting = self.workspace == GalleryWorkspace::Components
            && self.variants.axis().is_none()
            && self.target().is_some()
            && self.components_dock.layout.panel_state(&PanelId::Inspect) == PanelState::Front;
        inspect::set_enabled(&ctx, inspecting);
        if inspecting {
            self.inspector.begin(&ctx, Arc::clone(&tokens));
        }
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| self.dock(ui, inspecting));
        let mode = self.appearance.mode(applied::shown_mode(&ctx));
        self.scan.step(&ctx, self.applied.base.inputs.theme(mode));
        if std::mem::take(&mut self.reset_layout) {
            let shown = self.workspace;
            // Reset layout puts the panels back; the open component tabs are documents and stay.
            let tabs = self.open_tabs();
            *self.dock_of_mut(shown) = default_for(shown);
            if shown == GalleryWorkspace::Components {
                for component in tabs {
                    let _ = self.components_dock.set_visible(PanelId::Component(component), true, Some(PanelId::View), PanelId::permissions);
                }
                self.activate(PanelId::View);
            }
        }
        self.forget_closed_tabs();
    }

    /// Drops what a component tab that is no longer in the layout kept: its selected story, and
    /// the place the dock recorded for it. A closed tab does not come back from `hidden`.
    fn forget_closed_tabs(&mut self) {
        self.picks.forget_closed_tabs(&self.components_dock.layout);
        self.components_dock.hidden.retain(|(panel, _)| !matches!(panel, PanelId::Component(_)));
    }

    /// Puts the args of the story with id `id` back to its defaults.
    fn reset_story(&mut self, id: &str) {
        if let Some(story) = self.stories.iter().find(|s| s.id == id).copied() {
            self.args.reset(&story);
        }
    }

    fn reset_args(&mut self) {
        if let Some(story) = self.target() {
            self.reset_story(story.id);
        }
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        crate::preview::AppPreview::raw_input_hook(ctx, raw);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let picks = &self.picks;
        let saved = Saved {
            appearance: self.appearance.clone(),
            selected: None,
            overview_selected: picks.overview_selected.map(str::to_owned),
            overview_level: picks.overview_level.map(|l| l.slug().to_owned()),
            overview_story: picks.overview_story.iter().map(|(c, id)| ((*c).to_owned(), (*id).to_owned())).collect(),
            tab_story: picks.tab_story.iter().map(|(c, id)| ((*c).to_owned(), (*id).to_owned())).collect(),
            filter: self.components.filter.clone(),
            workspace: self.workspace,
            preview: self.preview,
            tokens_split: Some(self.browser.share()),
            args: self.args.to_saved(),
        };
        eframe::set_value(storage, KEY, &saved);
        eframe::set_value(storage, persist::LAYOUT_KEY, &self.components_dock);
        eframe::set_value(storage, persist::THEME_LAYOUT_KEY, &self.theme_dock);
    }
}
