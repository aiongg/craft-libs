//! Stories of [`DockArea`]: the docked, floating, drag-preview and iconic looks, drawn from the
//! `dock` and `float` tokens of the gallery's theme.

use egui::{Id, Rect, Sense, Ui, UiBuilder, pos2, vec2};

use super::chrome::drop_preview_with;
use super::{Column, DockArea, DockConfig, DockContent, DockLook, Floating, Group, Layout, Node, Permissions, Side, flyout, open_flyout};
use crate::direction;
use crate::layout::{SplitAxis, SplitSize};
use crate::story::{Args, Level, Story};

const CANVAS: u32 = 0;
const LAYERS: u32 = 1;
const CHANNELS: u32 = 2;
const PROPERTIES: u32 = 3;
const BRUSHES: u32 = 4;
const SWATCHES: u32 = 5;
const HISTORY: u32 = 6;

const HEIGHT: f32 = 360.0;
const LOCALES: &[&str] = &["en", "ar"];

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["DockArea"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_number("width", 640.0, 360.0, 960.0).with_choice("locale", LOCALES, 0);
    let story = |id, name, show| Story { id, level: Level::Patterns, component: "DockArea", name, args, show };
    vec![
        story("patterns/dock-area/docked", "Docked", show_docked),
        story("patterns/dock-area/floating", "Floating", show_floating),
        story("patterns/dock-area/drag", "Drag", show_drag),
        story("patterns/dock-area/iconic", "Iconic", show_iconic),
    ]
}

struct Panels;

impl DockContent<u32> for Panels {
    fn body(&mut self, ui: &mut Ui, panel: &u32) {
        ui.label(match *panel {
            CANVAS => "Canvas",
            _ => "Panel content",
        });
    }

    fn logical_label(&self, panel: &u32) -> String {
        match *panel {
            CANVAS => "Document",
            LAYERS => "Layers",
            CHANNELS => "Channels",
            PROPERTIES => "Properties",
            BRUSHES => "Brushes",
            SWATCHES => "Swatches",
            HISTORY => "History",
            _ => "Panel",
        }
        .into()
    }

    fn permissions(&self, panel: &u32) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::default() }
    }
}

fn canvas() -> Node<u32> {
    Node::Tabs { panels: vec![CANVAS], active: 0 }
}

/// `canvas` beside a right dock of `columns`, which together are `dock_width` wide.
fn right_dock(columns: Vec<Column<u32>>, dock_width: f32) -> Node<u32> {
    Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(dock_width),
        first: Box::new(canvas()),
        second: Box::new(Node::Dock { side: Side::End, columns }),
    }
}

fn iconic_column() -> Column<u32> {
    let mut column = Column::new(vec![Group::new(vec![BRUSHES], 0), Group::new(vec![SWATCHES], 0), Group::new(vec![HISTORY], 0)], 120.0);
    column.iconic = true;
    column
}

fn docked_layout() -> Layout<u32> {
    let main = Column::new(vec![Group::new(vec![LAYERS, CHANNELS], 0), Group::new(vec![PROPERTIES], 0)], 200.0);
    Layout { root: Some(right_dock(vec![main, iconic_column()], 250.0)), floating: Vec::new() }
}

fn floating_layout() -> Layout<u32> {
    let mut minimized = Floating::single(vec![HISTORY], 0, [250.0, 200.0, 180.0, 120.0]).with_id(2);
    minimized.minimized = true;
    Layout { root: Some(canvas()), floating: vec![Floating::single(vec![LAYERS, CHANNELS], 0, [24.0, 24.0, 200.0, 240.0]).with_id(1), minimized] }
}

fn iconic_layout() -> Layout<u32> {
    Layout { root: Some(right_dock(vec![iconic_column()], 36.0)), floating: Vec::new() }
}

/// Draw `initial` (kept in the context so edits stay) in an area `width` wide, and return the
/// area's rect.
fn show_area(ui: &mut Ui, args: &Args, salt: &'static str, initial: fn() -> Layout<u32>, open: Option<u32>) -> (Id, Rect) {
    let width = (args.number("width") as f32).min(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::hover());
    let id = ui.scope_id().with(("dock-story", salt));
    let key = id.with("layout");
    let layout = ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, initial).clone());
    let config = DockConfig { right_to_left: args.choice("locale") == "ar" || direction::is_rtl(ui), ..DockConfig::default() };
    if let Some(panel) = open.filter(|panel| flyout::<u32>(ui.ctx(), id).is_none() && layout.contains(panel)) {
        open_flyout(ui.ctx(), id, panel);
    }
    let output = ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| DockArea::new(id).show_content(ui, &layout, &config, &mut Panels));
    let mut next = layout;
    for action in output.inner.actions {
        let _ = next.apply_with_permissions(action, |panel| Panels.permissions(panel));
    }
    ui.data_mut(|d| d.insert_temp(key, next));
    (id, rect)
}

/// A right dock with two groups, tabs and a collapsed iconic column beside a canvas.
pub(super) fn show_docked(ui: &mut Ui, args: &mut Args) {
    show_area(ui, args, "docked", docked_layout, None);
}

/// One floating window with a tab group, and one minimized.
pub(super) fn show_floating(ui: &mut Ui, args: &mut Args) {
    show_area(ui, args, "floating", floating_layout, None);
}

/// The docked layout with the previews of a tab dragged over a group (a tab drop), over the dock's
/// edge (a new column) and over the canvas. A pointer drag draws these live; the story holds them
/// still.
pub(super) fn show_drag(ui: &mut Ui, args: &mut Args) {
    let (_, rect) = show_area(ui, args, "drag", docked_layout, None);
    let look = DockLook::of(ui);
    let painter = ui.painter_at(rect);
    let dock = Rect::from_min_max(pos2(rect.right() - 250.0, rect.top()), rect.right_bottom());
    let group = Rect::from_min_size(dock.min + vec2(0.0, 4.0), vec2(200.0, 150.0));
    let edge = Rect::from_min_size(dock.min, vec2(40.0, dock.height()));
    let canvas = Rect::from_min_max(rect.min, pos2(dock.left() - 8.0, rect.bottom())).shrink(30.0);
    drop_preview_with(&painter, group, &look, true);
    drop_preview_with(&painter, canvas, &look, false);
    drop_preview_with(&painter, edge, &look, false);
}

/// An iconic column with the flyout of its first group open.
pub(super) fn show_iconic(ui: &mut Ui, args: &mut Args) {
    show_area(ui, args, "iconic", iconic_layout, Some(BRUSHES));
}
