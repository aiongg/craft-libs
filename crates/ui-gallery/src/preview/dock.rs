//! Everything between the tool rail and the end edge: the canvas region and the dock at the end
//! edge, an iconic column beside an expanded column as in Photoshop's Essentials layout.

use std::cell::RefCell;

use craft_ui::docking::{
    Column, DockArea, DockConfig, DockContent, DockEvent, DockHeights, DockLook, Group, Layout, Node, Permissions, Side, Workspace, close_flyout,
    flyout,
};
use craft_ui::icons::{self, Icon};
use craft_ui::layout::{SplitAxis, SplitSize};
use egui::{CentralPanel, Color32, Frame, Painter, Rect, Ui};

use super::canvas::{self, Chrome};
use super::model::Mock;
use super::panels::{Panel, State};

/// The width of the expanded column.
pub const WIDTH: f32 = 290.0;

/// The panels in the dock and what they keep between frames.
#[derive(Clone, Debug)]
pub struct DockState {
    workspace: Workspace<Panel>,
    panels: State,
    /// Auto-Collapse Iconic Panels: the flyout closes on Escape or a press outside it.
    auto_collapse: bool,
    /// A panel whose flyout opens on the next frame, when it sits in the iconic column.
    raise: Option<Panel>,
    /// The panel whose flyout was open when the dock was last drawn.
    shown_flyout: Option<Panel>,
    /// An iconic panel the Window menu chose while it was shown: its flyout closes if open, else opens.
    flip: Option<Panel>,
    /// Whether the split has been sized for the iconic column, which needs the dock's look.
    sized: bool,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            workspace: default_workspace(),
            panels: State::default(),
            auto_collapse: true,
            raise: None,
            shown_flyout: None,
            flip: None,
            sized: false,
        }
    }
}

impl DockState {
    /// Opens the flyout that was open when the dock was last drawn, on the next draw. The flyout
    /// lives in egui memory, which a copy drawn in another context does not have.
    pub fn reopen_flyout(&mut self) {
        self.raise = self.shown_flyout.or(self.raise);
    }

    /// Whether the panel is in the dock, docked or floating, and not hidden by the Window menu.
    pub fn is_visible(&self, panel: Panel) -> bool {
        self.workspace.layout.contains(&panel)
    }

    /// Shows a hidden panel, at the place it was hidden from, and hides a shown one. A panel in
    /// the iconic column stays shown: showing it opens its flyout, and choosing it again closes the
    /// flyout when open and opens it when closed.
    pub fn toggle(&mut self, panel: Panel) {
        if !self.is_visible(panel) {
            self.set_visible(panel, true);
            if self.is_iconic(panel) {
                self.raise = Some(panel);
            }
        } else if self.is_iconic(panel) {
            self.flip = Some(panel);
        } else {
            self.set_visible(panel, false);
        }
    }

    /// Whether the panel is the front tab of its group (or a floating one).
    #[cfg(test)]
    pub fn is_front(&self, panel: Panel) -> bool {
        self.workspace.layout.panel_state(&panel) == craft_ui::docking::PanelState::Front
    }

    /// Hides the panel as the Window menu does.
    #[cfg(test)]
    pub fn hide(&mut self, panel: Panel) {
        self.set_visible(panel, false);
    }

    fn set_visible(&mut self, panel: Panel, visible: bool) {
        // A layout that fails validation is the dock's own bug; the dock keeps its last good layout.
        let _ = self.workspace.set_visible(panel, visible, None, Panel::permissions);
    }

    /// Whether the panel sits in an iconic column.
    pub fn is_iconic(&self, panel: Panel) -> bool {
        self.workspace.layout.location(&panel).ok().and_then(|l| l.group).and_then(|g| g.column).is_some_and(|c| c.iconic)
    }
}

impl Panel {
    /// Panels move, tab and float but the user cannot close them; the Window menu hides them.
    /// The canvas stays.
    pub fn permissions(&self) -> Permissions {
        if *self == Panel::Canvas { Permissions::PROTECTED } else { Permissions::default() }
    }

    fn glyph(self) -> Icon {
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

/// The canvas beside a dock at the end edge: an iconic column holding [History], then the
/// expanded column with [Swatches], [Properties, Adjustments] and [Layers, Channels].
fn default_workspace() -> Workspace<Panel> {
    let expanded = Column::new(
        vec![
            Group::new(vec![Panel::Swatches], 0),
            Group::new(vec![Panel::Properties, Panel::Adjustments], 0),
            Group::new(vec![Panel::Layers, Panel::Channels], 0),
        ],
        WIDTH,
    );
    let mut iconic = Column::new(vec![Group::new(vec![Panel::History], 0)], WIDTH);
    iconic.iconic = true;
    let root = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(WIDTH),
        first: Box::new(Node::Tabs { panels: vec![Panel::Canvas], active: 0 }),
        // The first column is the one at the end edge.
        second: Box::new(Node::Dock { side: Side::End, columns: vec![expanded, iconic] }),
    };
    Workspace::new(Layout { root: Some(root), floating: Vec::new() })
}

struct Content<'a> {
    mock: &'a mut Mock,
    panels: &'a mut State,
    chrome: Option<Chrome<'a>>,
    /// Tab was pressed while the canvas had focus.
    tab: bool,
    /// The panel a panel body asked to show.
    requested: Option<Panel>,
}

impl DockContent<Panel> for Content<'_> {
    fn body(&mut self, ui: &mut Ui, panel: &Panel) {
        match panel {
            Panel::Canvas => {
                self.tab |= canvas_region(ui, self.mock, self.chrome);
            }
            other => {
                if let Some(panel) = other.show(ui, self.mock, self.panels) {
                    self.requested = Some(panel);
                }
            }
        }
    }

    fn paint_icon(&mut self, painter: &Painter, rect: Rect, panel: &Panel, color: Color32) -> bool {
        let ctx = painter.ctx();
        let px = icons::pixels(rect.width(), ctx.pixels_per_point());
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(icons::texture(ctx, panel.glyph(), px), rect, uv, color);
        true
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

/// The canvas region: inside the shell, the canvas panel (tab strip above the document, which
/// the shell's backdrop shows through); without one, the canvas alone. Returns whether Tab was
/// pressed on the canvas.
fn canvas_region(ui: &mut Ui, mock: &mut Mock, chrome: Option<Chrome<'_>>) -> bool {
    let Some(chrome) = chrome else {
        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| canvas::show(ui, mock));
        return false;
    };
    let mock = RefCell::new(mock);
    let out = chrome.shell.canvas_panel(ui, |ui| canvas::tabs(ui, &mock.borrow()), |ui| canvas::document(ui, &mut mock.borrow_mut(), Some(chrome)));
    out.inner
}

/// The dock area over everything between the tool rail and the end edge. Its protected canvas
/// panel draws the canvas region.
///
/// With `chrome` the dock shows only while the shell's dock shows ([`ShellCx::dock_visible`]);
/// otherwise the canvas panel fills the rect. Returns whether Tab was pressed on the canvas.
pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut DockState, chrome: Option<Chrome<'_>>) -> bool {
    if chrome.is_some_and(|c| !c.dock || !c.shell.dock_visible()) {
        return canvas_region(ui, mock, chrome);
    }
    let id = ui.scope_id().with("dock");
    let mut config = DockConfig::default();
    config.right_to_left = craft_ui::direction::is_rtl(ui);
    // Panels never close by the user's hand; the Window menu hides and shows them.
    config.close_menu_items = false;
    config.close_button = false;
    let DockState { workspace, panels, auto_collapse, raise, shown_flyout, flip, sized } = state;
    if !*sized {
        // The dock holds the iconic column too; the expanded column keeps WIDTH beside it.
        if let Some(Node::Split { size, .. }) = &mut workspace.layout.root {
            *size = SplitSize::FixedSecond(WIDTH + DockLook::of(ui).iconic_width);
        }
        *sized = true;
    }
    if let Some(panel) = flip.take() {
        if flyout::<Panel>(ui.ctx(), id) == Some(panel) {
            close_flyout::<Panel>(ui.ctx(), id);
        } else {
            *raise = Some(panel);
        }
    }
    // A floated panel stays inside the part of the preview that shows.
    let bounds = ui.max_rect().intersect(ui.clip_rect());
    let mut content = Content { mock, panels, chrome, tab: false, requested: None };
    let mut actions = Vec::new();
    let mut requested = None;
    let mut tab = false;
    CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let mut area = DockArea::new(id).native_windows(false).float_bounds(bounds).locked(workspace.locked).auto_collapse_iconic(*auto_collapse);
        if let Some(panel) = raise.take() {
            area = area.raise_panel(panel);
        }
        let output = area.show_content(ui, &workspace.layout, &config, &mut content);
        if output.events.iter().any(|e| matches!(e, DockEvent::ToggleAutoCollapse)) {
            *auto_collapse = !*auto_collapse;
        }
        actions = output.actions;
        requested = content.requested;
        tab = content.tab;
        *shown_flyout = flyout::<Panel>(ui.ctx(), id);
    });
    for action in actions {
        let _ = workspace.apply(action, Panel::permissions);
    }
    if let Some(panel) = requested {
        let _ = workspace.set_visible(panel, true, None, Panel::permissions);
    }
    tab
}
