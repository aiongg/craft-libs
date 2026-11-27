//! The rounding of a rect's four corners, in whole points. Render backends convert it to their own
//! type.

/// The radius of each corner of a rect, in points.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct CornerRadius {
    /// North-west: the top-left corner.
    pub nw: u8,
    /// North-east: the top-right corner.
    pub ne: u8,
    /// South-west: the bottom-left corner.
    pub sw: u8,
    /// South-east: the bottom-right corner.
    pub se: u8,
}

impl CornerRadius {
    /// No rounding.
    pub const ZERO: CornerRadius = CornerRadius { nw: 0, ne: 0, sw: 0, se: 0 };

    /// The same radius at every corner.
    pub const fn same(radius: u8) -> CornerRadius {
        CornerRadius { nw: radius, ne: radius, sw: radius, se: radius }
    }

    /// Every corner has the same radius.
    pub fn is_same(&self) -> bool {
        self.nw == self.ne && self.nw == self.sw && self.nw == self.se
    }

    /// The mean of the four radii.
    pub fn average(&self) -> f32 {
        (self.nw as f32 + self.ne as f32 + self.sw as f32 + self.se as f32) / 4.0
    }

    /// Each corner at least `min`.
    pub fn at_least(self, min: u8) -> CornerRadius {
        CornerRadius { nw: self.nw.max(min), ne: self.ne.max(min), sw: self.sw.max(min), se: self.se.max(min) }
    }

    /// Each corner at most `max`.
    pub fn at_most(self, max: u8) -> CornerRadius {
        CornerRadius { nw: self.nw.min(max), ne: self.ne.min(max), sw: self.sw.min(max), se: self.se.min(max) }
    }
}

impl From<u8> for CornerRadius {
    fn from(radius: u8) -> CornerRadius {
        CornerRadius::same(radius)
    }
}

impl From<f32> for CornerRadius {
    /// Rounded to the nearest whole point, clamped to `0..=255`.
    fn from(radius: f32) -> CornerRadius {
        CornerRadius::same(radius.round().clamp(0.0, u8::MAX as f32) as u8)
    }
}

impl From<CornerRadius> for craft_ui_rt::CornerRadii {
    fn from(r: CornerRadius) -> craft_ui_rt::CornerRadii {
        craft_ui_rt::CornerRadii { top_left: r.nw as f32, top_right: r.ne as f32, bottom_right: r.se as f32, bottom_left: r.sw as f32 }
    }
}

// The epaint backend's corner type, while ui-contract still paints through it.
impl From<CornerRadius> for egui::CornerRadius {
    fn from(r: CornerRadius) -> egui::CornerRadius {
        egui::CornerRadius { nw: r.nw, ne: r.ne, sw: r.sw, se: r.se }
    }
}

impl From<egui::CornerRadius> for CornerRadius {
    fn from(r: egui::CornerRadius) -> CornerRadius {
        CornerRadius { nw: r.nw, ne: r.ne, sw: r.sw, se: r.se }
    }
}
