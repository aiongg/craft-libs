//! [`SliceStyle`]: the slice overlay's look, from tokens (task TK3).

use egui::{Color32, FontId, Stroke, Vec2, vec2};

use super::SliceTool;
use craft_theme::TypeRole;
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{ToEgui, UiTokens};

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
    pub color: Color32,
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
    /// Handle size and reach from the `Handle` painter, `Handle::style(tokens, HandleShape::Square)`
    /// (7 and 14).
    pub handle: f32,
    pub handle_hit: f32,
    /// A handle's fill: `overlay.handle_fill`.
    pub handle_fill: Color32,
    /// A handle's edge: the `Handle` painter's stroke width in `overlay.slice_selected`, as the
    /// artboard handles use theirs.
    pub handle_edge: Stroke,
}

impl SliceTool {
    /// The slice look in `tokens`.
    pub fn style(tokens: &UiTokens) -> SliceStyle {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-321 `draw_overlay` (the values in the doc
        // comments), :249 (the slice colour, default #38b5ff: crates/engine/src/prefs.rs:842).
        let (s, o) = (&tokens.component.slice, &tokens.color.overlay);
        let handle = Handle::style(tokens, HandleShape::Square);
        SliceStyle {
            outline: s.outline,
            selected_outline: s.selected_outline,
            color: o.slice.to_egui(),
            selected: o.slice_selected.to_egui(),
            auto_fill: o.slice_auto.to_egui(),
            text: o.slice_text.to_egui(),
            auto_dash: (s.auto_dash, s.auto_gap),
            auto_alpha: s.auto_alpha,
            drag_dash: (s.drag_dash, s.drag_gap),
            badge_offset: s.badge_offset,
            badge_pad: vec2(s.badge_pad[0], s.badge_pad[1]),
            badge_radius: s.badge_radius,
            badge_min: vec2(s.badge_min[0], s.badge_min[1]),
            badge_font: tokens.font(TypeRole::Caption),
            click_slop: s.click_slop,
            handle: handle.size,
            handle_hit: handle.hit,
            handle_fill: handle.fill,
            handle_edge: Stroke::new(handle.stroke.width, o.slice_selected.to_egui()),
        }
    }
}
