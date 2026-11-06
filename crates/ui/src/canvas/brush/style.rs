//! The look of the brush cursor and the resize gesture.

use egui::Color32;

use crate::canvas::{CursorHost, CursorStyle};
use crate::theme::{ToEgui, UiTokens};

const TIP_MIN_RADIUS: f32 = 3.0;
const TIP_MIN_CROSSHAIR: f32 = 4.0;
const RESIZE_TRAVEL: f32 = 200.0;
const PREVIEW_TINT_SHARE: f32 = 0.43;

/// The sizes and colours of the brush pointers and the resize gesture: the cursor family's
/// [`CursorStyle`] and the kernel leaves of the `brush` family. The same at every Size (they sit
/// on the pointer and the document).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStyle {
    /// The cursor line, halo and crosshair reach ([`CursorHost::style`](crate::canvas::CursorHost::style)).
    pub cursor: CursorStyle,
    /// Below this on-screen radius in points an outline is unreadable and the crosshair shows.
    pub tip_min_radius: f32,
    /// The arm of that crosshair, no gap.
    pub tip_min_crosshair: f32,
    /// Screen points of vertical resize travel from level 0 to 1.
    pub resize_travel: f32,
    /// The resize preview's default tint (Preferences › Cursors › Brush Preview): `status.bad.solid`
    /// at 43%. Apps pass the user's colour in
    /// [`BrushResizeOptions::tint`](super::BrushResizeOptions::tint).
    pub preview_tint: Color32,
}

impl BrushStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        BrushStyle {
            cursor: CursorHost::style(tokens),
            tip_min_radius: TIP_MIN_RADIUS,
            tip_min_crosshair: TIP_MIN_CROSSHAIR,
            resize_travel: RESIZE_TRAVEL,
            preview_tint: tokens.color.status.bad.solid.faded(PREVIEW_TINT_SHARE).to_egui(),
        }
    }
}
