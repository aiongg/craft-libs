//! The GPU devices of a Craft app, and their health.
//!
//! Engine tier: no egui, eframe or craft-ui-* dependency. craft-ui-gpu joins this crate to eframe
//! (the window's device, the first-frame watch, the fault notice).
//!
//! - [`GpuAccess`]: a device, its queue, adapter and [`DeviceHealth`]. The window's device is
//!   adopted from eframe; a device of its own (tools, tests, export, workers) is requested here.
//! - [`DeviceHealth`]: the one fault handler of a device. wgpu keeps one uncaptured-error handler
//!   and one device-lost callback per device, so nothing but [`DeviceHealth::watch`] registers
//!   either; everything else subscribes.
//! - [`scope`]: error scopes, for work whose failure the caller handles.
//! - [`limits`]: what a device asks its adapter for.
//! - [`software_rasterizer`]: whether an adapter runs on the CPU.
//! - [`memory`]: a card's own and free memory, and physical memory (native only).
//! - [`adapter`]: which adapter a window or device draws with (native only).
//! - [`env`]: the one set of environment variables that steers the GPU.
//! - [`start`]: the window's graphics start, the ladder of ways to start it and what follows when
//!   one fails, with the start-up record (native only).

pub mod access;
#[cfg(not(target_arch = "wasm32"))]
pub mod adapter;
pub mod env;
pub mod health;
pub mod limits;
#[cfg(not(target_arch = "wasm32"))]
pub mod memory;
pub mod scope;
pub mod software;
#[cfg(not(target_arch = "wasm32"))]
pub mod start;

pub use access::GpuAccess;
pub use health::{DeviceHealth, Fault, HealthEvent, OUT_OF_MEMORY, Subscription, WaitOutcome};
pub use limits::{DeviceRequest, LimitsTier, device_descriptor, device_limits};
#[cfg(not(target_arch = "wasm32"))]
pub use scope::scoped;
pub use scope::{Checked, ScopeError, ScopeKind, checked};
pub use software::software_rasterizer;
#[cfg(not(target_arch = "wasm32"))]
pub use start::{Attempt, Choice, Failure, GraphicsStart, Keep, Next, RestartState, Rung};
