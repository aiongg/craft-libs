//! Vanishing Point's marks: plane outlines, grid, nodes, the stamp ring's halo. The same at every Size (they sit on the image). The plane colours are the semantic `accent`, `status.warn.fg` and `status.bad.fg`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Vanishing Point's marks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct VanishingPointTokens {
    /// A plane's outline width: `vanishing_point.line`.
    pub line: f32,
    /// The selected plane's outline width: `vanishing_point.line_selected`.
    pub line_selected: f32,
    /// A grid line's width: `vanishing_point.grid`.
    pub grid: f32,
    /// A grid line's alpha, 0 to 1: `vanishing_point.grid_alpha`.
    pub grid_alpha: f32,
    /// A node square's side: `vanishing_point.node`.
    pub node: f32,
    /// Create Plane's pending corner square: `vanishing_point.pending`.
    pub pending: f32,
    /// The extra width of the halo under lines, the ring and the cross: `vanishing_point.halo_width`.
    pub halo_width: f32,
    /// An arm of the source cross: `vanishing_point.source_arm`.
    pub source_arm: f32,
    /// The halo under lines, the ring and the cross: `vanishing_point.halo`. Black at alpha 160 in every theme: it must darken whatever the image shows.
    pub halo: Rgba,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VanishingPointField {}

impl Family for VanishingPointTokens {
    type Field = VanishingPointField;
    const FIELDS: &'static [VanishingPointField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { VanishingPointTokens;
        "line" 0.0..=16.0 => line;
        "line_selected" 0.0..=16.0 => line_selected;
        "grid" 0.0..=16.0 => grid;
        "grid_alpha" 0.0..=1.0 => grid_alpha;
        "node" 1.0..=64.0 => node;
        "pending" 1.0..=64.0 => pending;
        "halo_width" 0.0..=16.0 => halo_width;
        "source_arm" 1.0..=64.0 => source_arm;
    };

    fn defaults(_: &Semantic<'_>) -> VanishingPointTokens {
        VanishingPointTokens {
            line: values::VANISHING_POINT_LINE,
            line_selected: values::VANISHING_POINT_LINE_SELECTED,
            grid: values::VANISHING_POINT_GRID,
            grid_alpha: values::VANISHING_POINT_GRID_ALPHA,
            node: values::VANISHING_POINT_NODE,
            pending: values::VANISHING_POINT_PENDING,
            halo_width: values::VANISHING_POINT_HALO_WIDTH,
            source_arm: values::VANISHING_POINT_SOURCE_ARM,
            halo: values::VANISHING_POINT_HALO,
        }
    }

    fn path(f: VanishingPointField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<VanishingPointField> {
        None
    }

    fn kind(f: VanishingPointField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: VanishingPointField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: VanishingPointField, _: OverrideValue) {
        match f {}
    }

    // The halo is pinned black, not a theme colour; the plane colours are semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
