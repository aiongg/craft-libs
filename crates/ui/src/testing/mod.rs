//! Test helpers for craft-ui and the apps.
//!
//! - [`dump`]: ordered `key = value` text dumps compared with a file (`UPDATE_DUMPS=1` rewrites).
//! - [`TypingField`]: a text field's typing run beside a canvas overlay under test.
//! - [`canvas_story`]: the story canvas every canvas overlay story draws on.
//! - [`edit_story`]: the args that set an event-log story's edit policy.
//! - [`screens`] (`screen-testing` feature, desktop): screenshot and IME smoke tests in the
//!   suite's test languages; its items are re-exported here.
//! - [`a11y`] (`screen-testing` feature, desktop): role, label, Tab order and focus ring checks
//!   on a kittest harness.
//! - [`symbols`] (`screen-testing` feature, desktop): craft-fonts' symbol faces for a kittest
//!   harness, from `CRAFT_FONTS_DIR`.
//! - [`wgpu_renderer`] (`screen-testing` feature, desktop): a kittest wgpu renderer on the
//!   process's one device; use it for every rendering harness.
//! - [`render`] (`screen-testing` feature, desktop): the exact UI drawn to an image without a
//!   window, at a size and scale.
//! - [`HarnessExt`] (`screen-testing` feature, desktop): clicks, drags, keys and typing one event
//!   per frame on a kittest harness, and pumps that wait for the app's background work.
//! - [`canvas`] (`screen-testing` feature and craft-ui's own tests, desktop): pointer and key input for a tool under a
//!   [`ToolHost`](crate::canvas::ToolHost) on any kittest harness.
//! - [`events`]: the one definition of a click's, drag's, key press's and typing's event frames.
//! - [`i18n_audit`]: interface text drawn without going through translation, found in the pseudo
//!   locale.

pub mod dump;
pub mod events;
pub use events::DRAG_STEPS;
pub mod i18n_audit;
pub mod style_dump;
mod typing_field;
pub use typing_field::TypingField;

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod a11y;
#[cfg(all(any(feature = "screen-testing", test), not(target_arch = "wasm32")))]
pub mod canvas;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod frame;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub use frame::{HarnessExt, PUMP_SLEEP, QUIET_FRAMES};
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod gpu;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub use gpu::wgpu_renderer;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod render;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod screens;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub use screens::*;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
pub mod symbols;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod canvas_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;

/// The args of an event-log story and the [`EditPolicy`](crate::edit::EditPolicy) they set.
pub mod edit_story {
    pub use crate::edit::story::{PolicyArgs, RateArg, policy};
}

/// The story canvas every canvas overlay story draws on: a checkerboard document under a
/// [`CanvasView`](crate::canvas::CanvasView) set by the story's [`ViewArgs`].
pub mod canvas_story {
    pub use crate::canvas::story::{DOC, PlainArgs, PppArg, StoryCanvas, ViewArgs, document, document_at, document_in, document_with};
}
