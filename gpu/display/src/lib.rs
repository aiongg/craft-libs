#![forbid(unsafe_code)]

//! Display colour for the Craft suite: how a canvas or viewer shows its pixels on a monitor.
//!
//! - [`monitor`]: the monitors (read by craft-platform), the profile each display resolves to
//!   and why ([`MonitorReport`]).
//! - [`transform`]: [`DisplayTransform`], a composite's profile to the monitor's (relative
//!   colorimetric with BPC), skipped when it changes nothing; linear composites stored
//!   sRGB-encoded.
//! - [`DisplayCaches`]: the monitor list, profiles and transforms of an app, shared by its
//!   windows.
//! - [`pass`]: the display pass, [`DisplayPass`]: Show Channel, exposure, a pending HDR step, a
//!   preview gamma and the display colour (shaper tables or a LUT), on the CPU.
//! - [`gpu`]: the same pass in WGSL, with its uniform and texture bytes. The GPU test target
//!   holds it to the CPU pass.
//!
//! SDR: the pass ends in 0..1 for an 8-bit target.

mod caches;
pub mod gpu;
pub mod monitor;
pub mod pass;
pub mod transform;

pub use caches::DisplayCaches;
pub use monitor::{DISPLAY_INTENT, Detection, DisplayId, Monitor, MonitorProblem, MonitorReport, MonitorStatus, MonitorsError, display_at};
pub use pass::{Channel, Curve, DisplayLut, DisplayPass, FrameEncoding, GamutOverlay, HdrStep, PassColor, shaper_from_curves};
pub use transform::{DisplayTransform, rgb_view};

#[cfg(test)]
mod tests;
