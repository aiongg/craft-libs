//! [`SliceStyle`]: the slice overlay's look, from tokens.

use egui::{Color32, FontId, Stroke, Vec2, vec2};

use super::SliceTool;
use craft_theme::{Rgba, TypeRole};
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{ToEgui, UiTokens};

const SLICE: Rgba = Rgba::rgb(0x38, 0xb5, 0xff);
const SLICE_SELECTED: Rgba = Rgba::rgb(0xff, 0xb0, 0x00);
const SLICE_AUTO: Rgba = Rgba::rgb(0x6e, 0x6e, 0x6e);
const SLICE_TEXT: Rgba = Rgba::rgb(0xff, 0xff, 0xff);
/// Alpha of an auto slice over the slice colour.
const AUTO_ALPHA: f32 = 0.7;
/// Dash and gap of a new slice's preview.
const DRAG_DASH: (f32, f32) = (4.0, 4.0);
/// A drag no wider and no taller than this, in document pixels, is a click.
const CLICK_SLOP: f32 = 2.0;
/// Digits the smallest badge-bearing slice has room for, and a digit's width as a share of the font size.
const BADGE_MIN_DIGITS: f32 = 2.0;
const BADGE_DIGIT_WIDTH: f32 = 0.6;

/// What [`SliceOverlay`](super::SliceOverlay) and [`SliceTool`] draw. The same at every Size (it
/// sits on the document).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliceStyle {
    /// A slice's outline width.
    pub outline: f32,
    /// The selected slice's outline width.
    pub selected_outline: f32,
    /// The slice colour, unless the preference overrides it.
    pub color: Color32,
    /// The selected slice colour.
    pub selected: Color32,
    /// An auto slice's badge fill.
    pub auto_fill: Color32,
    /// The badge text.
    pub text: Color32,
    /// An auto slice's dash and gap.
    pub auto_dash: (f32, f32),
    /// An auto slice's alpha over the slice colour.
    pub auto_alpha: f32,
    /// A new slice's preview dash and gap.
    pub drag_dash: (f32, f32),
    /// The badge's offset from the slice's corner.
    pub badge_offset: f32,
    /// The badge's padding around the number.
    pub badge_pad: Vec2,
    /// The badge's corner radius.
    pub badge_radius: f32,
    /// The smallest slice on screen that shows a badge.
    pub badge_min: Vec2,
    /// The badge's font: the caption role.
    pub badge_font: FontId,
    /// A drag no wider and no taller than this, in document pixels, is a click.
    pub click_slop: f32,
    /// Handle size and reach from the `Handle` painter, `Handle::style(tokens, HandleShape::Square)`
    /// (7 and 14).
    pub handle: f32,
    /// A handle's grab reach, in screen points.
    pub handle_hit: f32,
    /// A handle's fill: `overlay.handle_fill`.
    pub handle_fill: Color32,
    /// A handle's edge: the `Handle` painter's stroke width in the selected slice colour, as the
    /// artboard handles use theirs.
    pub handle_edge: Stroke,
}

impl SliceTool {
    /// The slice look in `tokens`.
    pub fn style(tokens: &UiTokens) -> SliceStyle {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-321 `draw_overlay` (the values in the doc
        // comments), :249 (the slice colour, default #38b5ff: crates/engine/src/prefs.rs:842).
        let (stroke, space) = (&tokens.stroke, &tokens.space);
        let handle = Handle::style(tokens, HandleShape::Square);
        let badge_font = tokens.font(TypeRole::Caption);
        let badge_pad = vec2(space.gap.tight, stroke.thin);
        // A badge shows only on a slice that holds the number's line of text plus its padding.
        let badge_min = vec2(BADGE_MIN_DIGITS * badge_font.size * BADGE_DIGIT_WIDTH, badge_font.size) + 2.0 * badge_pad;
        SliceStyle {
            outline: stroke.thin,
            selected_outline: stroke.thick,
            color: SLICE.to_egui(),
            selected: SLICE_SELECTED.to_egui(),
            auto_fill: SLICE_AUTO.to_egui(),
            text: SLICE_TEXT.to_egui(),
            auto_dash: (stroke.dash, stroke.dash_gap),
            auto_alpha: AUTO_ALPHA,
            drag_dash: DRAG_DASH,
            badge_offset: space.gap.tight,
            badge_pad,
            badge_radius: tokens.shape.radius.sm,
            badge_min,
            badge_font,
            click_slop: CLICK_SLOP,
            handle: handle.size,
            handle_hit: handle.hit,
            handle_fill: handle.fill,
            handle_edge: Stroke::new(handle.stroke.width, SLICE_SELECTED.to_egui()),
        }
    }
}
