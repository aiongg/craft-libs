//! Colour picker: the sizes of the area, strip, compare swatch, marker, pointer and swatch pair,
//! and the area marker's two colours. The sizes scale with the control height, and no theme
//! overrides them. The marker is achromatic (PhotoCraft's white inside black), so it doesn't tint
//! the colour it sits on; its two rings contrast with each other, so one of them reads on any
//! colour. The picker's other colours come from the line and text tokens.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::TokenRef;
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
    /// Side of a swatch pair's well: `color_picker.pair_well`.
    pub pair_well: f32,
    /// Offset of the back well from the front one, on both axes: `color_picker.pair_step`.
    pub pair_step: f32,
    /// Width of the gap around the front well: `color_picker.pair_gap`.
    pub pair_gap: f32,
    /// Width of a ring well's band: `color_picker.pair_ring`.
    pub pair_ring: f32,
    /// Side of the Defaults and Swap icon boxes over the chips: `color_picker.pair_icon`.
    pub pair_icon: f32,
    /// Between those icon boxes and the chips: `color_picker.pair_icon_gap`.
    pub pair_icon_gap: f32,
    /// Width of the colour readout's hex field: `color_picker.readout_hex_width`.
    pub readout_hex_width: f32,
    /// Width of the colour readout's R, G and B fields: `color_picker.readout_channel_width`.
    pub readout_channel_width: f32,
    /// The area marker's inner circle: `color_picker.marker`, white.
    pub marker: Rgba,
    /// The marker's outer ring: `color_picker.marker_halo`, black at alpha 160.
    pub marker_halo: Rgba,
}

/// An overridable colour picker field: the marker's colours. Paths: `marker`, `marker_halo`.
/// The sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorPickerField {
    /// `color_picker.marker`
    Marker,
    /// `color_picker.marker_halo`
    MarkerHalo,
}

impl Family for ColorPickerTokens {
    type Field = ColorPickerField;
    const FIELDS: &'static [ColorPickerField] = &[ColorPickerField::Marker, ColorPickerField::MarkerHalo];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ColorPickerTokens;
        "area" 0.0..=1024.0 => area;
        "area_height" 0.0..=1024.0 => area_height;
        "strip_full" 0.0..=1024.0 => strip_full;
        "strip" 0.0..=1024.0 => strip;
        "compare_width" 0.0..=1024.0 => compare_width;
        "compare_height" 0.0..=1024.0 => compare_height;
        "marker_inner_radius" 0.0..=1024.0 => marker_inner_radius;
        "marker_inner_width" 0.0..=16.0 => marker_inner_width;
        "marker_outer_radius" 0.0..=1024.0 => marker_outer_radius;
        "marker_outer_width" 0.0..=16.0 => marker_outer_width;
        "pointer_width" 0.0..=1024.0 => pointer_width;
        "pointer_height" 0.0..=1024.0 => pointer_height;
        "warning_icon" 0.0..=1024.0 => warning_icon;
        "warning_chip" 0.0..=1024.0 => warning_chip;
        "pair_well" 0.0..=1024.0 => pair_well;
        "pair_step" 0.0..=1024.0 => pair_step;
        "pair_gap" 0.0..=64.0 => pair_gap;
        "pair_ring" 0.0..=16.0 => pair_ring;
        "pair_icon" 0.0..=64.0 => pair_icon;
        "pair_icon_gap" 0.0..=64.0 => pair_icon_gap;
        "readout_hex_width" 0.0..=1024.0 => readout_hex_width;
        "readout_channel_width" 0.0..=1024.0 => readout_channel_width;
    };

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
            pair_well: scale(values::COLOR_PICKER_PAIR_WELL),
            pair_step: scale(values::COLOR_PICKER_PAIR_STEP),
            pair_gap: fine(values::COLOR_PICKER_PAIR_GAP),
            pair_ring: scale(values::COLOR_PICKER_PAIR_RING),
            pair_icon: values::COLOR_PICKER_PAIR_ICON,
            pair_icon_gap: values::COLOR_PICKER_PAIR_ICON_GAP,
            readout_hex_width: scale(values::COLOR_PICKER_READOUT_HEX_WIDTH),
            readout_channel_width: scale(values::COLOR_PICKER_READOUT_CHANNEL_WIDTH),
            marker: values::COLOR_PICKER_MARKER,
            marker_halo: values::COLOR_PICKER_MARKER_HALO,
        }
    }

    fn path(f: ColorPickerField) -> String {
        match f {
            ColorPickerField::Marker => "marker",
            ColorPickerField::MarkerHalo => "marker_halo",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<ColorPickerField> {
        Self::FIELDS.iter().copied().find(|&f| Self::path(f) == path)
    }

    fn kind(_: ColorPickerField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: ColorPickerField) -> OverrideValue {
        OverrideValue::Color(match f {
            ColorPickerField::Marker => self.marker,
            ColorPickerField::MarkerHalo => self.marker_halo,
        })
    }

    fn set(&mut self, f: ColorPickerField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            match f {
                ColorPickerField::Marker => self.marker = c,
                ColorPickerField::MarkerHalo => self.marker_halo = c,
            }
        }
    }

    /// The marker is a state cue drawn on any colour: its circle against its ring, 3:1 (4.5:1
    /// with high contrast).
    fn pairs() -> Vec<ContrastPair> {
        let field = |f| ComponentField::ColorPicker(f);
        vec![ContrastPair {
            fg: TokenRef::Component(field(ColorPickerField::Marker)),
            bg: TokenRef::Component(field(ColorPickerField::MarkerHalo)),
            minimum: values::NON_TEXT_MIN,
            alternate: None,
        }]
    }
}
