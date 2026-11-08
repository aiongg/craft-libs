//! Screen modes (the F key) and the window actions that enter and leave full screen.

use crate::runtime::windows::WindowAction;
use craft_host::target::WindowState;

/// The three screen modes. The shell keeps the current one in its `screen_mode` binding and maps it to the body mode,
/// the document tabs and the scroll bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
    /// Whether the shell shows the body only.
    pub fn body_only(self) -> bool {
        self == Self::FullScreen
    }

    /// Whether the document tabs show.
    pub fn tabs_visible(self) -> bool {
        self == Self::Standard
    }

    /// Whether the scroll bars show; the app's canvas draws them.
    pub fn scrollbars_visible(self) -> bool {
        self == Self::Standard
    }

    /// Whether the window is in OS full screen.
    pub fn is_full_screen(self) -> bool {
        self != Self::Standard
    }

    /// The mode the F key selects: Standard, Full Screen With Menu Bar, Full Screen, Standard.
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

/// The screen-mode commands of a window. The three modes are radio rows of View > Screen Mode; the cycle commands are
/// the F key and Shift+F. The window's shell takes them and writes its `screen_mode` binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, craft_ui_macros::CommandSet)]
#[command(prefix = "screenMode", scope = "window")]
#[non_exhaustive]
pub enum ScreenModeCommand {
    /// Everything shows.
    #[command(label = "Standard Screen Mode", kind = "radio", group = "screen_mode")]
    Standard,
    /// Every bar, rail and panel stays; the document tabs and the scroll bars hide.
    #[command(label = "Full Screen Mode With Menu Bar", kind = "radio", group = "screen_mode", alias = "screenMode.fullScreenWithMenuBar")]
    FullScreenWithMenuBar,
    /// The body only.
    #[command(label = "Full Screen Mode", kind = "radio", group = "screen_mode", alias = "screenMode.fullScreen")]
    FullScreen,
    /// The next mode.
    #[command(label = "Cycle Screen Mode", default = "F")]
    Cycle,
    /// The previous mode.
    #[command(label = "Cycle Screen Mode Backwards", default = "Shift+F")]
    CycleBack,
}

impl ScreenModeCommand {
    /// The command that selects `mode`.
    pub const fn of(mode: ScreenMode) -> Self {
        match mode {
            ScreenMode::Standard => ScreenModeCommand::Standard,
            ScreenMode::FullScreenWithMenuBar => ScreenModeCommand::FullScreenWithMenuBar,
            ScreenMode::FullScreen => ScreenModeCommand::FullScreen,
        }
    }

    /// The mode the command leaves a window in that is in `current`.
    pub fn apply(self, current: ScreenMode) -> ScreenMode {
        match self {
            ScreenModeCommand::Standard => ScreenMode::Standard,
            ScreenModeCommand::FullScreenWithMenuBar => ScreenMode::FullScreenWithMenuBar,
            ScreenModeCommand::FullScreen => ScreenMode::FullScreen,
            ScreenModeCommand::Cycle => current.next(),
            ScreenModeCommand::CycleBack => current.previous(),
        }
    }
}

/// The window actions that bring a window to full screen or back after a screen-mode change.
///
/// `WindowAction` has toggles only and `WindowHost::act` reads the window state refreshed once per pass, so the plan is
/// a small machine that looks at the state each pass and sends at most one action, then waits for the state to change.
///
/// With `keep_apart` (Windows), full screen and maximized never overlap: a maximized window is un-maximized before it
/// goes full screen and maximized again after it leaves.
///
/// WORKAROUND(winit#4756): winit on Windows keeps the maximized style on a window that goes full screen from
/// maximized, and the shell then draws the taskbar black instead of hiding it; on the way out the window stays
/// maximized at the monitor's full size, over the taskbar. Remove `keep_apart` once the winit in Cargo.lock has the
/// fix; check first with the reproduction gist of that issue (`--max`, no `--fix`: full screen must have no
/// WS_MAXIMIZE and hide the taskbar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullScreenPlan {
    active: bool,
    want_full: bool,
    was_maximized: bool,
    awaiting: Option<WindowState>,
}

impl FullScreenPlan {
    /// Starts moving the window to full screen (`want_full`) or out of it.
    pub fn start(&mut self, want_full: bool) {
        self.active = true;
        self.want_full = want_full;
        self.awaiting = None;
    }

    /// Whether a plan is running.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The next action for a window that reports `state` in this pass, if any. After an action the plan sends nothing
    /// until the state differs from the one it was sent in.
    pub fn step(&mut self, keep_apart: bool, state: WindowState) -> Option<WindowAction> {
        if !self.active || self.awaiting == Some(state) {
            return None;
        }
        self.awaiting = None;
        if state == WindowState::Minimized {
            self.active = false;
            return None;
        }
        let full = state == WindowState::Fullscreen;
        let action = if self.want_full {
            if full {
                self.active = false;
                return None;
            }
            if keep_apart && state == WindowState::Maximized {
                self.was_maximized = true;
                WindowAction::ToggleMaximize
            } else {
                WindowAction::ToggleFullscreen
            }
        } else if full {
            WindowAction::ToggleFullscreen
        } else if keep_apart && self.was_maximized && state == WindowState::Normal {
            self.was_maximized = false;
            WindowAction::ToggleMaximize
        } else {
            self.active = false;
            self.was_maximized = false;
            return None;
        };
        self.awaiting = Some(state);
        Some(action)
    }
}
