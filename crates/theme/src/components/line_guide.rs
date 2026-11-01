//! Line guides: the preview line of a straight-line stroke and the Ruler. The same at every Size. The line is `overlay.stroke` and its halo `overlay.halo`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Line guides: the line a straight-line stroke previews.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LineGuideTokens {
    /// The line width: `line_guide.width`.
    pub width: f32,
    /// The halo's width, under the line: `line_guide.halo`.
    pub halo: f32,
}

/// Line guides have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineGuideField {}

impl Family for LineGuideTokens {
    type Field = LineGuideField;
    const FIELDS: &'static [LineGuideField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { LineGuideTokens;
        "width" 0.0..=16.0 => width;
        "halo" 0.0..=16.0 => halo;
    };

    fn defaults(_: &Semantic<'_>) -> LineGuideTokens {
        LineGuideTokens { width: values::LINE_GUIDE_WIDTH, halo: values::LINE_GUIDE_HALO }
    }

    fn path(f: LineGuideField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<LineGuideField> {
        None
    }

    fn kind(f: LineGuideField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: LineGuideField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: LineGuideField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
