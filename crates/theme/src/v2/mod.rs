//! The v2 token model's primitives: sRGB and OKLCH colour, WCAG contrast, 12-step colour ramps
//! and the fixed scales for space, radius, stroke, elevation, motion and opacity.
//!
//! Pure data and functions, like the rest of the crate. This module replaces the pilot model at
//! the crate root once the rest of the design system lands.

pub mod color;
pub mod contrast;
pub mod ramp;
pub mod scale;

#[cfg(test)]
mod tests;

pub use color::{Oklab, Oklch, Rgba};
pub use contrast::{contrast_ratio, ensure_contrast, relative_luminance};
pub use ramp::{Mode, Ramp, RampKind};
pub use scale::{Duration, Easing, Elevation, Opacity, Radius, Space, Stroke};
