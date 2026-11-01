//! File list pickers: the list box and the width of its buttons. Sizes only (no colours): the colours are the surface, text and state tokens of the controls
//! inside, and no theme overrides these. The values are PhotoCraft's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// FileList tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FileListTokens {
    /// Size of the list box: `file_list.size`.
    pub size: [f32; 2],
    /// Least width of a button beside the list: `file_list.button_min`.
    pub button_min: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileListField {}

impl Family for FileListTokens {
    type Field = FileListField;
    const FIELDS: &'static [FileListField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FileListTokens;
        "size.0" 0.0..=4096.0 => size[0];
        "size.1" 0.0..=4096.0 => size[1];
        "button_min" 0.0..=1024.0 => button_min;
    };

    fn defaults(_: &Semantic<'_>) -> FileListTokens {
        FileListTokens { size: values::FILE_LIST_SIZE, button_min: values::FILE_LIST_BUTTON_MIN }
    }

    fn path(f: FileListField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FileListField> {
        None
    }

    fn kind(f: FileListField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FileListField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FileListField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
