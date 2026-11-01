//! [`SliceStyle`]: the slice overlay's look, from tokens (task TK3).

use egui::{Color32, FontId, Vec2};

use super::SliceTool;
use craft_ui::theme::UiTokens;

/// What [`SliceOverlay`](super::SliceOverlay) and [`SliceTool`] draw. The same at every Size (it
/// sits on the document).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliceStyle {
    /// A slice's outline width: `slice.outline` (1).
    pub outline: f32,
    /// The selected slice's outline width: `slice.selected_outline` (2).
    pub selected_outline: f32,
    /// The slice colour: `overlay.slice` (#38b5ff), unless the preference overrides it.
    pub colour: Color32,
    /// `overlay.slice_selected` (#ffb000).
    pub selected: Color32,
    /// An auto slice's badge fill: `overlay.slice_auto` (grey 110).
    pub auto_fill: Color32,
    /// The badge text: `overlay.slice_text` (white).
    pub text: Color32,
    /// An auto slice's dash and gap: `slice.auto_dash`, `slice.auto_gap` (3, 3).
    pub auto_dash: (f32, f32),
    /// An auto slice's alpha over the slice colour: `slice.auto_alpha` (0.7).
    pub auto_alpha: f32,
    /// A new slice's preview dash and gap: `slice.drag_dash`, `slice.drag_gap` (4, 4).
    pub drag_dash: (f32, f32),
    /// The badge's offset from the slice's corner: `slice.badge_offset` (2).
    pub badge_offset: f32,
    /// The badge's padding around the number: `slice.badge_pad` (3, 1).
    pub badge_pad: Vec2,
    /// `slice.badge_radius` (2).
    pub badge_radius: f32,
    /// The smallest slice on screen that shows a badge: `slice.badge_min` (18, 12).
    pub badge_min: Vec2,
    /// The badge's font: the caption role.
    pub badge_font: FontId,
    /// A drag no wider and no taller than this, in document pixels, is a click:
    /// `slice.click_slop` (2).
    pub click_slop: f32,
    /// Handle size and reach: `handle.size` (7), `handle.hit` (14).
    pub handle: f32,
    pub handle_hit: f32,
    /// A handle's fill: `overlay.handle_fill`.
    pub handle_fill: Color32,
}

impl SliceTool {
    /// The slice look in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the token family `slice` lands in task TK3")]
    pub fn style(tokens: &UiTokens) -> SliceStyle {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-321 `draw_overlay` (the values in the doc
        // comments), :249 (the slice colour, default #38b5ff: crates/engine/src/prefs.rs:842).
        todo!()
    }
}
