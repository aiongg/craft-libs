#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Shared lifecycle mechanics; application state, theme choices and file policy stay in hosts.
#[cfg(feature = "appearance")]
pub mod appearance;
#[cfg(feature = "autosave")]
pub mod autosave;
#[cfg(all(test, feature = "autosave"))]
mod autosave_tests;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub mod browser;
pub mod dialog_outcome;
#[cfg(all(feature = "dialogs", not(target_arch = "wasm32")))]
pub mod dialogs;
#[cfg(feature = "open")]
pub mod open;
#[cfg(all(test, feature = "open"))]
mod open_tests;
#[cfg(all(feature = "appearance", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod process;
