//! Window keys and the ids and memory the shell keys by them.

use std::fmt;

use egui::{Context, Id, Rect};

use super::slot::Edge;

/// Names one window of the app. Equal to the key `craft-window` restore stores the window's
/// placement under (`"main"` for the first window) and to the string behind its viewport id.
/// Every id and every piece of state the shell makes includes it, so two windows never share any.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(String);

impl WindowKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The first window, `"main"`.
    pub fn main() -> Self {
        Self::new("main")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for WindowKey {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

/// What the shell remembers about one window between passes. Temporary: never saved.
#[derive(Clone, Debug, Default)]
pub(super) struct State {
    /// The last pass was in body-only mode.
    pub only: bool,
    /// The last pass allowed a reveal.
    pub allow_reveal: bool,
    /// The Tab reveal is on.
    pub tab: bool,
    /// The edge reveal is on for the top, start and end edges.
    pub edges: [bool; 3],
    /// The slots shown on the last pass, for keeping an edge reveal while the pointer is over one.
    pub shown: Vec<(Edge, Rect)>,
    /// The window was maximized when a screen mode took it to full screen.
    pub was_maximized: bool,
    /// The `movable` flag last given to the macOS window.
    #[cfg(target_os = "macos")]
    pub movable_applied: Option<bool>,
}

/// The pass in which a window's keys were last recorded.
#[derive(Clone, Debug, Default)]
struct Seen {
    pass: u64,
    windows: Vec<WindowKey>,
}

pub(super) fn state_id(window: &WindowKey) -> Id {
    Id::unique(("app_shell", window.as_str(), "state"))
}

pub(super) fn state(ctx: &Context, window: &WindowKey) -> State {
    ctx.data(|d| d.get_temp::<State>(state_id(window))).unwrap_or_default()
}

pub(super) fn set_state(ctx: &Context, window: &WindowKey, state: State) {
    ctx.data_mut(|d| d.insert_temp(state_id(window), state));
}

/// The salt of a slot's panel: the window key and the slot key.
pub(super) fn slot_salt(window: &WindowKey, slot: &'static str) -> Id {
    Id::unique(("app_shell", window.as_str(), "slot", slot))
}

/// The id of the canvas the host registered with [`ShellCx::canvas`](super::ShellCx::canvas),
/// per viewport: a viewport is one window.
fn canvas_key(ctx: &Context) -> Id {
    Id::unique(("app_shell", ctx.viewport_id(), "canvas"))
}

/// The canvas id with the pass that registered it.
#[derive(Clone, Copy)]
struct Registered {
    id: Id,
    pass: u64,
}

/// Registers the canvas of this pass. The body registers it every pass it has a canvas.
pub(super) fn set_canvas(ctx: &Context, id: Id) {
    let (key, pass) = (canvas_key(ctx), ctx.cumulative_pass_nr());
    ctx.data_mut(|d| d.insert_temp(key, Registered { id, pass }));
}

/// The canvas registered on the last pass or this one; none when the body did not register one
/// (a document switch, a start screen), so a stale id never counts as "the canvas has focus".
pub(super) fn canvas(ctx: &Context) -> Option<Id> {
    let (key, pass) = (canvas_key(ctx), ctx.cumulative_pass_nr());
    ctx.data(|d| d.get_temp::<Registered>(key)).filter(|r| r.pass + 1 >= pass).map(|r| r.id)
}

/// Whether a shell with `window` was already shown on this pass; records it.
pub(super) fn shown_before(ctx: &Context, window: &WindowKey) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let key = Id::unique(("app_shell", "seen", ctx.viewport_id()));
    ctx.data_mut(|d| {
        let seen = d.get_temp_mut_or_default::<Seen>(key);
        if seen.pass != pass {
            *seen = Seen { pass, windows: Vec::new() };
        }
        let before = seen.windows.contains(window);
        if !before {
            seen.windows.push(window.clone());
        }
        before
    })
}
