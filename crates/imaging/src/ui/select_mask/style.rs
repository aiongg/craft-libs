//! [`SelectMaskStyle`]: Select and Mask's canvas marks, from tokens.

use craft_ui::canvas::draft::DraftReach;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// How near a press closes a Polygonal Lasso outline, in screen points.
const CLOSE_REACH: f32 = 6.0;
/// An Object Selection drag shorter than this many screen points is a click.
const CLICK_REACH: f32 = 4.0;

/// The opacity of the brush stroke being drawn.
const TRAIL_OPACITY: f32 = 0.43;

/// What Select and Mask draws on the canvas and how near a press counts, (the
/// marks sit on the image, so they do not follow the Size preset).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectMaskStyle {
    /// The Polygonal Lasso's reach: 6 (not the kernel's 8) to close, the kernel's same-point reach for the repeated click.
    pub reach: DraftReach,
    /// An Object Selection drag shorter than this many screen points is a click:
    /// 4.
    pub click_reach: f64,
    /// The brush stroke being drawn: `accent.solid` at 0.43 opacity.
    pub trail: Color32,
}

impl SelectMaskStyle {
    /// The look in `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        Self {
            reach: DraftReach { close: f64::from(CLOSE_REACH), ..DraftReach::of(tokens) },
            click_reach: f64::from(CLICK_REACH),
            trail: tokens.color.accent.solid.faded(TRAIL_OPACITY).to_egui(),
        }
    }
}
