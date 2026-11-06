//! Stand-ins for craft-behavior's `Metrics` and `Insets`, which the layout track owns.

use egui::Vec2;

/// Padding on the four logical sides.
// F2-MERGE: F2c
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Insets {
    /// Reading start side.
    pub start: f32,
    /// Reading end side.
    pub end: f32,
    /// Top.
    pub top: f32,
    /// Bottom.
    pub bottom: f32,
}

impl Insets {
    /// The same inset on every side.
    pub const fn same(inset: f32) -> Self {
        Insets { start: inset, end: inset, top: inset, bottom: inset }
    }
}

/// What a component's layout reads from its style.
// F2-MERGE: F2c
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Metrics {
    /// Padding the pipeline adds around the content.
    pub padding: Insets,
    /// The least size.
    pub min_size: Vec2,
    /// The wrap width.
    pub max_width: Option<f32>,
    /// The gap between parts.
    pub gap: f32,
    /// The least text width before eliding.
    pub min_text: f32,
}

impl Metrics {
    /// Metrics with no padding, minimum, wrap width or gap.
    pub const fn new() -> Self {
        Metrics { padding: Insets::same(0.0), min_size: Vec2::ZERO, max_width: None, gap: 0.0, min_text: 0.0 }
    }

    /// Sets the padding.
    pub const fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the least size.
    pub const fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Sets the gap.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}
