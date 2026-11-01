//! Guides: ruler guides and smart guides on the canvas. Their colours are the semantic
//! `overlay.guide`, `overlay.guide_selected` and `overlay.smart_guide`; the family holds their
//! geometry, the same at every Size because guides sit on the document, and the widths of the
//! fields and lists of the guide dialogs (PhotoCraft's).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Guide tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideTokens {
    /// Line width, in points: `guide.width`.
    pub width: f32,
    /// Distance from a guide within which the pointer takes it: `guide.hit`.
    pub hit: f32,
    /// Dash length of a dashed guide and of a smart guide's extension: `guide.dash`.
    pub dash: f32,
    /// Gap between the dashes: `guide.gap`.
    pub gap: f32,
    /// Gap between the dots of a dotted guide: `guide.dot_gap`.
    pub dot_gap: f32,
    /// Radius of a dotted guide's dots: `guide.dot_radius`.
    pub dot_radius: f32,
    /// Length of the end caps of a smart guide's distance: `guide.measure_cap`.
    pub measure_cap: f32,
    /// Padding beside the text of a distance's label: `guide.measure_pad_x`.
    pub measure_pad_x: f32,
    /// Padding above and below it: `guide.measure_pad_y`.
    pub measure_pad_y: f32,
    /// Width of Guide Layout's Number field: `guide.number_width`.
    pub number_width: f32,
    /// Width of its Width and Height fields (Size): `guide.size_width`.
    pub size_width: f32,
    /// Width of the field in each of its Margin sides: `guide.margin_width`.
    pub margin_width: f32,
    /// Width of New Guide's Position field: `guide.position_width`.
    pub position_width: f32,
    /// Width of New Guide's Color list: `guide.color_width`.
    pub color_width: f32,
    /// Width of Guide Layout's Preset and Target lists: `guide.layout_list_width`.
    pub layout_list_width: f32,
    /// Width of Guide Layout's Color list: `guide.layout_color_width`.
    pub layout_color_width: f32,
}

/// Guides have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GuideField {}

impl Family for GuideTokens {
    type Field = GuideField;
    const FIELDS: &'static [GuideField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { GuideTokens;
        "width" 0.0..=1024.0 => width;
        "hit" 0.0..=64.0 => hit;
        "dash" 0.0..=16.0 => dash;
        "gap" 0.0..=64.0 => gap;
        "dot_gap" 0.0..=64.0 => dot_gap;
        "dot_radius" 0.0..=1024.0 => dot_radius;
        "measure_cap" 0.0..=1024.0 => measure_cap;
        "measure_pad_x" 0.0..=64.0 => measure_pad_x;
        "measure_pad_y" 0.0..=64.0 => measure_pad_y;
        "number_width" 0.0..=1024.0 => number_width;
        "size_width" 0.0..=1024.0 => size_width;
        "margin_width" 0.0..=1024.0 => margin_width;
        "position_width" 0.0..=1024.0 => position_width;
        "color_width" 0.0..=1024.0 => color_width;
        "layout_list_width" 0.0..=1024.0 => layout_list_width;
        "layout_color_width" 0.0..=1024.0 => layout_color_width;
    };

    fn defaults(_: &Semantic<'_>) -> GuideTokens {
        GuideTokens {
            width: values::GUIDE_WIDTH,
            hit: values::GUIDE_HIT,
            dash: values::GUIDE_DASH,
            gap: values::GUIDE_GAP,
            dot_gap: values::GUIDE_DOT_GAP,
            dot_radius: values::GUIDE_DOT_RADIUS,
            measure_cap: values::GUIDE_MEASURE_CAP,
            measure_pad_x: values::GUIDE_MEASURE_PAD.0,
            measure_pad_y: values::GUIDE_MEASURE_PAD.1,
            number_width: values::GUIDE_NUMBER_WIDTH,
            size_width: values::GUIDE_SIZE_WIDTH,
            margin_width: values::GUIDE_MARGIN_WIDTH,
            position_width: values::GUIDE_POSITION_WIDTH,
            color_width: values::GUIDE_COLOR_WIDTH,
            layout_list_width: values::GUIDE_LAYOUT_LIST_WIDTH,
            layout_color_width: values::GUIDE_LAYOUT_COLOR_WIDTH,
        }
    }

    fn path(f: GuideField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GuideField> {
        None
    }

    fn kind(f: GuideField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GuideField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GuideField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
