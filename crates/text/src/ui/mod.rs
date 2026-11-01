//! The text interface on craft-ui (feature `ui`):
//!
//! - [`editor`] (F14.02): [`TextEditor`](editor::TextEditor), the text editor on a canvas, over a
//!   [`TextEditorHost`](editor::TextEditorHost) the app implements for its text and layout.
//!   Input methods go through craft-ui's [`Composer`](craft_ui::ime::Composer), with the marked
//!   text written into the app's text so it lays out like typed text.
//! - [`character`] (F14.04): [`CharacterControls`](character::CharacterControls) and
//!   [`ParagraphControls`](character::ParagraphControls), Photoshop's Character and Paragraph
//!   panels as controls for a panel body, a Properties section or an options bar.
//! - [`style_panel`] (F14.03): [`TextStylesPanel`](style_panel::TextStylesPanel), the Character
//!   Styles and Paragraph Styles panels.
//! - [`spelling`] (F14.06): [`CheckSpellingDialog`](spelling::CheckSpellingDialog), Edit › Check
//!   Spelling on the dialog stack, and the user's words as a settings section.
//!
//! The paged-text controls (6b):
//!
//! - [`ruler`] (N3.02): [`TextRuler`](ruler::TextRuler), tab stops and indents on a ruler.
//! - [`tabs_dialog`] (N3.09): [`TabStopsDialog`](tabs_dialog::TabStopsDialog).
//! - [`character`] also has the Paragraph dialog (N3.15):
//!   [`ParagraphDialog`](character::ParagraphDialog).
//! - [`style_panel`] also has multi-row highlight and the style chip (N3.04):
//!   [`StyleChip`](style_panel::StyleChip).
//! - [`notes`] (N3.18): [`NotesEditor`](notes::NotesEditor) and [`NotesView`](notes::NotesView).
//! - [`columns`] (N3.27): [`TextColumnsDialog`](columns::TextColumnsDialog) and the Columns menu.
//! - [`hyperlink`] (N3.28): [`HyperlinkDialog`](hyperlink::HyperlinkDialog).
//! - [`find`] (N3.03): [`FindBar`](find::FindBar) and [`FindReplaceDialog`](find::FindReplaceDialog).

pub mod character;
#[cfg(test)]
mod character_tests;
pub mod columns;
#[cfg(test)]
mod columns_tests;
pub mod editor;
#[cfg(test)]
mod editor_tests;
pub mod find;
#[cfg(test)]
mod find_tests;
pub mod hyperlink;
#[cfg(test)]
mod hyperlink_tests;
pub mod notes;
#[cfg(test)]
mod notes_tests;
mod paged_stories;
pub mod ruler;
#[cfg(test)]
mod ruler_tests;
pub mod spelling;
#[cfg(test)]
mod spelling_tests;
mod stories;
pub mod style_panel;
#[cfg(test)]
mod style_panel_tests;
pub mod tabs_dialog;
#[cfg(test)]
mod tabs_dialog_tests;

pub use stories::stories;

/// The count words of the text interface (none yet), for the facade's catalog extraction.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[];

/// Every labels table of the text interface, for the facade's catalog extraction.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    <character::CharacterLabels as craft_ui::localized::Labels>::WORDS,
    <character::ParagraphLabels as craft_ui::localized::Labels>::WORDS,
    <style_panel::TextStylesLabels as craft_ui::localized::Labels>::WORDS,
    <spelling::CheckSpellingLabels as craft_ui::localized::Labels>::WORDS,
];
