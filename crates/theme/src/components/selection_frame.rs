//! Selection frames: the box around selected objects, the key object's heavier line, the centre mark and a dashed outline. The same at every Size (they sit on the document). Their colour is the semantic `overlay.line`; the grips are the `handle` family's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Selection frames: the box, key object line, centre mark and dashed outline.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionFrameTokens {
    /// The box line and a solid outline: `selection_frame.line`.
    pub line: f32,
    /// The key object's box line: `selection_frame.key_line`.
    pub key_line: f32,
    /// The centre mark's side: `selection_frame.centre`.
    pub centre: f32,
    /// A dashed outline's width: `selection_frame.outline`.
    pub outline: f32,
    /// A dashed outline's dash: `selection_frame.dash`.
    pub dash: f32,
    /// A dashed outline's gap: `selection_frame.gap`.
    pub gap: f32,
    /// What one grow step adds to a grip: `selection_frame.grow_step`.
    pub grow_step: f32,
}

/// Selection frames have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SelectionFrameField {}

impl Family for SelectionFrameTokens {
    type Field = SelectionFrameField;
    const FIELDS: &'static [SelectionFrameField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SelectionFrameTokens;
        "line" 0.0..=16.0 => line;
        "key_line" 0.0..=16.0 => key_line;
        "centre" 0.0..=64.0 => centre;
        "outline" 0.0..=16.0 => outline;
        "dash" 0.0..=64.0 => dash;
        "gap" 0.0..=64.0 => gap;
        "grow_step" 0.0..=16.0 => grow_step;
    };

    fn defaults(_: &Semantic<'_>) -> SelectionFrameTokens {
        SelectionFrameTokens {
            line: values::SELECTION_FRAME_LINE,
            key_line: values::SELECTION_FRAME_KEY_LINE,
            centre: values::SELECTION_FRAME_CENTRE,
            outline: values::SELECTION_FRAME_OUTLINE,
            dash: values::SELECTION_FRAME_DASH,
            gap: values::SELECTION_FRAME_GAP,
            grow_step: values::SELECTION_FRAME_GROW_STEP,
        }
    }

    fn path(f: SelectionFrameField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SelectionFrameField> {
        None
    }

    fn kind(f: SelectionFrameField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SelectionFrameField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SelectionFrameField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
