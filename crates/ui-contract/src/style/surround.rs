//! The canvas surround: the fill around a document and the theme's pattern over it.

use super::UiTokens;
use crate::primitives::CheckerArea;
use egui::Color32;

/// How the surround around a canvas looks: its fill and the pattern drawn over it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SurroundLook {
    /// The surround's fill.
    pub fill: Color32,
    /// The pattern's colour; `None` draws no pattern.
    pub pattern: Option<Color32>,
}

impl SurroundLook {
    /// The surround of the theme `tokens` resolve from.
    pub fn of(tokens: &UiTokens) -> Self {
        let _ = tokens;
        todo!("cw.C47")
    }

    /// The same look with the pattern kept when `pattern_on`, else dropped.
    pub fn shown(self, pattern_on: bool) -> Self {
        let _ = pattern_on;
        todo!("cw.C47")
    }
}

/// Paints the surround `look` over `area`.
pub fn paint_surround(painter: &egui::Painter, area: CheckerArea, look: &SurroundLook) {
    let _ = (painter, area, look);
    todo!("cw.C47")
}
