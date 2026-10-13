//! The app's side: events from the platform menu in, menu bars out.

use crate::chord::Chord;
use crate::layout::{Clash, Layout};
use crate::model::{MINIMIZE, MenuBar};

/// What the native menu reports.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// An item was chosen with the mouse (or the keyboard inside an open menu).
    Click(String),
    /// A key equivalent was pressed: the shortcut, to be handled as the key press it was.
    Key(Chord),
}

/// A platform's native menu bar.
pub trait Backend {
    /// Bring the menu in step with `bar`: update labels, enabled and checked in place, or
    /// rebuild when its structure changed ([`crate::structure_key`], [`crate::window_key`]).
    fn sync(&mut self, bar: &MenuBar);
    /// Events since the last call. Platform items (Hide, Services, Zoom, …) are handled by the
    /// backend and not reported.
    fn drain(&mut self) -> Vec<Event>;
}

/// The native menu bar, while the app uses one (its in-window menus are hidden then).
pub struct NativeMenu {
    backend: Box<dyn Backend>,
    /// The state the menus were last built from.
    state: Option<u64>,
    clicks: Vec<String>,
    /// An event arrived (AppKit may have toggled a check item itself): re-sync.
    dirty: bool,
    clashes: Vec<Clash>,
}

impl NativeMenu {
    pub fn new(backend: Box<dyn Backend>) -> NativeMenu {
        NativeMenu { backend, state: None, clicks: Vec::new(), dirty: true, clashes: Vec::new() }
    }

    /// Before egui sees this frame's input: key equivalents become the key presses they were,
    /// and clicks wait for [`NativeMenu::take_clicks`].
    ///
    /// AppKit takes only the key-down: winit still delivers the key-up (it forwards key-ups while
    /// ⌘ is held). So a key equivalent adds just its press, ahead of its key-up when that is
    /// already queued. Adding a made-up release as well sends two key-ups; in PhotoCraft, which
    /// reads a ⌘V release without a press as an image paste, that made ⌘V paste three times.
    pub fn raw_input(&mut self, raw: &mut egui::RawInput) {
        for e in self.backend.drain() {
            match e {
                Event::Key(chord) => {
                    if let Some(press @ egui::Event::Key { key: pressed_key, .. }) = chord.key_event(true) {
                        let at = raw
                            .events
                            .iter()
                            .position(|e| matches!(e, egui::Event::Key { key, pressed: false, .. } if *key == pressed_key))
                            .unwrap_or(raw.events.len());
                        raw.events.insert(at, press);
                    }
                }
                Event::Click(id) => self.clicks.push(id),
            }
            self.dirty = true;
        }
    }

    /// The items clicked since the last call, for the app to run like in-window menu clicks.
    /// [`MINIMIZE`] (minimizes the window) and [`craft_quit::QUIT_MENU_ID`] (asks the window to
    /// close, see [`craft_quit::on_menu_event`]) are run here and left out.
    pub fn take_clicks(&mut self, ctx: &egui::Context) -> Vec<String> {
        let mut out = Vec::new();
        for id in std::mem::take(&mut self.clicks) {
            if id == MINIMIZE {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            } else if !craft_quit::on_menu_event(ctx, &id) {
                out.push(id);
            }
        }
        out
    }

    /// Bring the native menu up to date. `state` is a hash of what the menus show (see
    /// [`had_input`]); `build` lays them out ([`crate::mac_layout`]) and runs only when `state`
    /// changed or a menu event arrived since the last sync, since building the rows usually walks
    /// every command's enabled state.
    ///
    /// Returns the layout's clashes when they differ from the last ones returned, for the app to
    /// log; otherwise nothing.
    pub fn sync_with(&mut self, state: u64, build: impl FnOnce() -> Layout) -> Vec<Clash> {
        if self.state == Some(state) && !self.dirty {
            return Vec::new();
        }
        let layout = build();
        self.backend.sync(&layout.bar);
        self.state = Some(state);
        self.dirty = false;
        if layout.clashes == self.clashes {
            return Vec::new();
        }
        self.clashes = layout.clashes.clone();
        layout.clashes
    }
}

/// A key press or click this frame. It may have changed state an app's menu hash doesn't list,
/// so apps fold it into the `state` passed to [`NativeMenu::sync_with`], for example as
/// `had_input(ctx).then_some(ctx.cumulative_frame_nr())`.
pub fn had_input(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::PointerButton { pressed: false, .. })))
}
