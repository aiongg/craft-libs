//! Style of the `slider` family.

use craft_theme::{BySize, Oklab, Rgba, States, ThumbColors, Tokens};

/// Control height the slider proportions are set for.
const BASE_CONTROL: f32 = 24.0;
/// Slider width in control heights.
const WIDTH_IN_CONTROLS: f32 = 6.0;
/// How much shorter than the control the slider is, small and standard.
const HEIGHT_INSET: (f32, f32) = (4.0, 6.0);
/// Width of a range slider's thumb at the base control height.
const RANGE_THUMB_WIDTH: f32 = 5.0;
/// Height of a range slider's thumb at the base control height.
const RANGE_THUMB_HEIGHT: f32 = 12.0;
/// Size of a slider's tick mark at the base control height.
const MARK: f32 = 4.0;
/// Thickness of a plain track.
const TRACK_HEIGHT: f32 = 3.0;
/// Thickness of a gradient track.
const GRADIENT_HEIGHT: f32 = 5.0;
/// Distance from the thumb's edge to its focus ring.
const RING_OFFSET: f32 = 3.5;
/// Distance in points within which a drag snaps to a mark.
const SNAP: f32 = 6.0;
/// How far a disabled thumb's face moves from `surface.knob` toward `surface.panel`.
const KNOB_DISABLED: f64 = 0.5;

/// Sizes and thumb colours of sliders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderStyle {
    /// Height.
    pub height: BySize<f32>,
    /// Default width.
    pub width: BySize<f32>,
    /// Width of a range slider's thumb.
    pub range_thumb_width: BySize<f32>,
    /// Height of a range slider's thumb.
    pub range_thumb_height: BySize<f32>,
    /// Thickness of a plain track.
    pub track_height: f32,
    /// Thickness of a gradient track.
    pub gradient_height: f32,
    /// Face and outline of the thumb.
    pub thumb_colors: ThumbColors,
    /// Distance from the thumb to its focus ring.
    pub ring_offset: f32,
    /// Size of a tick mark.
    pub mark: BySize<f32>,
    /// Snap distance in points.
    pub snap: f32,
}

impl SliderStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let (c, size) = (&tokens.color, &tokens.size);
        let scaled = |base: f32| BySize { sm: (base * size.control.sm / BASE_CONTROL).round(), md: (base * size.control.md / BASE_CONTROL).round() };
        let (face, edge) = (c.surface.knob, c.line.knob_edge);
        let disabled = mix(face, c.surface.panel, KNOB_DISABLED);
        SliderStyle {
            height: BySize { sm: size.control.sm - HEIGHT_INSET.0, md: size.control.md - HEIGHT_INSET.1 },
            width: BySize { sm: WIDTH_IN_CONTROLS * size.control.sm, md: WIDTH_IN_CONTROLS * size.control.md },
            range_thumb_width: scaled(RANGE_THUMB_WIDTH),
            range_thumb_height: scaled(RANGE_THUMB_HEIGHT),
            track_height: TRACK_HEIGHT,
            gradient_height: GRADIENT_HEIGHT,
            thumb_colors: ThumbColors {
                fill: States::new(face, face, face, disabled),
                border: States::new(edge, edge, edge, c.line.divider),
            },
            ring_offset: RING_OFFSET,
            mark: scaled(MARK),
            snap: SNAP,
        }
    }
}

/// Linear interpolation in OKLab, opaque.
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let (a, b) = (a.to_oklab(), b.to_oklab());
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    Oklab { l: lerp(a.l, b.l), a: lerp(a.a, b.a), b: lerp(a.b, b.b) }.to_oklch().to_rgba()
}
