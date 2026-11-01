//! Liquify's marks on the canvas: the Freeze Lasso and the freeze mask. The same at every Size (they sit on the image). The mesh and the brush outline are other families'.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Liquify's marks on the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LiquifyTokens {
    /// The Freeze Lasso's line width: `liquify.lasso`.
    pub lasso: f32,
    /// The freeze mask's opacity over the image, 0 to 1: `liquify.mask_opacity`.
    pub mask_opacity: f32,
    /// The freeze mask's tint and the Freeze Lasso's line while it freezes: `liquify.freeze`, `status.bad.fg`.
    pub freeze: Rgba,
    /// The Freeze Lasso's line while it thaws: `liquify.thaw`, `accent`.
    pub thaw: Rgba,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiquifyField {}

impl Family for LiquifyTokens {
    type Field = LiquifyField;
    const FIELDS: &'static [LiquifyField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { LiquifyTokens;
        "lasso" 0.0..=16.0 => lasso;
        "mask_opacity" 0.0..=1.0 => mask_opacity;
    };

    fn defaults(s: &Semantic<'_>) -> LiquifyTokens {
        LiquifyTokens {
            lasso: values::LIQUIFY_LASSO,
            mask_opacity: values::LIQUIFY_MASK_OPACITY,
            freeze: s.color.status.bad.fg,
            thaw: s.color.accent.solid,
        }
    }

    fn path(f: LiquifyField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<LiquifyField> {
        None
    }

    fn kind(f: LiquifyField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: LiquifyField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: LiquifyField, _: OverrideValue) {
        match f {}
    }

    // The two colours are the semantic ones, read as they are.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
