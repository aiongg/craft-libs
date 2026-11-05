//! Before and after views: the divider line, its knob and the gaps between panes. The same at every Size. The line is `overlay.handle_fill` over an `overlay.halo` halo; the knob is `accent.strong` with a `text.on_strong` glyph.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Before and after views: the divider line, knob and gaps.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterTokens {
    /// The divider line's width: `before_after.line`.
    pub line: f32,
    /// The halo's extra width on each side of the line: `before_after.halo`.
    pub halo: f32,
    /// The knob's diameter: `before_after.knob`.
    pub knob: f32,
    /// Between two side-by-side panes: `before_after.gap_side`.
    pub gap_side: f32,
    /// Between two stacked panes: `before_after.gap_stacked`.
    pub gap_stacked: f32,
    /// Below a caption: `before_after.caption_gap`.
    pub caption_gap: f32,
}

/// Before and after views have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BeforeAfterField {}

impl Family for BeforeAfterTokens {
    type Field = BeforeAfterField;
    const FIELDS: &'static [BeforeAfterField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BeforeAfterTokens;
        "line" 0.0..=16.0 => line;
        "halo" 0.0..=16.0 => halo;
        "knob" 0.0..=128.0 => knob;
        "gap_side" 0.0..=128.0 => gap_side;
        "gap_stacked" 0.0..=128.0 => gap_stacked;
        "caption_gap" 0.0..=64.0 => caption_gap;
    };

    fn defaults(_: &Semantic<'_>) -> BeforeAfterTokens {
        BeforeAfterTokens {
            line: values::BEFORE_AFTER_LINE,
            halo: values::BEFORE_AFTER_HALO,
            knob: values::BEFORE_AFTER_KNOB,
            gap_side: values::BEFORE_AFTER_GAP_SIDE,
            gap_stacked: values::BEFORE_AFTER_GAP_STACKED,
            caption_gap: values::BEFORE_AFTER_CAPTION_GAP,
        }
    }

    fn path(f: BeforeAfterField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<BeforeAfterField> {
        None
    }

    fn kind(f: BeforeAfterField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: BeforeAfterField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: BeforeAfterField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
