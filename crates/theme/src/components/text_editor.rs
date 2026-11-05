//! The text editor on a canvas: the caret and the marked-text underlines. The selection and the
//! underlines draw in the accent (`accent.solid`); the caret's white halo and black line are
//! fixed so the caret reads over any pixels.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Text editor tokens, the same at every Size because they sit over a document.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextEditorTokens {
    /// Width of the caret's halo stroke, in points: `text_editor.caret_halo`.
    pub caret_halo: f32,
    /// Width of the caret's line, in points: `text_editor.caret_line`.
    pub caret_line: f32,
    /// Share of the accent the selection fill keeps, 0 to 1: `text_editor.selection_alpha`.
    pub selection_alpha: f32,
    /// Width of the underline of marked (uncommitted) text, in points: `text_editor.preedit_underline`.
    pub preedit_underline: f32,
    /// Width of the underline of the segment being converted, in points: `text_editor.active_underline`.
    pub active_underline: f32,
    /// The size of the notes' default text, points: a notes indent scales as `size / this`: `text_editor.notes_points`.
    pub notes_points: f32,
    /// The largest notes indent, points: `text_editor.notes_max_indent`.
    pub notes_max_indent: f32,
}

/// The text editor has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextEditorField {}

impl Family for TextEditorTokens {
    type Field = TextEditorField;
    const FIELDS: &'static [TextEditorField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TextEditorTokens;
        "caret_halo" 0.0..=16.0 => caret_halo;
        "caret_line" 0.0..=16.0 => caret_line;
        "selection_alpha" 0.0..=1.0 => selection_alpha;
        "preedit_underline" 0.0..=16.0 => preedit_underline;
        "active_underline" 0.0..=16.0 => active_underline;
        "notes_points" 1.0..=144.0 => notes_points;
        "notes_max_indent" 0.0..=4096.0 => notes_max_indent;
    };

    fn defaults(_: &Semantic<'_>) -> TextEditorTokens {
        TextEditorTokens {
            caret_halo: values::TEXT_EDITOR_CARET_HALO,
            caret_line: values::TEXT_EDITOR_CARET_LINE,
            selection_alpha: values::TEXT_EDITOR_SELECTION_ALPHA,
            preedit_underline: values::TEXT_EDITOR_PREEDIT_UNDERLINE,
            active_underline: values::TEXT_EDITOR_ACTIVE_UNDERLINE,
            notes_points: values::TEXT_EDITOR_NOTES_POINTS,
            notes_max_indent: values::TEXT_EDITOR_NOTES_MAX_INDENT,
        }
    }

    fn path(f: TextEditorField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TextEditorField> {
        None
    }

    fn kind(f: TextEditorField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TextEditorField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TextEditorField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the editor draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
