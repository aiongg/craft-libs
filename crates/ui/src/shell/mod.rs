//! `AppShell` and its presets, `TitleBar`, captions, `ToastHost` and `app::run`.

#[cfg(feature = "app")]
pub mod app;
pub mod title_bar;
mod words;

// The caption record: the exclusive window parts a title bar registers.
mod captions;

pub use captions::{Caption, CaptionRegions, WindowHit};

// Shell interface: the slot API.
mod regions;

pub use regions::{Slot, SlotSize};

// App shell, canvas preset, screen modes and reveal.
mod app_shell;
mod canvas_shell;
mod reveal;
mod screen_mode;

pub use app_shell::{AppShell, AppShellOutput, ShellRegion, ShellRegionOutput};
pub use canvas_shell::{CanvasPanel, CanvasPanelOutput, CanvasShell, slots};
pub use reveal::{Aim, PanelsHidden, Reveal, Tracking, chrome_toggles};
pub use screen_mode::{FullScreenPlan, ScreenMode, ScreenModeCommand};
pub use words::AppShellWords;
