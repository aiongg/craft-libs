//! Per-pass and per-frame services and the one egui adapter.

pub mod access;
#[cfg(feature = "testing")]
pub mod commands;
pub mod cursor;
pub mod derived;
pub mod drag;
pub mod edit;
pub mod egui;
pub mod env;
pub mod focus;
pub mod frame;
pub mod ime;
pub(crate) mod inputs;
pub mod inspect;
pub mod install;
pub mod keys;
pub mod layout_memory;
pub(crate) mod lazy;
#[doc(hidden)]
pub mod lock_keys;
#[doc(hidden)]
pub mod measure;
pub mod memory;
pub mod motion;
pub mod overlay;
pub mod pen;
pub mod pending;
pub mod pointer;
pub mod requests;
pub mod sampler;
pub(crate) mod shift;
pub mod toast;
pub(crate) mod toggle_sweep;
pub mod wheel;
pub mod windows;
pub mod words;

pub use ::egui::RecordShift;
pub use install::{InstallError, InstallSpec, install};
#[doc(hidden)]
pub use shift::{Clip, Shift};
