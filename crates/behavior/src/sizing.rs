//! `Proposal` and `Metrics`: what a measure call is asked and what a style gives it to add.

use crate::flow::Insets;
use emath::Vec2;

/// The question one measure call answers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Proposal {
    /// The size the content wants, unwrapped.
    Ideal,
    /// The smallest size the content can take.
    Min,
    /// The height at this width.
    Width(f32),
}

/// What the pipeline adds around a component's measured content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Metrics {
    /// Space between the component's edge and its content.
    pub padding: Insets,
    /// The least size of the component.
    pub min_size: Vec2,
    /// The wrap width of text.
    pub max_width: Option<f32>,
    /// Space between content parts.
    pub gap: f32,
    /// The least text width before eliding.
    pub min_text: f32,
}

impl Metrics {
    /// No padding, no minimum, no wrap width.
    pub const fn new() -> Self {
        Metrics { padding: Insets::new(0.0, 0.0, 0.0, 0.0), min_size: Vec2::ZERO, max_width: None, gap: 0.0, min_text: 0.0 }
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

    /// Sets the wrap width.
    pub const fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets the gap between content parts.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Sets the least text width.
    pub const fn min_text(mut self, width: f32) -> Self {
        self.min_text = width;
        self
    }
}
