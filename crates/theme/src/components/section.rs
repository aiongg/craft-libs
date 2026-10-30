//! Section headers: the row, chevron and title of a collapsible section. Sizes only (no colours): no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Section headers tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SectionTokens {
    /// Height of the header row: `section.row_height`.
    pub row_height: f32,
    /// Side of the chevron icon: `section.chevron`.
    pub chevron: f32,
    /// Side of the box the chevron is centred in: `section.chevron_box`.
    pub chevron_box: f32,
    /// From the row start to the title: `section.title_offset`.
    pub title_offset: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SectionField {}

impl Family for SectionTokens {
    type Field = SectionField;
    const FIELDS: &'static [SectionField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SectionTokens;
        "row_height" 0.0..=1024.0 => row_height;
        "chevron" 0.0..=1024.0 => chevron;
        "chevron_box" 0.0..=1024.0 => chevron_box;
        "title_offset" 0.0..=64.0 => title_offset;
    };

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> SectionTokens {
        let k = s.size.control.md / values::SECTION_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        SectionTokens {
            row_height: scale(values::SECTION_ROW_HEIGHT),
            chevron: scale(values::SECTION_CHEVRON),
            chevron_box: scale(values::SECTION_CHEVRON_BOX),
            title_offset: scale(values::SECTION_TITLE_OFFSET),
        }
    }

    fn path(f: SectionField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SectionField> {
        None
    }

    fn kind(f: SectionField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SectionField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SectionField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
