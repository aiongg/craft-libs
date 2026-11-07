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

// Caption buttons: the four families' button, its slot layout and the icon-theme glyphs.
mod caption_button;
mod caption_icons;

pub use caption_button::{CaptionButton, CaptionEnd, CaptionMetrics, CaptionPlan, CaptionSlot, WINDOWS_BUTTON_WIDTH};
pub use caption_icons::{CaptionIcon, CaptionIcons, CaptionMask, IconThemeChain, RenderIconFn, ThemeFiles, base_dirs, caption_icon_theme};
pub use craft_window::policy::{CaptionFamily, Captions, Frame, MacInset};
pub use words::CaptionLabels;
