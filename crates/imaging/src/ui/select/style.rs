//! [`SelectToolStyle`]: the selection tools' sizes and colours, from tokens.

use craft_ui::theme::{ToEgui, UiTokens};
use egui::Color32;

/// The share of `overlay.halo`'s alpha under the closing ring (PhotoCraft's black at alpha 200).
const RING_HALO_ALPHA: f32 = 0.78;
/// The share of `overlay.halo`'s alpha in the width circle's dark ring (alpha 140).
const WIDTH_RING_DARK_ALPHA: f32 = 0.55;
/// The share of `overlay.handle_fill`'s alpha in the width circle's light ring (alpha 220).
const WIDTH_RING_LIGHT_ALPHA: f32 = 0.86;

/// The side of a Magnetic Lasso fastening point's square.
const FASTENING_SIZE: f32 = 5.0;
/// How far the Magnetic Lasso's closing ring lies right of and below the pointer.
const RING_OFFSET: f32 = 12.0;
/// The width circle's smallest radius.
const WIDTH_RING_MIN: f32 = 2.0;
/// How far the width circle's dark ring lies outside the light one.
const WIDTH_RING_GAP: f32 = 0.5;

/// What the selection tools draw with. How near a press closes an outline is
/// [`DraftReach`](craft_ui::canvas::draft::DraftReach). The
/// marks sit on the document, so their sizes do not follow the Size preset. The
/// outline being drawn and the marquee's ants use `Marquee::style` (`marquee.*`), not this.
///
/// Named `SelectToolStyle`, not `SelectStyle`: `craft_ui::controls::SelectStyle` is the drop-down's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectToolStyle {
    /// The side of a Magnetic Lasso fastening point's square: 5.
    pub fastening: f32,
    /// The fastening point's fill: `overlay.handle_fill` (white).
    pub fastening_fill: Color32,
    /// The fastening point's 1 pt outline: `overlay.handle_stroke` (black).
    pub fastening_stroke: Color32,
    /// The Magnetic Lasso's closing ring: radius `shape.radius.sm`, its centre
    /// 12 right of and below the pointer (mirrored in neither
    /// direction).
    pub ring_radius: f32,
    #[allow(missing_docs)]
    pub ring_offset: f32,
    /// The ring's halo: `stroke.medium` plus a `stroke.thin` halo on each side, of `overlay.halo` at 0.78 (PhotoCraft's black at
    /// alpha 200).
    pub ring_halo: f32,
    #[allow(missing_docs)]
    pub ring_halo_color: Color32,
    /// The ring's line over the halo: `stroke.thin` of `overlay.handle_fill`.
    pub ring_line: f32,
    #[allow(missing_docs)]
    pub ring_line_color: Color32,
    /// The Magnetic Lasso's width circle with Other Cursors › Precise: a 1 pt ring of
    /// `overlay.halo` at 0.55 half a point outside a 1 pt ring of `overlay.handle_fill` at 0.86
    /// (PhotoCraft's black at alpha 140 and white at alpha 220).
    pub width_ring_dark: Color32,
    #[allow(missing_docs)]
    pub width_ring_light: Color32,
    /// The width circle's smallest radius: 2.
    pub width_ring_min: f32,
    /// How far the dark ring lies outside the light one: 0.5.
    pub width_ring_gap: f32,
}

impl SelectToolStyle {
    /// The selection tools' look in `tokens`.
    pub fn of(tokens: &UiTokens) -> SelectToolStyle {
        let (stroke, o) = (&tokens.stroke, &tokens.color.overlay);
        SelectToolStyle {
            fastening: FASTENING_SIZE,
            fastening_fill: o.handle_fill.to_egui(),
            fastening_stroke: o.handle_stroke.to_egui(),
            ring_radius: tokens.shape.radius.sm,
            ring_offset: RING_OFFSET,
            ring_halo: stroke.medium + 2.0 * stroke.thin,
            ring_halo_color: o.halo.faded(RING_HALO_ALPHA).to_egui(),
            ring_line: stroke.thin,
            ring_line_color: o.handle_fill.to_egui(),
            width_ring_dark: o.halo.faded(WIDTH_RING_DARK_ALPHA).to_egui(),
            width_ring_light: o.handle_fill.faded(WIDTH_RING_LIGHT_ALPHA).to_egui(),
            width_ring_min: WIDTH_RING_MIN,
            width_ring_gap: WIDTH_RING_GAP,
        }
    }
}
