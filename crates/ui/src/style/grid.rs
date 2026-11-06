//! Style of the `grid` family.

use egui::Color32;

use crate::theme::{ToEgui as _, UiTokens};

const MINOR_ALPHA: f32 = 0.45;
const MIN_STEP: f32 = 6.0;
const CROSSING_MIN: f32 = 8.0;

/// The grid's look, from [`CanvasGrid::style`](crate::canvas::CanvasGrid::style).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GridStyle {
    /// Line width: `stroke.thin`.
    pub width: f32,
    /// Minor lines' share of the colour's alpha.
    pub minor_alpha: f32,
    /// Smallest step in screen points at which an axis draws.
    pub min_step: f32,
    /// Side of a crossing dot: `stroke.medium`.
    pub crossing: f32,
    /// Smallest major spacing in screen points at which crossings draw.
    pub crossing_min: f32,
    /// Dashes and gaps of the Dashed style: `stroke.dash`, `stroke.dash_gap`.
    pub dash: f32,
    /// The gap length of the Dashed style, in screen points.
    pub gap: f32,
    /// Dot spacing of the Dots style: `stroke.dash_gap`.
    pub dot_gap: f32,
    /// The dot radius of the Dots style, in screen points.
    pub dot_radius: f32,
    /// `overlay.grid`.
    pub color: Color32,
}

impl GridStyle {
    /// The grid style for `tokens`.
    pub fn new(tokens: &UiTokens) -> Self {
        Self {
            width: tokens.stroke.thin,
            minor_alpha: MINOR_ALPHA,
            min_step: MIN_STEP,
            crossing: tokens.stroke.medium,
            crossing_min: CROSSING_MIN,
            dash: tokens.stroke.dash,
            gap: tokens.stroke.dash_gap,
            dot_gap: tokens.stroke.dash_gap,
            dot_radius: crate::style::guide::GuideStyle::new(tokens).dot_radius,
            color: tokens.color.overlay.grid.to_egui(),
        }
    }
}
