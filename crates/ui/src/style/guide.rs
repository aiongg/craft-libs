//! Style of the `guide` family.

use egui::{Color32, Stroke, Vec2};

use crate::primitives::FocusRing;
use crate::theme::{ToEgui as _, TypeRole, UiTokens};
use craft_theme::{Rgba, contrast_ratio};

const DOT_RADIUS: f32 = 0.6;
const MEASURE_CAP: f32 = 4.0;
/// The colour of the selected guide.
const SELECTED_GUIDE: Rgba = Rgba::rgb(0, 0, 255);
/// The colour of smart guides and their distances.
const SMART_GUIDE: Rgba = Rgba::rgb(255, 0, 255);

/// The guides' look, from [`Guides::style`](crate::canvas::Guides::style); smart guides share it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideStyle {
    /// Line width: `stroke.thin`.
    pub width: f32,
    /// Distance from a guide within which the pointer takes it: `size.hit.slop`.
    pub hit: f32,
    /// A dashed guide's dashes and gaps: `stroke.dash`, `stroke.dash_gap`.
    pub dash: f32,
    /// A dashed guide's gap length, in screen points.
    pub gap: f32,
    /// A dotted guide's dot spacing: `stroke.dash_gap`.
    pub dot_gap: f32,
    /// A dotted guide's dot radius, in screen points.
    pub dot_radius: f32,
    /// `overlay.guide`.
    pub color: Color32,
    /// The selected guide: blue.
    pub selected: Color32,
    /// Smart guides and distances: magenta.
    pub smart: Color32,
    /// A distance's end caps.
    pub measure_cap: f32,
    /// Around a distance's label: `space.pad.control` and `space.gap.tight`.
    pub measure_pad: Vec2,
    /// A distance label's corners: `shape.radius.control`.
    pub measure_radius: egui::CornerRadius,
    /// A distance label's type role: `caption`.
    pub measure_role: TypeRole,
    /// A distance label's text: black or white, whichever contrasts more with `smart`.
    pub measure_text: Color32,
    /// The lines beside the focused guide: the focus ring's stroke.
    pub focus: Stroke,
    /// Under each focus line, so it reads on any image: `overlay.halo`, `stroke.thin` wider than
    /// the line on each side.
    pub focus_halo: Stroke,
    /// From the guide's edge to the focus lines: the focus ring's offset.
    pub focus_offset: f32,
    /// Width of Guide Layout's Number field.
    pub number_width: f32,
    /// Width of its Width and Height fields.
    pub size_width: f32,
    /// Width of the field in each of its Margin sides.
    pub margin_width: f32,
    /// Width of New Guide's Position field.
    pub position_width: f32,
    /// Width of New Guide's Color list.
    pub color_width: f32,
    /// Width of Guide Layout's Preset and Target lists.
    pub layout_list_width: f32,
    /// Width of Guide Layout's Color list.
    pub layout_color_width: f32,
}

impl GuideStyle {
    /// The guide style for `tokens`.
    pub fn new(tokens: &UiTokens) -> Self {
        let control = tokens.size.control.md;
        let o = &tokens.color.overlay;
        let ring = FocusRing::style(tokens);
        let smart = SMART_GUIDE.to_egui();
        Self {
            width: tokens.stroke.thin,
            hit: tokens.size.hit.slop,
            dash: tokens.stroke.dash,
            gap: tokens.stroke.dash_gap,
            dot_gap: tokens.stroke.dash_gap,
            dot_radius: DOT_RADIUS,
            color: o.guide.to_egui(),
            selected: SELECTED_GUIDE.to_egui(),
            smart,
            measure_cap: MEASURE_CAP,
            measure_pad: egui::vec2(tokens.space.pad.control, tokens.space.gap.tight),
            measure_radius: tokens.radius.control,
            measure_role: TypeRole::Caption,
            measure_text: readable_on(smart),
            focus: ring.stroke,
            focus_halo: Stroke::new(ring.stroke.width + 2.0 * tokens.stroke.thin, o.halo.to_egui()),
            focus_offset: ring.offset,
            number_width: control * 2.25,
            size_width: control * 2.0,
            margin_width: control * 2.25,
            position_width: control * 4.0,
            color_width: control * 4.5,
            layout_list_width: control * 7.5,
            layout_color_width: control * 5.75,
        }
    }
}

/// Black or white text, whichever contrasts more with `fill`.
pub(crate) fn readable_on(fill: Color32) -> Color32 {
    let fill = Rgba::from(fill);
    if contrast_ratio(Rgba::BLACK, fill) >= contrast_ratio(Rgba::WHITE, fill) { Color32::BLACK } else { Color32::WHITE }
}
