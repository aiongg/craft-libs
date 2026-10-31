//! Colour wheels and discs: the hue ring round a saturation and brightness triangle, and the hue and
//! saturation disc of a harmony. The sizes scale with the control height, and the spoke of the
//! disc is overridable. Everything else draws in semantic tokens and the colour picker's marker.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Colour wheel tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorWheelTokens {
    /// Default height of the wheel's area: `color_wheel.side`.
    pub side: f32,
    /// Least height of the area: `color_wheel.side_min`.
    pub side_min: f32,
    /// Least width of the hue ring: `color_wheel.ring_min`.
    pub ring_min: f32,
    /// Greatest width of the hue ring: `color_wheel.ring_max`.
    pub ring_max: f32,
    /// Ring width as a proportion of the outer radius: `color_wheel.ring_ratio`.
    pub ring_ratio: f32,
    /// Distance from the area's edge to the ring's outer edge: `color_wheel.outer_inset`.
    pub outer_inset: f32,
    /// Distance from the ring's inner edge to the triangle's circle: `color_wheel.triangle_inset`.
    pub triangle_inset: f32,
    /// Distance outside the ring within which a press still takes it: `color_wheel.grab_outer`.
    pub grab_outer: f32,
    /// Distance inside the ring from which a press takes it: `color_wheel.grab_inner`.
    pub grab_inner: f32,
    /// Distance from a colour disc's area edge to the disc: `color_wheel.disc_inset`.
    pub disc_inset: f32,
    /// Radius of a colour disc's marker: `color_wheel.disc_marker`.
    pub disc_marker: f32,
    /// Radius of a colour disc's selected marker: `color_wheel.disc_marker_selected`.
    pub disc_marker_selected: f32,
    /// Distance from a press within which a colour disc grabs a marker: `color_wheel.disc_grab`.
    pub disc_grab: f32,
    /// Least brightness a colour disc draws: `color_wheel.disc_floor`.
    pub disc_floor: f32,
    /// The line from a colour disc's centre to a marker: `color_wheel.spoke`, white at alpha 160.
    pub spoke: Rgba,
}

/// An overridable colour wheel field. Path: `spoke`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorWheelField {
    /// `color_wheel.spoke`
    Spoke,
}

impl Family for ColorWheelTokens {
    type Field = ColorWheelField;
    const FIELDS: &'static [ColorWheelField] = &[ColorWheelField::Spoke];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ColorWheelTokens;
        "side" 0.0..=1024.0 => side;
        "side_min" 0.0..=1024.0 => side_min;
        "ring_min" 0.0..=1024.0 => ring_min;
        "ring_max" 0.0..=1024.0 => ring_max;
        "outer_inset" 0.0..=64.0 => outer_inset;
        "triangle_inset" 0.0..=64.0 => triangle_inset;
        "grab_outer" 0.0..=64.0 => grab_outer;
        "grab_inner" 0.0..=64.0 => grab_inner;
        "disc_inset" 0.0..=64.0 => disc_inset;
        "disc_marker" 0.0..=1024.0 => disc_marker;
        "disc_marker_selected" 0.0..=1024.0 => disc_marker_selected;
        "disc_grab" 0.0..=64.0 => disc_grab;
    };

    /// The Compact sizes (PhotoCraft's wheel, VectorCraft's harmony wheel), scaled by the control
    /// height of the size; a scaled value is rounded to whole points, the Compact values are not;
    /// proportions stay.
    fn defaults(s: &Semantic<'_>) -> ColorWheelTokens {
        let k = s.size.control.md / values::COLOR_WHEEL_BASE_CONTROL;
        let scale = |w: f32| if k == 1.0 { w } else { (w * k).round() };
        ColorWheelTokens {
            side: scale(values::COLOR_WHEEL_SIDE),
            side_min: scale(values::COLOR_WHEEL_SIDE_MIN),
            ring_min: scale(values::COLOR_WHEEL_RING_MIN),
            ring_max: scale(values::COLOR_WHEEL_RING_MAX),
            ring_ratio: values::COLOR_WHEEL_RING_RATIO,
            outer_inset: scale(values::COLOR_WHEEL_OUTER_INSET),
            triangle_inset: scale(values::COLOR_WHEEL_TRIANGLE_INSET),
            grab_outer: scale(values::COLOR_WHEEL_GRAB_OUTER),
            grab_inner: scale(values::COLOR_WHEEL_GRAB_INNER),
            disc_inset: scale(values::COLOR_WHEEL_DISC_INSET),
            disc_marker: scale(values::COLOR_WHEEL_DISC_MARKER),
            disc_marker_selected: scale(values::COLOR_WHEEL_DISC_MARKER_SELECTED),
            disc_grab: scale(values::COLOR_WHEEL_DISC_GRAB),
            disc_floor: values::COLOR_WHEEL_DISC_FLOOR,
            spoke: values::COLOR_WHEEL_SPOKE,
        }
    }

    fn path(_: ColorWheelField) -> String {
        "spoke".to_owned()
    }

    fn from_path(path: &str) -> Option<ColorWheelField> {
        (path == "spoke").then_some(ColorWheelField::Spoke)
    }

    fn kind(_: ColorWheelField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: ColorWheelField) -> OverrideValue {
        OverrideValue::Color(self.spoke)
    }

    fn set(&mut self, _: ColorWheelField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.spoke = c;
        }
    }

    // The spoke is decoration over the disc, so it has no contrast pair.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
