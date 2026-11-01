//! The Blend If bar: its width range, ramp and handle triangles. The same at every Size.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Blend If bar: its width range, ramp and handle triangles. The same at every Size.
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
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendIfField {}

impl Family for BlendIfTokens {
    type Field = BlendIfField;
    const FIELDS: &'static [BlendIfField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BlendIfTokens;
        "width_min" 0.0..=1024.0 => width_min;
        "width_max" 0.0..=2048.0 => width_max;
        "height" 0.0..=256.0 => height;
        "ramp" 0.0..=128.0 => ramp;
        "inset" 0.0..=64.0 => inset;
        "handle_width" 0.0..=64.0 => handle_width;
        "handle_height" 0.0..=64.0 => handle_height;
    };

    fn defaults(_: &Semantic<'_>) -> BlendIfTokens {
        BlendIfTokens {
            width_min: values::BLEND_IF_WIDTH_MIN,
            width_max: values::BLEND_IF_WIDTH_MAX,
            height: values::BLEND_IF_HEIGHT,
            ramp: values::BLEND_IF_RAMP,
            inset: values::BLEND_IF_INSET,
            handle_width: values::BLEND_IF_HANDLE_WIDTH,
            handle_height: values::BLEND_IF_HANDLE_HEIGHT,
        }
    }

    fn path(f: BlendIfField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<BlendIfField> {
        None
    }

    fn kind(f: BlendIfField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: BlendIfField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: BlendIfField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
