//! `Panel` and `CentralPanel`: the pane slots of the mount host.

use egui::{Frame, Id, Rect, Ui};

/// Which edge of the parent a pane slot sits on, or `Center` for what the edge panes leave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

/// Shows `body` in the pane slot at `edge` of `ui`. An edge pane is as wide (or as high) as `extent`, the size
/// the caller keeps, and takes no resize handle: the pane's own grip resizes it. It returns the pane's rect.
///
/// A left or right pane counts the width of the room it takes its extent from as used: `ui`'s used rect always holds
/// the layout's start corner at the left, so a right pane would otherwise use the room's left part and a left pane
/// not. Counted this way the used rect is the same whichever edge reading order gives the pane, and the pane lies at
/// its mirrored edge in it. The room left beside the pane stays free for what follows.
pub(crate) fn show_pane(ui: &mut Ui, edge: Edge, id: Id, extent: Option<f32>, body: impl FnOnce(&mut Ui)) -> Rect {
    let room = ui.available_rect_before_wrap();
    let rect = show_panel(ui, edge, id, extent, body);
    if matches!(edge, Edge::Left | Edge::Right) && rect.is_positive() {
        ui.expand_to_include_rect(Rect::from_x_y_ranges(room.x_range(), rect.y_range()));
    }
    rect
}

fn show_panel(ui: &mut Ui, edge: Edge, id: Id, extent: Option<f32>, body: impl FnOnce(&mut Ui)) -> Rect {
    let panel = match edge {
        Edge::Center => return egui::CentralPanel::default().frame(Frame::NONE).show(ui, body).response.rect,
        Edge::Left => egui::Panel::left(id),
        Edge::Right => egui::Panel::right(id),
        Edge::Top => egui::Panel::top(id),
        Edge::Bottom => egui::Panel::bottom(id),
    };
    let panel = panel.frame(Frame::NONE).resizable(false).show_separator_line(false);
    let panel = match extent {
        Some(extent) => panel.exact_size(extent),
        None => panel,
    };
    panel.show(ui, body).response.rect
}
