#![cfg(not(target_arch = "wasm32"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

//! Drive a running Craft app from tests, agents and the CLI.
//!
//! A transport thread ([`server`], or a channel in tests) turns each JSON line into a
//! [`control::ControlRequest`]; the UI thread drains them between frames through
//! [`control::ControlChannel`], runs the app's method table on each, and settles the
//! [`control::Reply`]: a reply now, a reply once queued synthetic input ([`synthetic`]) has been
//! processed, a reply once a background job ends, or a screenshot through the bounded queue
//! ([`screenshot`]). [`profiler`] and [`trace`] time frames for the channel's `perf` output.
//!
//! The crate never names a document: what `engine.execute`, `ui.inspect` or `ui.set` mean is the
//! app's, through its method table. Everything a method table does not want to own is here.

pub mod args;
pub mod control;
pub mod profiler;
pub mod protocol;
pub mod screenshot;
pub mod security;
pub mod synthetic;
pub mod trace;

#[cfg(feature = "tcp")]
pub mod client;
#[cfg(feature = "tcp")]
pub mod server;

pub use control::{ControlChannel, ControlRequest, ControlResponse, Dispatch, REPLY_DEADLINE, Reply};
pub use screenshot::Captures;
pub use security::AutomationError;
