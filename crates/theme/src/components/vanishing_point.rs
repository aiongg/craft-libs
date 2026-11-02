//! Vanishing Point's marks: plane outlines, grid, nodes, the stamp ring's halo. The same at every Size (they sit on the image). The plane colours are the semantic `accent`, `status.warn.fg` and `status.bad.fg`, checked against the overridable halo.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken as T, TokenRef};
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
    /// How far from a node's centre a press reaches, in points: `vanishing_point.press_reach`.
    pub press_reach: f32,
    /// The click-versus-drag distance of a torn plane's far edge node, in document pixels: `vanishing_point.click_slop`.
    pub click_slop: f32,
    /// The white line of the stamp ring and the source cross: `vanishing_point.ring_line`.
    pub ring_line: f32,
    /// The alpha of the pasted image's white tint, 0 to 1: `vanishing_point.paste_alpha`.
    pub paste_alpha: f32,
    /// The halo under lines, the ring and the cross: `vanishing_point.halo`. Black at alpha 160 by default: it darkens whatever the image shows.
    pub halo: Rgba,
}

/// The overridable Vanishing Point field. Path: `halo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VanishingPointField {
    /// `vanishing_point.halo`
    Halo,
}

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
        "press_reach" 1.0..=64.0 => press_reach;
        "click_slop" 0.0..=64.0 => click_slop;
        "ring_line" 0.0..=16.0 => ring_line;
        "paste_alpha" 0.0..=1.0 => paste_alpha;
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
            press_reach: values::VANISHING_POINT_PRESS_REACH,
            click_slop: values::VANISHING_POINT_CLICK_SLOP,
            ring_line: values::VANISHING_POINT_RING_LINE,
            paste_alpha: values::VANISHING_POINT_PASTE_ALPHA,
            halo: values::VANISHING_POINT_HALO,
        }
    }

    fn path(f: VanishingPointField) -> String {
        match f {
            VanishingPointField::Halo => "halo".to_owned(),
        }
    }

    fn from_path(path: &str) -> Option<VanishingPointField> {
        (path == "halo").then_some(VanishingPointField::Halo)
    }

    fn kind(f: VanishingPointField) -> FieldKind {
        match f {
            VanishingPointField::Halo => FieldKind::Color,
        }
    }

    fn get(&self, f: VanishingPointField) -> OverrideValue {
        match f {
            VanishingPointField::Halo => OverrideValue::Color(self.halo),
        }
    }

    fn set(&mut self, f: VanishingPointField, v: OverrideValue) {
        if let (VanishingPointField::Halo, OverrideValue::Color(c)) = (f, v) {
            self.halo = c;
        }
    }

    /// The plane colours are drawn over the halo (a translucent halo is measured over
    /// `surface.panel`): each is a canvas mark and needs 3:1 against it (4.5:1 with high contrast).
    fn pairs() -> Vec<ContrastPair> {
        [T::AccentSolid, T::StatusWarnFg, T::StatusBadFg]
            .into_iter()
            .map(|fg| ContrastPair {
                fg: TokenRef::Color(fg),
                bg: TokenRef::Component(ComponentField::VanishingPoint(VanishingPointField::Halo)),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
