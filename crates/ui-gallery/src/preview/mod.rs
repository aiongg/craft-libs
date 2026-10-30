//! A mock photo editor, built only from library components, drawn into the `Ui` it is given.
//!
//! The window is PhotoCraft's, laid out by `craft_ui::shell::CanvasShell`: title bar with menus,
//! options bar, tool rail at the start edge, a dock at the end edge (an iconic column beside an
//! expanded one) around the canvas panel, and a status bar. [`AppPreview::show`] fills
//! `ui.max_rect()`; the caller decides where that is (a gallery story today, the Theme
//! workspace's centre later).

mod canvas;
mod chrome;
mod dialog;
mod dock;
mod model;
mod options;
mod panels;
mod story;
mod tools;

use std::cell::RefCell;

use craft_ui::dialog::DialogState;
use craft_ui::patterns::LastUsed;
use craft_ui::shell::{AppShell, BodyMode, CanvasLabels, CanvasShell, ScreenMode, WindowKey};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{Rect, Ui, UiBuilder, Vec2, vec2};

use dock::DockState;
use model::{ImageSizeDraft, Mock, Tool};

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
    image_size: DialogState<ImageSizeDraft>,
}

impl AppPreview {
    /// A copy to draw in another context, with the dock flyout that is open here open there too.
    pub(crate) fn for_probe(&self) -> Self {
        let mut copy = self.clone();
        copy.dock.reopen_flyout();
        copy
    }

    /// Opens the History flyout.
    #[cfg(test)]
    pub(crate) fn open_history_flyout(&mut self) {
        self.dock.toggle(panels::Panel::History);
    }

    /// Draws the whole app into `ui.max_rect()`, laid out at least [`MIN_SIZE`] and clipped to the rect.
    ///
    /// The app is a [`CanvasShell`] in a child `Ui` limited to that rectangle, so it stays within
    /// the dock panel or story frame that holds it.
    pub fn show(&mut self, ui: &mut Ui) {
        self.show_with(ui, ShellView::default());
    }

    /// The key of the shell window that [`show_with`](Self::show_with) draws in `ui`.
    pub(crate) fn window_key(ui: &Ui) -> WindowKey {
        WindowKey::new(format!("app-preview-{:x}", ui.scope_id().value()))
    }

    /// Draws the app with the slots and screen mode of `view`. In a body-only mode, Tab with the
    /// canvas focused toggles the shell's reveal.
    pub(crate) fn show_with(&mut self, ui: &mut Ui, view: ShellView) {
        let key = Self::window_key(ui);
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let layout = Rect::from_min_size(rect.min, rect.size().max(MIN_SIZE));
        let background = Tokens::of(ui).color.surface.background.to_egui();
        let mut ui = ui.new_child(UiBuilder::new().id_salt("app-preview").max_rect(layout));
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let ui = &mut ui;
        ui.painter().rect_filled(layout, 0.0, background);
        // The slot closures and the body run one after the other; the cell hands each the app.
        let this = RefCell::new(&mut *self);
        let labels = CanvasLabels { options: Some("Options".to_owned()), tools: Some("Tools".to_owned()), status: Some("Status".to_owned()) };
        let mode = view.mode;
        let body_mode = if mode.body_only() { BodyMode::Only } else { BodyMode::Normal };
        let shell = CanvasShell::new(key.clone())
            .title(view.title, |ui| {
                let mut guard = this.borrow_mut();
                let this = &mut **guard;
                chrome::title_bar(ui, &mut this.mock, &mut this.dock);
            })
            .options(view.options, |ui| {
                let mut guard = this.borrow_mut();
                let this = &mut **guard;
                options::show(ui, &mut this.mock, &mut this.home);
            })
            .tools(view.tools, |ui| {
                let mut guard = this.borrow_mut();
                let this = &mut **guard;
                tools::rail(ui, &mut this.mock, &mut this.last_used);
            })
            .status(view.status, |ui| {
                let mut guard = this.borrow_mut();
                let this = &mut **guard;
                chrome::status_bar(ui, &mut this.mock);
            })
            .labels(labels)
            .body_mode(body_mode)
            .document_tabs(view.tabs && mode.tabs_visible())
            .body_label("Workspace");
        let out = shell.show(ui, |ui, cx| {
            let mut guard = this.borrow_mut();
            let this = &mut **guard;
            let chrome = canvas::Chrome { shell: cx, scrollbars: mode.scrollbars_visible(), dock: view.dock };
            dock::show(ui, &mut this.mock, &mut this.dock, Some(chrome))
        });
        if out.inner {
            AppShell::toggle_reveal(&ctx, &key);
        }
        let this = this.into_inner();
        dialog::show(ui, &mut this.mock, &mut this.image_size);
        tools::handle_keys(ui, &mut this.mock);
    }
}

/// What the host of the mock app shows: the slot flags and the screen mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellView {
    pub title: bool,
    pub options: bool,
    pub tools: bool,
    pub dock: bool,
    pub status: bool,
    pub tabs: bool,
    pub mode: ScreenMode,
}

impl Default for ShellView {
    fn default() -> Self {
        Self { title: true, options: true, tools: true, dock: true, status: true, tabs: true, mode: ScreenMode::Standard }
    }
}

#[cfg(test)]
mod tests;
