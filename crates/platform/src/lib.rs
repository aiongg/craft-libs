#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Shared lifecycle mechanics; application state, theme choices and file policy stay in hosts.
#[cfg(feature = "appearance")]
pub mod appearance;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub mod browser;
#[cfg(feature = "clock")]
pub mod clock;
pub mod dialog_outcome;
#[cfg(all(feature = "dialogs", not(target_arch = "wasm32")))]
pub mod dialogs;
#[cfg(feature = "logs")]
pub mod logs;
#[cfg(feature = "pacing")]
pub mod pacing;
#[cfg(all(feature = "appearance", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod process;
