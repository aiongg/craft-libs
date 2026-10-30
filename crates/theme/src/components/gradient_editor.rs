//! Gradient editors: the stop bar, its markers and midpoint diamonds. Sizes scale with the Size
//! preset; the colours are overridable. Stop colours, the opacity markers' greys and the ramp are
//! content and have no token.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken as T, TokenRef};
use crate::values;

/// Gradient editor tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientEditorTokens {
    /// Height of the ramp: `gradient_editor.ramp_height`.
    pub ramp_height: f32,
    /// Inset of the ramp at both ends of the bar: `gradient_editor.inset`.
    pub inset: f32,
    /// Side of a stop marker's square: `gradient_editor.marker`.
    pub marker: f32,
    /// Height of a stop marker's pointer triangle: `gradient_editor.pointer`.
    pub pointer: f32,
    /// Radius of a midpoint diamond: `gradient_editor.diamond`.
    pub diamond: f32,
    /// Distance from a stop marker that takes a press: `gradient_editor.hit`.
    pub hit: f32,
    /// Distance from a midpoint diamond that takes a press: `gradient_editor.diamond_hit`.
    pub diamond_hit: f32,
    /// How far above or below the ramp a dragged stop is removed: `gradient_editor.tear_off`.
    pub tear_off: f32,
    /// Thickness of the line that marks a drop place: `gradient_editor.drop_line`.
    pub drop_line: f32,
    /// The 1 pt edge round the ramp: `line.control_edge`.
    pub edge: Rgba,
    /// Marker outlines, pointers and diamonds: `text.muted`.
    pub marker_edge: Rgba,
    /// The selected marker's pointer, outline and diamond: `accent.solid`.
    pub selected: Rgba,
}

/// An overridable gradient editor field. Paths: `edge`, `marker_edge`, `selected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GradientEditorField {
    /// `gradient_editor.edge`
    Edge,
    /// `gradient_editor.marker_edge`
    MarkerEdge,
    /// `gradient_editor.selected`
    Selected,
}

impl GradientEditorField {
    const ALL: [GradientEditorField; 3] = [GradientEditorField::Edge, GradientEditorField::MarkerEdge, GradientEditorField::Selected];

    const fn name(self) -> &'static str {
        match self {
            GradientEditorField::Edge => "edge",
            GradientEditorField::MarkerEdge => "marker_edge",
            GradientEditorField::Selected => "selected",
        }
    }
}

impl GradientEditorTokens {
    fn color(&self, f: GradientEditorField) -> Rgba {
        match f {
            GradientEditorField::Edge => self.edge,
            GradientEditorField::MarkerEdge => self.marker_edge,
            GradientEditorField::Selected => self.selected,
        }
    }

    fn color_mut(&mut self, f: GradientEditorField) -> &mut Rgba {
        match f {
            GradientEditorField::Edge => &mut self.edge,
            GradientEditorField::MarkerEdge => &mut self.marker_edge,
            GradientEditorField::Selected => &mut self.selected,
        }
    }
}

impl Family for GradientEditorTokens {
    type Field = GradientEditorField;
    const FIELDS: &'static [GradientEditorField] = &GradientEditorField::ALL;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { GradientEditorTokens;
        "ramp_height" 0.0..=1024.0 => ramp_height;
        "inset" 0.0..=64.0 => inset;
        "marker" 0.0..=1024.0 => marker;
        "pointer" 0.0..=1024.0 => pointer;
        "diamond" 0.0..=1024.0 => diamond;
        "hit" 0.0..=64.0 => hit;
        "diamond_hit" 0.0..=64.0 => diamond_hit;
        "tear_off" 0.0..=64.0 => tear_off;
        "drop_line" 0.0..=16.0 => drop_line;
    };

    /// The Compact sizes (PhotoCraft's Gradient Editor), scaled by the control height of the size
    /// and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> GradientEditorTokens {
        let k = s.size.control.md / values::GRADIENT_EDITOR_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let c = s.color;
        GradientEditorTokens {
            ramp_height: scale(values::GRADIENT_EDITOR_RAMP_HEIGHT),
            inset: scale(values::GRADIENT_EDITOR_INSET),
            marker: scale(values::GRADIENT_EDITOR_MARKER),
            pointer: scale(values::GRADIENT_EDITOR_POINTER),
            diamond: scale(values::GRADIENT_EDITOR_DIAMOND),
            hit: scale(values::GRADIENT_EDITOR_HIT),
            diamond_hit: scale(values::GRADIENT_EDITOR_DIAMOND_HIT),
            tear_off: scale(values::GRADIENT_EDITOR_TEAR_OFF),
            drop_line: scale(values::GRADIENT_EDITOR_DROP_LINE),
            edge: c.line.control_edge,
            marker_edge: c.text.muted,
            selected: c.accent.solid,
        }
    }

    fn path(f: GradientEditorField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<GradientEditorField> {
        GradientEditorField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: GradientEditorField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: GradientEditorField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: GradientEditorField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    /// The marker outline identifies the control and the accent marks the selection: 3:1 on the
    /// panel (4.5:1 with high contrast).
    fn pairs() -> Vec<ContrastPair> {
        [GradientEditorField::Selected, GradientEditorField::MarkerEdge]
            .into_iter()
            .map(|f| ContrastPair {
                fg: TokenRef::Component(ComponentField::GradientEditor(f)),
                bg: TokenRef::Color(T::SurfacePanel),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
