//! Colour for Craft apps: the colour picker, wells, swatch tables, ramps and the hex field.
//!
//! The widgets are behind the `ui` feature; the colour model and conversions are in `craft-theme`.
//! Without `ui`, the crate has the helpers that need no egui: [`AreaMode`], [`GrayModel`], hex
//! text ([`parse_hex`], [`format_hex`]) and web-safe colours ([`web_safe`], [`not_web_safe`]).
//! The `serde` feature derives `Serialize` and `Deserialize` for [`AreaMode`] (and for
//! `ui::RampKind`), as its [`key`](AreaMode::key).

mod gray;
mod hex;
mod mode;
#[cfg(feature = "ui")]
pub mod ui;
mod web;

pub use gray::GrayModel;
pub use hex::{format_hex, parse_hex};
pub use mode::AreaMode;
pub use web::{not_web_safe, web_safe};
