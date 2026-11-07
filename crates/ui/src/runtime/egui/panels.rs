//! `Panel` and `CentralPanel`: the pane slots of the mount host.

use egui::{Frame, Id, Ui};

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
/// its content took last pass, and takes no resize handle: the pane's own grips resize it.
pub(crate) fn show_pane<R>(ui: &mut Ui, edge: Edge, id: Id, extent: Option<f32>, body: impl FnOnce(&mut Ui) -> R) -> R {
    let panel = match edge {
        Edge::Center => return egui::CentralPanel::default().frame(Frame::NONE).show(ui, body).inner,
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
    panel.show(ui, body).inner
}
