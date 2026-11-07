//! `AppShell` and its presets, `TitleBar`, captions, `ToastHost` and `app::run`.

#[cfg(feature = "app")]
pub mod app;
pub mod title_bar;
mod words;

// The caption record: the exclusive window parts a title bar registers.
mod captions;

pub use captions::{Caption, CaptionRegions, WindowHit};

// Shell interface: slots, regions and the shell's context.
mod cx;
mod regions;

pub use cx::ShellCx;
pub use regions::{BodyMode, ShellRegions, Slot, SlotKey, SlotSize, WindowKey};
