//! `AppShell` and its presets, `TitleBar`, captions, window zones, `ToastHost` and `app::run`.

#[cfg(feature = "app")]
pub mod app;
pub mod title_bar;
mod words;

// The window parts: edge zones, the drag strip and the caption record.
mod captions;
mod drag;
mod edges;

pub use captions::{Caption, CaptionHit, CaptionRegions};
pub use drag::{WindowDrag, WindowDragOutput, WindowDragStyle};
pub use edges::{WindowEdges, WindowEdgesOutput, WindowEdgesStyle};

// Shell interface: the slot API.
mod regions;

pub use regions::{Slot, SlotSize};
