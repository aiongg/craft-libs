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
use egui::{Frame, Panel, Rect, Ui, UiBuilder, Vec2, vec2};

use dock::DockState;
use model::{Mock, Tool};

pub(crate) use story::stories;

/// The size the app lays out at when its host is smaller; the host clips the rest. A desktop
/// window-style minimum, so a small Overview tile or Variants card shows the top-left part of a
/// real layout.
const MIN_SIZE: Vec2 = vec2(960.0, 600.0);

/// The mock app and the state of its chrome.
#[derive(Clone, Debug, Default)]
pub struct AppPreview {
    pub(crate) mock: Mock,
    dock: DockState,
    last_used: LastUsed<Tool>,
    home: bool,
}

impl AppPreview {
    /// Draws the whole app into `ui.max_rect()`, laid out at least [`MIN_SIZE`] and clipped to the rect.
    ///
    /// The bars are `egui::Panel`s shown inside a child `Ui` limited to that rectangle, so the
    /// app stays within the dock panel or story frame that holds it.
    pub fn show(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let layout = Rect::from_min_size(rect.min, rect.size().max(MIN_SIZE));
        let background = Tokens::of(ui).color.surface.background.to_egui();
        let mut ui = ui.new_child(UiBuilder::new().id_salt("app-preview").max_rect(layout));
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let ui = &mut ui;
        ui.painter().rect_filled(layout, 0.0, background);
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
        dialog::show(ui, &mut self.mock);
        tools::handle_keys(ui, &mut self.mock);
    }
}

#[cfg(test)]
mod tests;
