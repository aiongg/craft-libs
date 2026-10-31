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

craft_ui::words! {
    /// The Replace question of a Save the file panel did not ask about. The service shows it as a
    /// native message box, so the app hands these words to the panels with
    /// [`ReplaceLabels::words`] at startup and when the language changes.
    #[non_exhaustive]
    pub struct ReplaceLabels {
        title: "fileService.replaceTitle" = "Replace";
        question: "fileService.replaceQuestion" = "“{name}” already exists. Replace it?";
        replace: "fileService.replace" = "Replace";
        cancel: "fileService.replaceCancel" = "Cancel";
    }
}

impl ReplaceLabels {
    /// The words for [`FilePanels::replace_words`](crate::service::FilePanels::replace_words).
    pub fn words(&self) -> crate::service::ReplaceWords {
        crate::service::ReplaceWords {
            title: self.title.clone(),
            question: self.question.clone(),
            replace: self.replace.clone(),
            cancel: self.cancel.clone(),
        }
    }
}

pub use file_list::{FileList, FileListLabels, FileListMode, FileListPicker, FileListPickerResponse};
pub use path_field::{PathField, PathFieldLabels, PathFieldResponse};

/// The count words of the file service's egui parts (none yet), for the words registry.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[];

/// The labels tables of the file service's egui parts, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    <FileListLabels as craft_ui::localized::Labels>::WORDS,
    <PathFieldLabels as craft_ui::localized::Labels>::WORDS,
    <ReplaceLabels as craft_ui::localized::Labels>::WORDS,
];
