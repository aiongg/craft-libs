//! Per-pass and per-frame services and the one egui adapter.

pub mod access;
pub mod commands;
pub mod cursor;
pub mod drag;
pub mod edit;
pub mod egui;
pub mod env;
pub mod focus;
pub mod frame;
pub mod ime;
pub mod inspect;
pub mod keys;
pub mod install;
pub mod memory;
pub mod pointer;
pub mod toast;
pub mod windows;

pub use install::{InstallError, InstallSpec, install};
