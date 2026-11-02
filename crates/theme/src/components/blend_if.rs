//! The Blend If bar: its width range, ramp and handle triangles. The sizes are the same at every Size; the ramp's border is overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// The Blend If bar: its width range, ramp and handle triangles. The sizes are the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlendIfTokens {
    /// The bar's narrowest width: `blend_if.width_min`.
    pub width_min: f32,
    /// The bar's widest width: `blend_if.width_max`.
    pub width_max: f32,
    /// The bar's height: `blend_if.height`.
    pub height: f32,
    /// The ramp's height: `blend_if.ramp`.
    pub ramp: f32,
    /// The ramp's side inset: `blend_if.inset`.
    pub inset: f32,
    /// A handle triangle's width: `blend_if.handle_width`.
    pub handle_width: f32,
    /// A handle triangle's height: `blend_if.handle_height`.
    pub handle_height: f32,
    /// The ramp's 1 pt border: `line.control_edge`, the field edge: `blend_if.ramp_border`.
    pub ramp_border: Rgba,
}

/// An overridable Blend If field. Path: `ramp_border`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendIfField {
    /// `blend_if.ramp_border`
    RampBorder,
}

impl Family for BlendIfTokens {
    type Field = BlendIfField;
    const FIELDS: &'static [BlendIfField] = &[BlendIfField::RampBorder];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BlendIfTokens;
        "width_min" 0.0..=1024.0 => width_min;
        "width_max" 0.0..=2048.0 => width_max;
        "height" 0.0..=256.0 => height;
        "ramp" 0.0..=128.0 => ramp;
        "inset" 0.0..=64.0 => inset;
        "handle_width" 0.0..=64.0 => handle_width;
        "handle_height" 0.0..=64.0 => handle_height;
    };

    fn defaults(s: &Semantic<'_>) -> BlendIfTokens {
        BlendIfTokens {
            width_min: values::BLEND_IF_WIDTH_MIN,
            width_max: values::BLEND_IF_WIDTH_MAX,
            height: values::BLEND_IF_HEIGHT,
            ramp: values::BLEND_IF_RAMP,
            inset: values::BLEND_IF_INSET,
            handle_width: values::BLEND_IF_HANDLE_WIDTH,
            handle_height: values::BLEND_IF_HANDLE_HEIGHT,
            ramp_border: s.color.line.control_edge,
        }
    }

    fn path(_: BlendIfField) -> String {
        "ramp_border".to_owned()
    }

    fn from_path(path: &str) -> Option<BlendIfField> {
        (path == "ramp_border").then_some(BlendIfField::RampBorder)
    }

    fn kind(_: BlendIfField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: BlendIfField) -> OverrideValue {
        OverrideValue::Color(self.ramp_border)
    }

    fn set(&mut self, _: BlendIfField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.ramp_border = c;
        }
    }

    /// The ramp's border bounds the bar on the panel and on a raised surface (a dialog), as a
    /// field's rest border does.
    fn pairs() -> Vec<ContrastPair> {
        let edge = TokenRef::Component(ComponentField::BlendIf(BlendIfField::RampBorder));
        [ColorToken::SurfacePanel, ColorToken::SurfaceRaised]
            .map(|bg| ContrastPair { fg: edge, bg: TokenRef::Color(bg), minimum: values::CONTROL_EDGE_MIN, alternate: None })
            .to_vec()
    }
}
