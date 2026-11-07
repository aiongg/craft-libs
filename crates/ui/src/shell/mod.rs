//! `AppShell` and its presets, `TitleBar`, captions, window zones, `ToastHost` and `app::run`.

#[cfg(feature = "app")]
pub mod app;
pub mod title_bar;
mod words;

// Shell interface: slots, regions and the shell's context.
mod cx;
mod regions;

pub use cx::ShellCx;
pub use regions::{BodyMode, ShellRegions, Slot, SlotKey, SlotSize, WindowKey};
