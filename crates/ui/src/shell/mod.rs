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
pub use screen_mode::{ScreenMode, ScreenModeCommand};
pub use words::AppShellWords;

// Start screen and app windows.
mod recent;
mod start;
mod viewport_window;

pub use recent::{RecentFile, RecentSort, RecentView, Template, ThumbKey, sort_recent};
pub use start::{StartAction, StartOutput, StartPinnedCommands, StartRecentCommands, StartScreen};
pub use viewport_window::{ViewportWindow, ViewportWindowOutput};
pub use words::StartWords;

// Caption buttons: the four families' button, its slot layout and the icon-theme glyphs.
mod caption_button;
mod caption_icons;

pub use caption_button::{CaptionButton, CaptionEnd, CaptionMetrics, CaptionPlan, CaptionSlot, WINDOWS_BUTTON_WIDTH};
pub use caption_icons::{CaptionIcon, CaptionIcons, CaptionMask, IconThemeChain, RenderIconFn, ThemeFiles, base_dirs, caption_icon_theme};
pub use craft_window::policy::{CaptionFamily, Captions, Frame, MacInset};
pub use words::CaptionLabels;

// A document's window: a container in its own OS viewport.
mod document_window;

pub use document_window::{DocumentWindow, DocumentWindowOutput, DocumentWindowStyle};
