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
mod host_dialogs;
mod model;
mod options;
mod panels;
mod story;
mod tools;

use std::cell::RefCell;

use craft_ui::dialog::DialogState;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::LastUsed;
use craft_ui::shell::{AppShell, BodyMode, CanvasLabels, CanvasShell, ScreenMode, WindowKey, take_tab_toggle};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{Id, Rect, Ui, UiBuilder, Vec2, vec2};

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
    dialogs: host_dialogs::Hosted,
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

    /// For the host's `raw_input_hook`: takes the Tab press that toggles the reveal of a
    /// body-only preview while no control has the keyboard focus, through the library's
    /// [`take_tab_toggle`]. Any other Tab is left to egui, so focus can leave the canvas.
    pub fn raw_input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
        let state = ctx.data(|d| d.get_temp::<BodyOnly>(Id::unique("app-preview-body-only"))).unwrap_or_default();
        // Only previews drawn in the last pass count; an older list is stale.
        let windows = if state.pass + 1 == ctx.cumulative_pass_nr() { state.windows } else { Vec::new() };
        let focused = ctx.memory(|m| m.focused());
        let Some(window) = windows.iter().find(|w| focused == Some(canvas::canvas_id(w))).or(windows.first()).cloned() else {
            return;
        };
        if take_tab_toggle(ctx, raw) {
            ctx.data_mut(|d| d.insert_temp(Id::unique(TAB_TOGGLE), Some(window)));
        }
    }

    /// Draws the app with the slots and screen mode of `view`. In a body-only mode, a Tab taken
    /// by [`raw_input_hook`](Self::raw_input_hook) toggles the shell's reveal.
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
        let labels = CanvasLabels::of(ui);
        let mode = view.mode;
        let body_mode = if mode.body_only() { BodyMode::Only } else { BodyMode::Normal };
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| {
            let state = d.get_temp_mut_or_default::<BodyOnly>(Id::unique("app-preview-body-only"));
            if state.pass != pass {
                *state = BodyOnly { pass, windows: Vec::new() };
            }
            if mode.body_only() {
                state.windows.push(key.clone());
            }
        });
        let taken = ctx.data_mut(|d| {
            d.get_temp::<Option<WindowKey>>(Id::unique(TAB_TOGGLE)).flatten().filter(|w| *w == key).inspect(|_| {
                d.insert_temp::<Option<WindowKey>>(Id::unique(TAB_TOGGLE), None);
            })
        });
        if taken.is_some() && mode.body_only() {
            AppShell::toggle_reveal(&ctx, &key);
        }
        let shell = CanvasShell::new(key.clone())
            .title(view.title, |ui| {
                let mut guard = this.borrow_mut();
                let this = &mut **guard;
                chrome::title_bar(ui, &mut this.mock, &mut this.dock, this.dialogs.host());
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
        shell.show(ui, |ui, cx| {
            let mut guard = this.borrow_mut();
            let this = &mut **guard;
            let chrome = canvas::Chrome { shell: cx, scrollbars: mode.scrollbars_visible(), dock: view.dock };
            dock::show(ui, &mut this.mock, &mut this.dock, Some(chrome))
        });
        let this = this.into_inner();
        dialog::show(ui, &mut this.mock, &mut this.image_size);
        host_dialogs::show(&ctx, this.dialogs.host(), &mut this.mock);
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

const TAB_TOGGLE: &str = "app-preview-tab-toggle";

/// The shell windows of the previews that were in a body-only mode in pass `pass`, where Tab
/// toggles the reveal. A new pass starts a new list.
#[derive(Clone, Default)]
struct BodyOnly {
    pass: u64,
    windows: Vec<WindowKey>,
}
