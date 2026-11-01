//! [`SelectToolStyle`]: the selection tools' sizes and colours, from tokens (task TK1).

use craft_ui::canvas::draft::DraftReach;
use craft_ui::theme::{ToEgui, UiTokens};
use egui::Color32;

/// The share of `overlay.halo`'s alpha under the closing ring (PhotoCraft's black at alpha 200).
const RING_HALO_ALPHA: f32 = 0.78;
/// The share of `overlay.halo`'s alpha in the width circle's dark ring (alpha 140).
const WIDTH_RING_DARK_ALPHA: f32 = 0.55;
/// The share of `overlay.handle_fill`'s alpha in the width circle's light ring (alpha 220).
const WIDTH_RING_LIGHT_ALPHA: f32 = 0.86;

/// What the selection tools draw with and how near a press closes an outline. Every size is a
/// token of the `select` family, the same at every Size (the marks sit on the document). The
/// outline being drawn and the marquee's ants use `Marquee::style` (`marquee.*`), not this.
///
/// Named `SelectToolStyle`, not `SelectStyle`: `craft_ui::controls::SelectStyle` is the drop-down's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectToolStyle {
    /// A press this near the first point closes an outline of 3 or more points, in screen points:
    /// `select.close_radius` (8). The Magnetic Lasso's first-point radius too.
    pub close_radius: f64,
    /// A press this near the last point is the same click again and closes: `select.same_point`
    /// (3).
    pub same_point: f64,
    /// The side of a Magnetic Lasso fastening point's square: `select.fastening` (5).
    pub fastening: f32,
    /// The fastening point's fill: `overlay.handle_fill` (white).
    pub fastening_fill: Color32,
    /// The fastening point's 1 pt outline: `overlay.handle_stroke` (black).
    pub fastening_stroke: Color32,
    /// The Magnetic Lasso's closing ring: radius `select.ring_radius` (4), its centre
    /// `select.ring_offset` (12, 12) right of and below the pointer (mirrored in neither
    /// direction).
    pub ring_radius: f32,
    pub ring_offset: f32,
    /// The ring's halo: `select.ring_halo` (3) of `overlay.halo` at 0.78 (PhotoCraft's black at
    /// alpha 200).
    pub ring_halo: f32,
    pub ring_halo_color: Color32,
    /// The ring's line over the halo: `select.ring_line` (1.2) of `overlay.handle_fill`.
    pub ring_line: f32,
    pub ring_line_color: Color32,
    /// The Magnetic Lasso's width circle with Other Cursors › Precise: a 1 pt ring of
    /// `overlay.halo` at 0.55 half a point outside a 1 pt ring of `overlay.handle_fill` at 0.86
    /// (PhotoCraft's black at alpha 140 and white at alpha 220).
    pub width_ring_dark: Color32,
    pub width_ring_light: Color32,
}

impl SelectToolStyle {
    /// The selection tools' look in `tokens`.
    pub fn of(tokens: &UiTokens) -> SelectToolStyle {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:48 (`SAME_POINT` 3), :170 (8);
        // crates/ui-egui/src/tools/magnetic_lasso_ui.rs:81 (8), :426-428 (5 × 5, white, 1 pt black),
        // :434-436 (offset 12, radius 4, 3 pt black alpha 200, 1.2 pt white), :444-446 (the width
        // circle: black alpha 140 at `r + 0.5`, white alpha 220 at `r`).
        let (s, o) = (&tokens.component.select, &tokens.color.overlay);
        SelectToolStyle {
            close_radius: f64::from(s.close_radius),
            same_point: f64::from(s.same_point),
            fastening: s.fastening,
            fastening_fill: o.handle_fill.to_egui(),
            fastening_stroke: o.handle_stroke.to_egui(),
            ring_radius: s.ring_radius,
            ring_offset: s.ring_offset,
            ring_halo: s.ring_halo,
            ring_halo_color: o.halo.faded(RING_HALO_ALPHA).to_egui(),
            ring_line: s.ring_line,
            ring_line_color: o.handle_fill.to_egui(),
            width_ring_dark: o.halo.faded(WIDTH_RING_DARK_ALPHA).to_egui(),
            width_ring_light: o.handle_fill.faded(WIDTH_RING_LIGHT_ALPHA).to_egui(),
        }
    }

    /// The close radii an [`OutlineDraft`](craft_ui::canvas::draft::OutlineDraft) takes.
    pub fn reach(&self) -> DraftReach {
        DraftReach { close: self.close_radius, same_point: self.same_point }
    }
}
