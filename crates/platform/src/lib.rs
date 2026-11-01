#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Shared lifecycle mechanics; application state, theme choices and file policy stay in hosts.
#[cfg(feature = "appearance")]
pub mod appearance;
pub mod autosave;
#[cfg(test)]
mod autosave_tests;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub mod browser;
pub mod clock;
pub mod dialog_outcome;
#[cfg(all(feature = "dialogs", not(target_arch = "wasm32")))]
pub mod dialogs;
pub mod logs;
pub mod open;
#[cfg(test)]
mod open_tests;
#[cfg(feature = "pacing")]
pub mod pacing;
#[cfg(feature = "pointer")]
pub mod pointer;
#[cfg(all(feature = "appearance", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod process;
#[cfg(feature = "screen")]
pub mod screen;
pub mod session;
#[cfg(test)]
mod session_tests;
#[cfg(feature = "usage")]
pub mod usage;
#[cfg(all(test, feature = "usage"))]
mod usage_tests;
