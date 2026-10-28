//! Type roles as egui text formats.

use craft_theme::{TypeRole, TypeStyle};
use egui::{Color32, TextFormat};

use super::convert::FontFamilies;
use super::tokens::UiTokens;

/// The OpenType tag of tabular (equal-width) figures.
pub const TABULAR_FIGURES: &[u8; 4] = b"tnum";

/// The format of `role` in `tokens`: the role's font (family of its font slot, size), its line
/// height and `color`.
pub fn role_format(tokens: &UiTokens, role: TypeRole, color: Color32) -> TextFormat {
    style_format(tokens.families(), tokens.typography.get(role), color)
}

/// The format of a type style drawn with `families`.
pub fn style_format(families: &FontFamilies, style: TypeStyle, color: Color32) -> TextFormat {
    TextFormat { font_id: families.font_id(style), line_height: Some(style.line_height), color, ..TextFormat::default() }
}

/// `format` with tabular figures, so digits are equally wide and columns of numbers line up. Faces
/// without a `tnum` feature draw as before.
pub fn tabular(mut format: TextFormat) -> TextFormat {
    format.font_features.push(TABULAR_FIGURES, 1);
    format
}
