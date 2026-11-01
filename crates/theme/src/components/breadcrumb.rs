//! Breadcrumb bars: a path of segments with chevrons between. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Breadcrumb tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BreadcrumbTokens {
    /// Height of the bar, a standard control's (`size.control.md`): `breadcrumb.height`.
    pub height: f32,
    /// Side of the chevron between segments: `breadcrumb.separator`.
    pub separator: f32,
    /// Beside a segment's text and either side of a chevron: `breadcrumb.gap`.
    pub gap: f32,
    /// The room one shown segment counts for when the bar decides how many fit: `breadcrumb.min_segment`.
    pub min_segment: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BreadcrumbField {}

impl Family for BreadcrumbTokens {
    type Field = BreadcrumbField;
    const FIELDS: &'static [BreadcrumbField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BreadcrumbTokens;
        "height" 0.0..=256.0 => height;
        "separator" 0.0..=64.0 => separator;
        "gap" 0.0..=64.0 => gap;
        "min_segment" 0.0..=1024.0 => min_segment;
    };

    fn defaults(s: &Semantic<'_>) -> BreadcrumbTokens {
        BreadcrumbTokens {
            height: s.size.control.md,
            separator: values::BREADCRUMB_SEPARATOR,
            gap: values::BREADCRUMB_GAP,
            min_segment: values::BREADCRUMB_MIN_SEGMENT,
        }
    }

    fn path(f: BreadcrumbField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<BreadcrumbField> {
        None
    }

    fn kind(f: BreadcrumbField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: BreadcrumbField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: BreadcrumbField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
