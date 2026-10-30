//! [`Fit`]: which scales leave a usable window on the current monitor.

use egui::{Vec2, vec2};

use super::InterfaceScale;

/// The smallest logical window the app supports, and the monitor's size in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    /// The smallest window, in points, the app lays out without clipping.
    pub min_logical: Vec2,
    /// The monitor's size in physical pixels; `None` when unknown, which allows every scale.
    pub monitor_px: Option<Vec2>,
}

impl Default for Fit {
    fn default() -> Self {
        Self { min_logical: vec2(900.0, 600.0), monitor_px: None }
    }
}

impl Fit {
    /// Whether `scale` leaves the monitor at least `min_logical` points in both axes. `Auto` is
    /// always allowed, so the way back to the display's own scale never disappears.
    pub fn allows(self, scale: InterfaceScale) -> bool {
        match (scale, self.monitor_px) {
            (InterfaceScale::Percent(p), Some(monitor)) => {
                let ppp = f32::from(p.get()) / 100.0;
                monitor.x / ppp >= self.min_logical.x && monitor.y / ppp >= self.min_logical.y
            }
            _ => true,
        }
    }
}
