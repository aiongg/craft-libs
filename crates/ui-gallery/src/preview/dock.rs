//! Everything between the tool rail and the end edge: the canvas region with the panel rail at
//! its end, and the dock column.

use craft_ui::docking::{Column, DockArea, DockConfig, DockContent, DockHeights, Group, Layout, Node, PanelState, Permissions, Side, Workspace};
use craft_ui::icons::Icon;
use craft_ui::layout::{SplitAxis, SplitSize};
use craft_ui::patterns::{PanelRail, PanelRailItem, PanelRailRequest};
use egui::{CentralPanel, Frame, Ui};

use super::canvas;
use super::model::Mock;
use super::panels::{Panel, State};

/// The width of the dock column.
pub const WIDTH: f32 = 290.0;

/// The panels in the dock and what they keep between frames.
#[derive(Clone, Debug)]
pub struct DockState {
    workspace: Workspace<Panel>,
    panels: State,
}

impl Default for DockState {
    fn default() -> Self {
        Self { workspace: default_workspace(), panels: State::default() }
    }
}

impl DockState {
    /// Whether the panel is in the dock, docked or floating, and not hidden by the Window menu.
    pub fn is_visible(&self, panel: Panel) -> bool {
        self.workspace.layout.contains(&panel)
    }

    /// Shows a hidden panel, at the place it was hidden from, and hides a shown one.
    pub fn toggle(&mut self, panel: Panel) {
        self.set_visible(panel, !self.is_visible(panel));
    }

    /// Shows the panel and makes it the front tab of its group.
    pub fn front(&mut self, panel: Panel) {
        self.set_visible(panel, true);
    }

    fn set_visible(&mut self, panel: Panel, visible: bool) {
        // A layout that fails validation is the dock's own bug; the dock keeps its last good layout.
        let _ = self.workspace.set_visible(panel, visible, None, Panel::permissions);
    }

    /// Whether the panel is the front tab of its group (or a floating one).
    pub fn is_front(&self, panel: Panel) -> bool {
        self.workspace.layout.panel_state(&panel) == PanelState::Front
    }
}

impl Panel {
    /// Panels move, tab and float but the user cannot close them; the Window menu hides them.
    /// The canvas stays.
    pub fn permissions(&self) -> Permissions {
        if *self == Panel::Canvas { Permissions::PROTECTED } else { Permissions::default() }
    }

    fn icon(self) -> Icon {
        match self {
            Panel::Canvas => Icon::Image,
            Panel::Swatches => Icon::SwatchBook,
            Panel::Properties => Icon::Adjustments,
            Panel::Adjustments => Icon::AdjustmentLayer,
            Panel::Layers => Icon::Layers,
            Panel::Channels => Icon::Blend,
            Panel::History => Icon::Clock,
        }
    }
}

/// The canvas with the panels' rail at its end, and a column at the end edge with [Swatches],
/// [Properties, Adjustments] and [Layers, Channels, History].
fn default_workspace() -> Workspace<Panel> {
    let column = Column::new(
        vec![
            Group::new(vec![Panel::Swatches], 0),
            Group::new(vec![Panel::Properties, Panel::Adjustments], 0),
            Group::new(vec![Panel::Layers, Panel::Channels, Panel::History], 0),
        ],
        WIDTH,
    );
    let root = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(WIDTH),
        first: Box::new(Node::Tabs { panels: vec![Panel::Canvas], active: 0 }),
        second: Box::new(Node::Dock { side: Side::End, columns: vec![column] }),
    };
    Workspace::new(Layout { root: Some(root), floating: Vec::new() })
}

struct Content<'a> {
    mock: &'a mut Mock,
    panels: &'a mut State,
    /// The panels in front, which the rail marks.
    front: Vec<Panel>,
    /// The panel the rail asked for.
    requested: Option<Panel>,
}

impl Content<'_> {
    fn canvas(&mut self, ui: &mut Ui) {
        let items: Vec<PanelRailItem<Panel>> = Panel::ALL.iter().map(|p| PanelRailItem::new(*p, p.icon(), p.label())).collect();
        let id = ui.scope_id().with("panel-rail");
        let out = PanelRail::new(id).show(ui, &items, &self.front, |_| {});
        if let Some(PanelRailRequest::Open(panel) | PanelRailRequest::Close(panel)) = out.request {
            self.requested = Some(panel);
        }
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| canvas::show(ui, self.mock));
    }
}

impl DockContent<Panel> for Content<'_> {
    fn body(&mut self, ui: &mut Ui, panel: &Panel) {
        match panel {
            Panel::Canvas => self.canvas(ui),
            other => other.show(ui, self.mock, self.panels),
        }
    }

    fn logical_label(&self, panel: &Panel) -> String {
        panel.label().to_owned()
    }

    fn permissions(&self, panel: &Panel) -> Permissions {
        panel.permissions()
    }

    fn dock_heights(&self, panel: &Panel) -> Option<DockHeights> {
        match panel {
            Panel::Swatches => Some(DockHeights::new(110.0).compact(90.0)),
            Panel::Properties | Panel::Adjustments => Some(DockHeights::new(200.0).compact(120.0)),
            Panel::Layers | Panel::Channels | Panel::History => Some(DockHeights::new(0.0).fill(300.0)),
            Panel::Canvas => None,
        }
    }
}

/// The dock area over everything between the tool rail and the end edge. Its protected canvas
/// panel draws the panel rail and the canvas region.
pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut DockState) {
    let id = ui.scope_id().with("dock");
    let mut config = DockConfig::default();
    config.right_to_left = craft_ui::direction::is_rtl(ui);
    // Panels never close by the user's hand; the Window menu and the rail hide and show them.
    config.close_menu_items = false;
    config.close_button = false;
    let front: Vec<Panel> = Panel::ALL.into_iter().filter(|p| state.is_front(*p)).collect();
    let DockState { workspace, panels } = state;
    // A floated panel stays inside the part of the preview that shows.
    let bounds = ui.max_rect().intersect(ui.clip_rect());
    let mut content = Content { mock, panels, front, requested: None };
    let mut actions = Vec::new();
    let mut requested = None;
    CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let output = DockArea::new(id).native_windows(false).float_bounds(bounds).locked(workspace.locked).show_content(
            ui,
            &workspace.layout,
            &config,
            &mut content,
        );
        actions = output.actions;
        requested = content.requested;
    });
    for action in actions {
        let _ = workspace.apply(action, Panel::permissions);
    }
    if let Some(panel) = requested {
        let _ = workspace.set_visible(panel, true, None, Panel::permissions);
    }
}
