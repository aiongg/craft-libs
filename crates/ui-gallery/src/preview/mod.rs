//! A mock photo editor, built only from library components, drawn into the `Ui` it is given.
//!
//! The window is PhotoCraft's: title bar with menus, options bar, tool rail at the start edge,
//! a canvas region, a dock at the end edge and a status bar. [`AppPreview::show`] fills
//! `ui.max_rect()`; the caller decides where that is (a gallery story today, the Theme
//! workspace's centre later).

#![allow(dead_code, unused_imports, reason = "the canvas, dock and dialog steps use the rest of the model and the stubs' functions")]

mod canvas;
mod chrome;
mod dialog;
mod dock;
mod model;
mod options;
mod panels;
mod story;
mod tools;

use craft_ui::patterns::LastUsed;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{CentralPanel, Frame, Panel, Ui, UiBuilder};

use dock::DockState;
use model::{Mock, Tool};

pub(crate) use story::stories;

/// The mock app and the state of its chrome.
#[derive(Clone, Debug, Default)]
pub struct AppPreview {
    pub(crate) mock: Mock,
    dock: DockState,
    last_used: LastUsed<Tool>,
    home: bool,
}

impl AppPreview {
    /// Draws the whole app into `ui.max_rect()`.
    ///
    /// The bars are `egui::Panel`s shown inside a child `Ui` limited to that rectangle, so the
    /// app stays within the dock panel or story frame that holds it.
    pub fn show(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let background = Tokens::of(ui).color.surface.background.to_egui();
        let builder = UiBuilder::new().id_salt("app-preview").max_rect(rect);
        ui.scope_builder(builder, |ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            ui.painter().rect_filled(rect, 0.0, background);
            tools::handle_keys(ui, &mut self.mock);
            let id = ui.scope_id();
            Panel::top(id.with("title")).frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| {
                chrome::title_bar(ui, &mut self.mock, &mut self.dock);
            });
            Panel::top(id.with("options")).frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| {
                options::show(ui, &mut self.mock, &mut self.home);
            });
            Panel::bottom(id.with("status")).frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| {
                chrome::status_bar(ui, &mut self.mock);
            });
            tools::rail(ui, &mut self.mock, &mut self.last_used);
            dock::show(ui, &mut self.mock, &mut self.dock);
            CentralPanel::default().frame(Frame::NONE).show(ui, |ui| canvas::show(ui, &mut self.mock));
            dialog::show(ui, &mut self.mock);
        });
    }
}

#[cfg(test)]
mod tests;
