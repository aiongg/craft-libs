//! Screen modes (Photoshop's F key) and the window commands that enter and leave full screen.

use egui::{Context, ViewportCommand};

use super::ids::{self, WindowKey};

/// Photoshop's three screen modes. The host keeps the current one, starts in `Standard` and maps
/// it to [`BodyMode`](super::BodyMode), the document tabs and the scroll bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScreenMode {
    /// Everything shows.
    #[default]
    Standard,
    /// Every bar, rail and panel stays; the document tabs and the scroll bars hide.
    FullScreenWithMenuBar,
    /// The body only.
    FullScreen,
}

impl ScreenMode {
    /// Whether the shell runs in [`BodyMode::Only`](super::BodyMode::Only).
    pub fn body_only(self) -> bool {
        self == Self::FullScreen
    }

    /// Whether the document tabs show.
    pub fn tabs_visible(self) -> bool {
        self == Self::Standard
    }

    /// Whether the scroll bars show (the host draws them).
    pub fn scrollbars_visible(self) -> bool {
        self == Self::Standard
    }

    /// Whether the window is in OS full screen.
    pub fn is_full_screen(self) -> bool {
        self != Self::Standard
    }

    /// The mode the F key selects: Standard, With Menu Bar, Full Screen, Standard.
    pub fn next(self) -> Self {
        match self {
            Self::Standard => Self::FullScreenWithMenuBar,
            Self::FullScreenWithMenuBar => Self::FullScreen,
            Self::FullScreen => Self::Standard,
        }
    }

    /// The mode Shift+F selects.
    pub fn previous(self) -> Self {
        match self {
            Self::Standard => Self::FullScreen,
            Self::FullScreenWithMenuBar => Self::Standard,
            Self::FullScreen => Self::FullScreenWithMenuBar,
        }
    }
}

/// The window commands for switching `window` to screen mode `mode`; the host sends them with
/// `ctx.send_viewport_cmd`. Also ends the window's Tab and edge reveal, as every mode change does.
///
/// With `keep_apart` (Windows), full screen and maximized never overlap: a maximized window is
/// un-maximized before it goes full screen and maximized again after it leaves. Why: winit on
/// Windows keeps the maximized style on a window that goes full screen from maximized, and the
/// shell then draws the taskbar black instead of hiding it; on the way out the window stays
/// maximized at the monitor's full size, over the taskbar (#3550, #4018, #4230). [OBSERVED
/// 2026-10-12, Windows 11, eframe 0.36.2, winit 0.30.13, a minimal eframe window; un-maximizing
/// first gave a clean full screen and a work-area maximize.]
///
/// WORKAROUND(winit#4756): remove `keep_apart` once the winit in Cargo.lock (0.30.13 today) has
/// the fix; check first with https://gist.github.com/aweussom/3f80ffb82df7f0102341858afbbffd9b
/// (`--max`, no `--fix`: full screen must have no WS_MAXIMIZE and hide the taskbar).
pub fn full_screen_commands(ctx: &Context, window: &WindowKey, mode: ScreenMode, keep_apart: bool) -> Vec<ViewportCommand> {
    let (full_screen, maximized) = ctx.input(|i| (i.viewport().fullscreen.unwrap_or(false), i.viewport().maximized.unwrap_or(false)));
    let mut state = ids::state(ctx, window);
    let (commands, was_maximized) = plan(keep_apart, full_screen, maximized, state.was_maximized, mode.is_full_screen());
    state.was_maximized = was_maximized;
    state.tab = false;
    state.edges = [false; 3];
    ids::set_state(ctx, window, state);
    ctx.request_repaint();
    commands
}

/// The commands, and whether the window should be maximized again when it leaves full screen.
pub(super) fn plan(keep_apart: bool, full_screen: bool, maximized: bool, was_maximized: bool, want: bool) -> (Vec<ViewportCommand>, bool) {
    let mut commands = Vec::new();
    let mut remember = was_maximized;
    if keep_apart && want && !full_screen && maximized {
        commands.push(ViewportCommand::Maximized(false));
        remember = true;
    }
    commands.push(ViewportCommand::Fullscreen(want));
    if !want {
        if keep_apart && full_screen && remember {
            commands.push(ViewportCommand::Maximized(true));
        }
        remember = false;
    }
    (commands, remember)
}
