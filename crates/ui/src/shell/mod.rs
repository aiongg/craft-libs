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

// Start screen and app windows.
mod recent;
mod start;
mod viewport_window;

pub use recent::{RecentFile, RecentSort, RecentView, Template, ThumbKey, sort_recent};
pub use start::{StartAction, StartOutput, StartPinnedCommands, StartRecentCommands, StartScreen};
pub use viewport_window::{ViewportWindow, ViewportWindowOutput};
pub use words::StartWords;
