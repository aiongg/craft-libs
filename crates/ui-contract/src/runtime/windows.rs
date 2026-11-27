//! Window values the contract keeps: the window spec and level, the window root and its registry. The driver is
//! `craft_ui_rt::Driver`; the platform crates own the windows.

pub use crate::contract::rt::scope::{WindowEnv, window_root, window_root_with};
pub use crate::contract::rt::windows::{WindowRegistry, caption_regions};
pub use hit::{CaptionRegions, HitSink, ResizeEdge, WindowHit};
pub(crate) use spec::chrome_of as chrome_of_frame;
pub use spec::{WindowLevel, WindowSpec};

mod hit;
mod spec;
