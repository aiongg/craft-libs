//! The look of the brush cursor and the resize gesture.
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Color32;

use crate::canvas::CursorStyle;
use crate::theme::UiTokens;

/// The sizes and colours of the brush pointers and the resize gesture: the cursor family's
/// [`CursorStyle`] and the kernel leaves of the `brush` family. The same at every Size (they sit
/// on the pointer and the document).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStyle {
    /// The cursor line, halo and crosshair reach ([`CursorHost::style`](crate::canvas::CursorHost::style)).
    pub cursor: CursorStyle,
    /// Below this on-screen radius in points an outline is unreadable and the crosshair shows:
    /// `brush.tip_min_radius` (PhotoCraft 3).
    pub tip_min_radius: f32,
    /// The arm of that crosshair, no gap: `brush.tip_min_crosshair` (PhotoCraft 4).
    pub tip_min_crosshair: f32,
    /// Screen points of vertical resize travel from level 0 to 1: `brush.resize_travel`
    /// (PhotoCraft 200).
    pub resize_travel: f32,
    /// The resize preview's default tint (Preferences › Cursors › Brush Preview): `brush.preview_tint`
    /// (PhotoCraft red, α 110). Apps pass the user's colour in
    /// [`BrushResizeOptions::tint`](super::BrushResizeOptions::tint).
    pub preview_tint: Color32,
}

impl BrushStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // new: token task TK1 adds the `brush` family; `cursor` is `CursorHost::style(tokens)`.
        todo!()
    }
}
