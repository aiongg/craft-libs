//! Per-pass and per-frame services and the one egui adapter.

pub mod access;
pub mod commands;
pub mod cursor;
pub mod derived;
pub mod drag;
pub mod edit;
pub mod env;
pub mod focus;
pub mod ime;
pub mod inspect;
pub mod keys;
#[doc(hidden)]
#[doc(hidden)]
pub mod memory;
pub mod motion;
pub mod overlay;
pub mod pending;
pub mod pointer;
pub mod requests;
pub mod sampler;
pub(crate) mod shift;
pub mod toast;
pub mod windows;
pub mod words;

pub use shift::RecordShift;
#[doc(hidden)]
pub use shift::{Clip, Shift};
