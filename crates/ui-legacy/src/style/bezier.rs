//! Style of the `bezier` family.

use egui::Color32;

use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui as _, UiTokens};

/// Largest distance between a curve and the lines it is drawn with.
const FLATTEN_TOLERANCE: f32 = 0.3;
/// Width of the close cue's ring.
const CLOSE_RING_WIDTH: f32 = 3.5;

/// A path overlay's look, from `BezierOverlay::style`, and the geometry the pen shares. A host
/// preference (VectorCraft's anchor size and handle style) changes a copy and passes it to the
/// overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BezierStyle {
    /// Outline width.
    pub line: f32,
    /// A selected segment's width.
    pub selected_segment: f32,
    /// A path without a colour of its own: `overlay.line`.
    pub color: Color32,
    /// An anchor: a `size.handle.sm` square of `overlay.handle_fill` outlined in
    /// `overlay.handle_stroke`, or in the path's colour.
    pub anchor: HandleStyle,
    /// A selected anchor: filled with `overlay.handle_stroke`, or the path's colour.
    pub selected_anchor: HandleStyle,
    /// A direction handle's end: a solid `size.handle.sm` dot of `overlay.handle_stroke`, or the
    /// path's colour; its line from the anchor is the same colour.
    pub control: HandleStyle,
    /// The line from an anchor to a handle's end.
    pub control_line: f32,
    /// Distance within which the pointer takes a handle, an anchor or a segment:
    /// `size.hit.target`.
    pub hit: f32,
    /// Farthest a press may move and still be a click.
    pub click_slop: f32,
    /// Largest distance between a curve and the lines it is drawn with.
    pub flatten: f32,
    /// Side of an anchor drawn by `BezierOverlay::highlight`.
    pub highlight_anchor: f32,
    /// Under a highlighted segment: `overlay.halo`, the thin stroke width wider on each side.
    pub highlight_halo: f32,
    /// The colour of [`highlight_halo`](Self::highlight_halo): `overlay.halo`.
    pub halo: Color32,
    /// Width of the close cue's ring beside the pointer.
    pub close_ring: f32,
    /// Distance across and down from the pointer to the close cue's ring.
    pub close_offset: f32,
}

impl BezierStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &UiTokens) -> Self {
        let (stroke, size) = (&tokens.stroke, &tokens.size);
        let sized = |style: HandleStyle| HandleStyle { size: size.handle.sm, hit: size.handle.sm + 2.0 * size.hit.target, ..style };
        Self {
            line: stroke.thin,
            selected_segment: stroke.thick,
            color: tokens.color.overlay.line.to_egui(),
            anchor: sized(Handle::style(tokens, HandleShape::Square)),
            selected_anchor: sized(Handle::selected_style(tokens, HandleShape::Square)),
            control: sized(Handle::style(tokens, HandleShape::Rotate)),
            control_line: stroke.thin,
            hit: size.hit.target,
            click_slop: size.hit.slop,
            flatten: FLATTEN_TOLERANCE,
            highlight_anchor: size.handle.md,
            highlight_halo: stroke.thin,
            halo: tokens.color.overlay.halo.to_egui(),
            close_ring: CLOSE_RING_WIDTH,
            close_offset: size.handle.md + tokens.space.gap.inline,
        }
    }
}
