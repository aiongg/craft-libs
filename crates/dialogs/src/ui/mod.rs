//! The file service's egui parts: [`PathField`] and [`FileListPicker`].

mod file_list;
#[cfg(test)]
mod file_list_tests;
mod path_field;
#[cfg(test)]
mod path_field_tests;
mod story;

craft_ui::story::register! {
    story,
}

/// The stories of the file list picker and the path field.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

pub use file_list::{FileList, FileListLabels, FileListMode, FileListPicker, FileListResponse};
pub use path_field::{PathField, PathFieldLabels, PathFieldResponse};

/// The labels tables of the file service's egui parts, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] =
    &[<FileListLabels as craft_ui::localized::Labels>::WORDS, <PathFieldLabels as craft_ui::localized::Labels>::WORDS];
