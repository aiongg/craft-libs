//! Test helpers for craft-ui and the apps.
//!
//! - [`dump`]: ordered `key = value` text dumps compared with a file (`UPDATE_DUMPS=1` rewrites).
//! - [`screens`] (`screen-testing` feature, desktop): screenshot and IME smoke tests in the
//!   suite's test languages; its items are re-exported here.
//! - [`a11y`] (`screen-testing` feature, desktop): role, label, Tab order and focus ring checks
//!   on a kittest harness.
//! - [`symbols`] (`screen-testing` feature, desktop): craft-fonts' symbol faces for a kittest
//!   harness, from `CRAFT_FONTS_DIR`.
//! - [`wgpu_renderer`] (`screen-testing` feature, desktop): a kittest wgpu renderer on the
//!   process's one device; use it for every rendering harness.

pub mod dump;

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod a11y;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod gpu;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub use gpu::wgpu_renderer;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod screens;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub use screens::*;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod symbols;

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;
