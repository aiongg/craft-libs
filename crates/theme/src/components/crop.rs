//! Crop frame: the frame line, corner brackets, edge bars, composition guides and shield. Colours are the semantic `overlay.*` tokens; the family holds sizes and
//! shares, the same at every Size because the overlay sits on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Crop tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropTokens {
    /// Reach of the frame's edges and corners: `crop.hit`.
    pub hit: f32,
    /// Frame line width: `crop.line`.
    pub line: f32,
    /// Composition guide line width: `crop.guide_line`.
    pub guide_line: f32,
    /// Corner bracket and edge bar thickness: `crop.bracket`.
    pub bracket: f32,
    /// Corner bracket arm length: `crop.bracket_arm`.
    pub bracket_arm: f32,
    /// Divisor of the side giving an edge bar's length: `crop.bar_share`.
    pub bar_share: f32,
    /// Default shield opacity: `crop.shield_opacity`.
    pub shield_opacity: f32,
    /// Share of the opacity kept during a gesture: `crop.shield_editing`.
    pub shield_editing: f32,
    /// Target cell side of the Grid guide: `crop.grid_step`.
    pub grid_step: f32,
    /// Most Grid guide lines per axis: `crop.grid_max`.
    pub grid_max: f32,
    /// Shortest Straighten line: `crop.straighten_min`.
    pub straighten_min: f32,
    /// Straighten line width: `crop.straighten_line`.
    pub straighten_line: f32,
    /// Width of the halo under the Straighten line: `crop.straighten_halo`.
    pub straighten_halo: f32,
    /// Side of a perspective crop's corner square: `crop.perspective_corner`.
    pub perspective_corner: f32,
    /// Edge width of a perspective crop's corner square and placed corner dot: `crop.perspective_corner_edge`.
    pub perspective_corner_edge: f32,
    /// Width of a perspective crop's outline: `crop.perspective_outline`.
    pub perspective_outline: f32,
    /// Width of a perspective crop's grid line: `crop.perspective_grid_width`.
    pub perspective_grid_width: f32,
    /// Screen points of the longest side per perspective grid step: `crop.perspective_grid_step`.
    pub perspective_grid_step: f32,
    /// Radius of a placed perspective corner click: `crop.perspective_click_dot`.
    pub perspective_click_dot: f32,
    /// Movement under which a perspective press is a click, at the default reach: `crop.perspective_click_move`.
    pub perspective_click_move: f32,
    /// Default reach of a perspective corner: `crop.perspective_reach`.
    pub perspective_reach: f32,
}

/// Crop has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CropField {}

impl Family for CropTokens {
    type Field = CropField;
    const FIELDS: &'static [CropField] = &[];

    fn defaults(_: &Semantic<'_>) -> CropTokens {
        CropTokens {
            hit: values::CROP_HIT,
            line: values::CROP_LINE,
            guide_line: values::CROP_GUIDE_LINE,
            bracket: values::CROP_BRACKET,
            bracket_arm: values::CROP_BRACKET_ARM,
            bar_share: values::CROP_BAR_SHARE,
            shield_opacity: values::CROP_SHIELD_OPACITY,
            shield_editing: values::CROP_SHIELD_EDITING,
            grid_step: values::CROP_GRID_STEP,
            grid_max: values::CROP_GRID_MAX,
            straighten_min: values::CROP_STRAIGHTEN_MIN,
            straighten_line: values::CROP_STRAIGHTEN_LINE,
            straighten_halo: values::CROP_STRAIGHTEN_HALO,
            perspective_corner: values::CROP_PERSPECTIVE_CORNER,
            perspective_corner_edge: values::CROP_PERSPECTIVE_CORNER_EDGE,
            perspective_outline: values::CROP_PERSPECTIVE_OUTLINE,
            perspective_grid_width: values::CROP_PERSPECTIVE_GRID_WIDTH,
            perspective_grid_step: values::CROP_PERSPECTIVE_GRID_STEP,
            perspective_click_dot: values::CROP_PERSPECTIVE_CLICK_DOT,
            perspective_click_move: values::CROP_PERSPECTIVE_CLICK_MOVE,
            perspective_reach: values::CROP_PERSPECTIVE_REACH,
        }
    }

    fn path(f: CropField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CropField> {
        None
    }

    fn kind(f: CropField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CropField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CropField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
