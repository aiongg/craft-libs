#![forbid(unsafe_code)]

//! Every blend mode once, for the CPU and in WGSL.
//!
//! [`BlendMode`] lists the modes of every app (PhotoCraft's Photoshop and Paint.NET modes,
//! After Effects' Classic and alpha modes, Substance's signed and normal-map modes) with stable
//! ids. Where apps compute a shared mode differently, [`Variant`] picks whose formula applies;
//! [`Range`] says whether values above 1 are light. [`Blend`] holds both and evaluates
//! `B(Cb, Cs)` ([`Blend::channel`], [`Blend::rgb`], Photoshop's Fill in [`Blend::rgb_fill`])
//! and composites a pixel ([`Blend::composite`], straight or premultiplied, in a blend and a mix
//! [`Space`]).
//!
//! The same formulas in WGSL are in [`wgsl`]: one [`craft_wgsl::Module`] per variant, with mode
//! numbers generated from the enum. The GPU test target compares the two.
//!
//! PhotoCraft checks these formulas against Photoshop's own composites (its psd-tools corpus
//! gate); a formula edit here needs that job's result.

pub mod cmyk;
mod composite;
mod fill;
mod formula;
mod mode;
mod space;
pub mod wgsl;

#[cfg(test)]
mod tests;

pub use composite::{Alpha, Layer, Weights, composite_normal_span};
pub use fill::fill_is_special;
pub use formula::{Blend, EDGE, lum, set_lum};
pub use mode::{BlendMode, Range, Variant};
pub use space::{Linear, Space, Srgb, TEXT_GAMMA, TextGamma};
