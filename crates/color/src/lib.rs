//! Colour for Craft apps: the colour picker, wells, swatch tables, ramps and the hex field.
//!
//! The widgets are behind the `ui` feature; the colour model and conversions are in `craft-theme`.
//! Without `ui`, the crate has the helpers that need no egui: [`AreaMode`], [`GrayModel`],
//! [`CmykModel`], hex text ([`parse_hex`], [`format_hex`]), web-safe colours ([`web_safe`],
//! [`not_web_safe`]), swatch files ([`palette`]), the display transform ([`display`]) and colour
//! harmonies ([`harmony`]).
//! The `serde` feature derives `Serialize` and `Deserialize` for [`AreaMode`] (and for
//! `ui::RampKind`), as its [`key`](AreaMode::key), and for the swatch and harmony settings types.

mod cmyk;
pub mod display;
mod gray;
pub mod harmony;
mod hex;
mod mode;
pub mod palette;
#[cfg(feature = "ui")]
pub mod ui;
mod web;

pub use cmyk::CmykModel;
pub use gray::GrayModel;
pub use hex::{format_hex, parse_hex};
pub use mode::AreaMode;
pub use web::{not_web_safe, web_safe};
