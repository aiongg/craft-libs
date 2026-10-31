#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Shared lifecycle mechanics; application state, theme choices and file policy stay in hosts.
#[cfg(feature = "appearance")]
pub mod appearance;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub mod browser;
pub mod dialog_outcome;
#[cfg(all(feature = "dialogs", not(target_arch = "wasm32")))]
pub mod dialogs;
#[cfg(feature = "pointer")]
pub mod pointer;
#[cfg(all(feature = "appearance", unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
mod process;
#[cfg(feature = "session")]
pub mod session;
#[cfg(all(test, feature = "session"))]
mod session_tests;
#[cfg(feature = "usage")]
pub mod usage;
#[cfg(all(test, feature = "usage"))]
mod usage_tests;
