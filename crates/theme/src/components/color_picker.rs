//! Colour picker: the sizes of the area, strip, compare swatch, marker and pointer. Sizes only (no
//! colours: the picker's colours come from the line, overlay and text tokens): they scale with
//! the control height, and no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Colour picker tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorPickerTokens {
    /// Side of the Full picker's colour area: `color_picker.area`.
    pub area: f32,
    /// Height of the Compact picker's colour area when the host gives none: `color_picker.area_height`.
    pub area_height: f32,
    /// Width of the component strip in the Full picker: `color_picker.strip_full`.
    pub strip_full: f32,
    /// Width of the component strip in the Compact picker: `color_picker.strip`.
    pub strip: f32,
    /// Width of the compare swatch: `color_picker.compare_width`.
    pub compare_width: f32,
    /// Height of the compare swatch: `color_picker.compare_height`.
    pub compare_height: f32,
    /// Radius of the area marker's inner circle: `color_picker.marker_inner_radius`.
    pub marker_inner_radius: f32,
    /// Stroke width of the marker's inner circle: `color_picker.marker_inner_width`.
    pub marker_inner_width: f32,
    /// Radius of the marker's outer ring: `color_picker.marker_outer_radius`.
    pub marker_outer_radius: f32,
    /// Stroke width of the marker's outer ring: `color_picker.marker_outer_width`.
    pub marker_outer_width: f32,
    /// Width of a strip pointer triangle: `color_picker.pointer_width`.
    pub pointer_width: f32,
    /// Height of a strip pointer triangle: `color_picker.pointer_height`.
    pub pointer_height: f32,
    /// Size of a warning icon: `color_picker.warning_icon`.
    pub warning_icon: f32,
    /// Size of a warning's chip of the nearest passing colour: `color_picker.warning_chip`.
    pub warning_chip: f32,
}

/// The colour picker has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorPickerField {}

impl Family for ColorPickerTokens {
    type Field = ColorPickerField;
    const FIELDS: &'static [ColorPickerField] = &[];

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size. Sizes of
    /// strokes and shapes round to half points, the rest to whole points.
    fn defaults(s: &Semantic<'_>) -> ColorPickerTokens {
        let k = s.size.control.md / values::COLOR_PICKER_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let fine = |w: f32| (w * k * 2.0).round() / 2.0;
        ColorPickerTokens {
            area: scale(values::COLOR_PICKER_AREA),
            area_height: scale(values::COLOR_PICKER_AREA_HEIGHT),
            strip_full: scale(values::COLOR_PICKER_STRIP_FULL),
            strip: scale(values::COLOR_PICKER_STRIP),
            compare_width: scale(values::COLOR_PICKER_COMPARE_WIDTH),
            compare_height: scale(values::COLOR_PICKER_COMPARE_HEIGHT),
            marker_inner_radius: fine(values::COLOR_PICKER_MARKER_INNER_RADIUS),
            marker_inner_width: fine(values::COLOR_PICKER_MARKER_INNER_WIDTH),
            marker_outer_radius: fine(values::COLOR_PICKER_MARKER_OUTER_RADIUS),
            marker_outer_width: fine(values::COLOR_PICKER_MARKER_OUTER_WIDTH),
            pointer_width: fine(values::COLOR_PICKER_POINTER_WIDTH),
            pointer_height: fine(values::COLOR_PICKER_POINTER_HEIGHT),
            warning_icon: scale(values::COLOR_PICKER_WARNING_ICON),
            warning_chip: scale(values::COLOR_PICKER_WARNING_CHIP),
        }
    }

    fn path(f: ColorPickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ColorPickerField> {
        None
    }

    fn kind(f: ColorPickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ColorPickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ColorPickerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
