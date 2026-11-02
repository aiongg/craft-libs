//! The Layer Style dialog: the effect list and page columns, the Styles page's thumbnails and the contour field. The sizes are the same at every Size; the contour thumbnail's border is overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// The Layer Style dialog: the effect list and page columns, the Styles page's thumbnails and the contour field. The sizes are the same at every Size; the contour thumbnail's border is overridable.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerStyleTokens {
    /// The effect list's width: `layer_style.list_width`.
    pub list_width: f32,
    /// The page column's width: `layer_style.page_width`.
    pub page_width: f32,
    /// A Styles page thumbnail's side: `layer_style.preset_thumb`.
    pub preset_thumb: f32,
    /// The Styles page's scroll height: `layer_style.presets_height`.
    pub presets_height: f32,
    /// The contour field's thumbnail side: `layer_style.contour_thumb`.
    pub contour_thumb: f32,
    /// The contour field's picker arrow width: `layer_style.contour_arrow`.
    pub contour_arrow: f32,
    /// A cell of the contour preset picker: `layer_style.contour_cell`.
    pub contour_cell: f32,
    /// The Contour Editor's graph side: `layer_style.contour_graph`.
    pub contour_graph: f32,
    /// The contour thumbnail's 1 pt border: `line.control_edge`, the field edge: `layer_style.contour_border`.
    pub contour_border: Rgba,
}

/// An overridable Layer Style field. Path: `contour_border`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LayerStyleField {
    /// `layer_style.contour_border`
    ContourBorder,
}

impl Family for LayerStyleTokens {
    type Field = LayerStyleField;
    const FIELDS: &'static [LayerStyleField] = &[LayerStyleField::ContourBorder];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { LayerStyleTokens;
        "list_width" 0.0..=1024.0 => list_width;
        "page_width" 0.0..=2048.0 => page_width;
        "preset_thumb" 0.0..=256.0 => preset_thumb;
        "presets_height" 0.0..=2048.0 => presets_height;
        "contour_thumb" 0.0..=256.0 => contour_thumb;
        "contour_arrow" 0.0..=128.0 => contour_arrow;
        "contour_cell" 0.0..=256.0 => contour_cell;
        "contour_graph" 0.0..=1024.0 => contour_graph;
    };

    fn defaults(s: &Semantic<'_>) -> LayerStyleTokens {
        LayerStyleTokens {
            list_width: values::LAYER_STYLE_LIST_WIDTH,
            page_width: values::LAYER_STYLE_PAGE_WIDTH,
            preset_thumb: values::LAYER_STYLE_PRESET_THUMB,
            presets_height: values::LAYER_STYLE_PRESETS_HEIGHT,
            contour_thumb: values::LAYER_STYLE_CONTOUR_THUMB,
            contour_arrow: values::LAYER_STYLE_CONTOUR_ARROW,
            contour_cell: values::LAYER_STYLE_CONTOUR_CELL,
            contour_graph: values::LAYER_STYLE_CONTOUR_GRAPH,
            contour_border: s.color.line.control_edge,
        }
    }

    fn path(_: LayerStyleField) -> String {
        "contour_border".to_owned()
    }

    fn from_path(path: &str) -> Option<LayerStyleField> {
        (path == "contour_border").then_some(LayerStyleField::ContourBorder)
    }

    fn kind(_: LayerStyleField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: LayerStyleField) -> OverrideValue {
        OverrideValue::Color(self.contour_border)
    }

    fn set(&mut self, _: LayerStyleField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.contour_border = c;
        }
    }

    /// The thumbnail's border bounds it on the panel and on a raised surface (a dialog), as a
    /// field's rest border does.
    fn pairs() -> Vec<ContrastPair> {
        let edge = TokenRef::Component(ComponentField::LayerStyle(LayerStyleField::ContourBorder));
        [ColorToken::SurfacePanel, ColorToken::SurfaceRaised]
            .map(|bg| ContrastPair { fg: edge, bg: TokenRef::Color(bg), minimum: values::CONTROL_EDGE_MIN, alternate: None })
            .to_vec()
    }
}
