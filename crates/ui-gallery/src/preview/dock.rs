//! The dock column at the end edge.

use craft_ui::direction;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{CentralPanel, Frame, Ui};

use super::canvas;
use super::model::Mock;
use super::panels::Panel;

/// The width of the dock column.
pub const WIDTH: f32 = 290.0;

/// Which panels the dock shows.
#[derive(Clone, Debug, Default)]
pub struct DockState {
    hidden: Vec<Panel>,
}

impl DockState {
    pub fn is_visible(&self, panel: Panel) -> bool {
        !self.hidden.contains(&panel)
    }

    /// Shows a hidden panel and hides a shown one.
    pub fn toggle(&mut self, panel: Panel) {
        match self.hidden.iter().position(|p| *p == panel) {
            Some(at) => {
                self.hidden.remove(at);
            }
            None => self.hidden.push(panel),
        }
    }
}

/// Everything between the tool rail and the end edge: the fixed column on `surface.panel`, and
/// the canvas region in the remaining centre.
pub fn show(ui: &mut Ui, mock: &mut Mock, _state: &mut DockState) {
    let tokens = Tokens::of(ui);
    let frame = Frame::NONE.fill(tokens.color.surface.panel.to_egui());
    drop(tokens);
    let id = ui.scope_id().with("dock");
    direction::end_panel(ui, id).exact_size(WIDTH).resizable(false).show_separator_line(false).frame(frame).show(ui, |ui| {
        ui.label("Panels");
    });
    CentralPanel::default().frame(Frame::NONE).show(ui, |ui| canvas::show(ui, mock));
}
