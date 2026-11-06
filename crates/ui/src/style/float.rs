//! Style of the `float` family.

use crate::theme::{UiTokens, role_format};
use craft_theme::{Tokens, TypeRole};
use egui::text::LayoutJob;
use egui::{Color32, Painter, Vec2, vec2};

/// Characters of body text across a new floating window.
const SAMPLE: &str = "00000000000000000000000000000000";
/// Lines of body text down a new floating window.
const LINES: f32 = 16.0;
/// Least height of a floating window that fits its content.
const MIN_FITTED_HEIGHT: f32 = 120.0;
/// Least width of a floating window.
const MIN_WIDTH: f32 = 40.0;
/// Least height of a floating window's body.
const MIN_BODY_HEIGHT: f32 = 20.0;

/// Sizes of floating windows that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatStyle {
    /// Least height of a floating window that fits its content.
    pub min_fitted_height: f32,
    /// Least width of a floating window.
    pub min_width: f32,
    /// Least height of a floating window's body.
    pub min_body_height: f32,
}

impl FloatStyle {
    /// The style for `tokens`.
    pub fn new(_tokens: &Tokens) -> Self {
        Self { min_fitted_height: MIN_FITTED_HEIGHT, min_width: MIN_WIDTH, min_body_height: MIN_BODY_HEIGHT }
    }

    /// Size of a new floating window: a line of sample text wide and a number of body lines
    /// tall, in the body type role.
    pub fn size(painter: &Painter, tokens: &UiTokens) -> Vec2 {
        let job = LayoutJob::single_section(SAMPLE.to_owned(), role_format(tokens, TypeRole::Body, Color32::PLACEHOLDER));
        vec2(painter.layout_job(job).size().x, LINES * tokens.typography.get(TypeRole::Body).line_height)
    }
}
