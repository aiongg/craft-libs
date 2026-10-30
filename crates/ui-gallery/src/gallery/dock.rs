//! The dock: the shown workspace's panels in their dock area, and what each panel draws.
use craft_theme::ControlSize;
use craft_ui::composites::EmptyState;
use craft_ui::controls::IconButton;
use craft_ui::direction;
use craft_ui::docking::{Action, DockArea, DockConfig, DockContent, DockHeights, Permissions, Workspace};
use craft_ui::icons::Icon;
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::story::Story;
use craft_ui::theme::{Tokens, TokensOf, margin};
use egui::{Frame, ScrollArea, Ui};

use super::applied::shown_mode;
use super::tabs::{self, TabEnv};
use super::tiles::TileEvents;
use super::{Gallery, stories, variants};
use crate::browser;
use crate::layout::{GalleryWorkspace, PanelId, PreviewView, VariantAxis, default_for, reconciled};
use crate::usage::Highlight;

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

impl Gallery {
    /// The shown workspace's panels in their dock. The panels draw from a copy of the layout,
    /// because they borrow the gallery and may change its dock (opening a tab); the dock's own
    /// actions of this frame are applied to the gallery's layout afterwards.
    pub(super) fn dock(&mut self, ui: &mut Ui, inspecting: bool) {
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
        // Component tabs are documents: they close from their ×.
        config.tab_close_buttons = true;
        let mut panels = Panels { gallery: self, inspecting, control: tokens.size.control.sm };
        let output = DockArea::new(ui.make_persistent_id(id)).locked(workspace.locked).show_content(ui, &workspace.layout, &config, &mut panels);
        for action in output.actions {
            if let Action::Activate { panel } = &action
                && matches!(panel, PanelId::View | PanelId::Component(_))
            {
                self.picks.centre = *panel;
            }
            let _ = self.dock_of_mut(shown).apply(action, PanelId::permissions);
        }
        // Closing the tab in front shows the Overview.
        if shown == GalleryWorkspace::Components && !self.components_dock.layout.contains(&front) {
            self.activate(PanelId::View);
        }
    }

    /// The story the side panels follow: the Overview's selected tile's, or the front component
    /// tab's selected story. None while the Overview has no selection.
    pub(super) fn target(&self) -> Option<Story> {
        self.picks.target(&self.stories, self.front_tab())
    }

    /// The components list; a click selects a component, Open, Enter and a double click open its
    /// tab.
    fn components_panel(&mut self, ui: &mut Ui) {
        let current = self.target().map(|s| s.component);
        let events = self.components.show(ui, &self.stories, current);
        if let Some(component) = events.picked {
            self.select_component(component);
        }
        if let Some(component) = events.opened {
            self.open_component(component);
        }
    }

    /// The target component's stories. In the Overview a pick changes what the selected tile
    /// shows; in a component tab it selects that story's tile.
    fn stories_panel(&mut self, ui: &mut Ui) {
        let Some(target) = self.target() else {
            select_a_component(ui);
            return;
        };
        if let Some(id) = stories::show(ui, &self.stories, &target) {
            self.picks.pick_story(self.front_tab(), target.component, id);
        }
    }

    /// The target story's args, under a header with its name and Reset.
    fn properties_panel(&mut self, ui: &mut Ui) {
        let Some(story) = self.target() else {
            select_a_component(ui);
            return;
        };
        self.args.show(ui, &story);
    }

    /// The Variants view of the target story, in the front centre tab.
    fn variants_view(&mut self, ui: &mut Ui, axis: VariantAxis) {
        let Some(story) = self.target() else {
            self.variants.close();
            return;
        };
        let args = self.args.get(&story);
        if variants::show(ui, &story, axis, args, &mut self.appearance, self.applied.os_applied) {
            self.variants.close();
        }
    }

    /// What the tabs that draw tiles need.
    fn tab_env(&mut self, ui: &Ui, inspecting: bool) -> TabEnv<'_> {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let highlight =
            self.browser.selected().map(|token| Highlight { scan: &mut self.scan, token, theme: self.applied.base.inputs.theme(mode), mode });
        TabEnv {
            highlight,
            stories: &self.stories,
            filter: &self.components.filter,
            appearance: &self.appearance,
            system: self.applied.os_applied,
            args: &mut self.args,
            inspector: &mut self.inspector,
            picks: &mut self.picks,
            inspecting,
        }
    }

    /// Applies what a set of tiles reported, other than a pick, which the tab interprets.
    fn apply_tile_events(&mut self, events: &TileEvents) {
        if let Some(id) = events.reset {
            self.reset_story(id);
        }
        if let Some(component) = events.opened {
            self.open_component(component);
        }
        self.highlighted.extend(&events.highlighted);
    }

    /// The Overview tab, or the Variants view when it shows there.
    fn overview_tab(&mut self, ui: &mut Ui, inspecting: bool) {
        if let Some(axis) = self.variants.in_tab(PanelId::View) {
            self.variants_view(ui, axis);
            return;
        }
        let events = tabs::overview(ui, &mut self.tab_env(ui, inspecting));
        self.apply_tile_events(&events);
    }

    /// A component's tab, or the Variants view when it shows there.
    fn component_tab(&mut self, ui: &mut Ui, component: &'static str, inspecting: bool) {
        if let Some(axis) = self.variants.in_tab(PanelId::Component(component)) {
            self.variants_view(ui, axis);
            return;
        }
        let events = tabs::component_tab(ui, &mut self.tab_env(ui, inspecting), component);
        self.apply_tile_events(&events);
    }

    /// The Theme workspace's centre: the mock app, or the Controls level's tiles.
    fn preview_panel(&mut self, ui: &mut Ui) {
        match self.preview {
            PreviewView::App => self.app.show(ui),
            PreviewView::Components => {
                let events = tabs::preview_components(ui, &mut self.tab_env(ui, false));
                self.apply_tile_events(&events);
            }
        }
    }

    fn tokens_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let opened = self.browser.show(ui, &self.applied.base, mode, &self.scan);
        self.open_requested(opened);
    }

    /// Opens the component a Used by link named.
    fn open_requested(&mut self, component: Option<&'static str>) {
        if let Some(component) = component {
            self.open_component(component);
        }
    }

    /// The Theme workspace's Theme panel: the theme's basics above the token browser, which is
    /// the one the Tokens tab draws.
    fn theme_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let theme = self.applied.base.inputs.theme(mode).clone();
        let max = (ui.available_height() * BASICS_SHARE).round();
        ScrollArea::vertical().id_salt("gallery.theme.basics.scroll").max_height(max).auto_shrink([false, true]).show(ui, |ui| {
            browser::basics(ui, &theme, mode);
        });
        let opened = self.browser.show(ui, &self.applied.base, mode, &self.scan);
        self.open_requested(opened);
    }

    /// The Problems panel; a click on a row selects its foreground token in the Theme panel.
    fn problems_panel(&mut self, ui: &mut Ui) {
        let mode = self.appearance.mode(shown_mode(ui.ctx()));
        let explained = self.browser.explained(&self.applied.base, mode);
        if let Some(token) = self.problems.show(ui, &self.applied.base, mode, &explained) {
            self.browser.select(token);
            self.activate(PanelId::Theme);
        }
    }
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
pub(super) fn load_dock(saved: Option<Workspace<PanelId>>, workspace: GalleryWorkspace) -> Workspace<PanelId> {
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
            PanelId::Inspect if self.gallery.variants.axis().is_some() => placeholder(ui, "Inspect is off while Variants shows."),
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
