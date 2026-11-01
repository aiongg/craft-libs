//! [`NotesEditor`] (N3.18): a multi-line rich-text box for speaker or page notes (PowerPoint's
//! Notes pane): the notes' bold, italic, underline and colour runs drawn while editing, bullets
//! and numbers at each paragraph's start with wrapped lines under the text, not under the bullet;
//! typing keeps the formatting around the caret. [`NotesView`] draws the same notes read-only and
//! selectable (the presenter's notes pane).
//!
//! The notes are the app's; [`NotesText`] is the shape the editor reads and writes.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Response, Ui};

use crate::lists::ListMarker;

/// A run of notes text with its formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    /// `None`: the editor's text colour.
    pub color: Option<Color32>,
}

/// One paragraph: its runs, list level and marker, and its margins (points; `None` takes
/// [`crate::lists::LEVEL_INDENT`] per level).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesParagraph {
    pub runs: Vec<NotesRun>,
    pub level: u8,
    pub marker: Option<ListMarker>,
    pub margin: Option<f64>,
    pub indent: Option<f64>,
}

/// The notes as the editor sees them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesText {
    pub paragraphs: Vec<NotesParagraph>,
}

impl NotesText {
    /// The plain text, paragraphs joined with `\n` (what the text field edits).
    pub fn plain(&self) -> String {
        todo!()
    }

    /// The notes as they read with `text` typed in, one edit (an insertion or a deletion) away
    /// from [`plain`](Self::plain): the runs around the edit keep their formatting and typed text
    /// takes the formatting before it; `None` when `text` isn't one edit away (the app then
    /// replaces the notes with plain paragraphs).
    // deckcraft: crates/ui-egui/src/panels/notes.rs:41 `typed`.
    pub fn typed(&self, text: &str) -> Option<NotesText> {
        todo!()
    }
}

/// What the editor did this pass.
#[derive(Debug)]
pub struct NotesResponse {
    /// The notes changed: the app stores them (one undo step per typing run, as text fields).
    pub changed: Option<NotesText>,
    pub response: Response,
}

/// The editor, built per pass over the app's notes.
///
/// - Text at the body type role × [`zoom`](NotesEditor::zoom), wrapped to
///   the width; each paragraph laid out at its indent, its marker
///   ([`crate::lists::markers`]) drawn at its first line's start.
/// - An empty editor shows [`NotesLabels::placeholder`]; the editor is named by
///   [`NotesLabels::notes`].
/// - Keys and focus are the craft-ui text field's (it is one, multi-line); ⌘/Ctrl-B, -I, -U toggle
///   the run style at the selection through the app's commands, not here.
pub struct NotesEditor<'a> {
    id_salt: egui::IdSalt,
    notes: &'a NotesText,
    zoom: f32,
    labels: Option<NotesLabels>,
}

craft_ui::words! {
    /// The notes pane's words (PowerPoint's).
    #[non_exhaustive]
    pub struct NotesLabels {
        notes: "notes.notes" = "Notes";
        placeholder: "notes.placeholder" = "Click to add notes";
    }
}

impl<'a> NotesEditor<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, notes: &'a NotesText) -> Self {
        NotesEditor { id_salt: egui::IdSalt::new(id_salt), notes, zoom: 1.0, labels: None }
    }

    /// The notes pane's zoom (PowerPoint's Zoom of the notes pane), a factor over the body type
    /// role's size; 1 by default.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    pub fn labels(mut self, labels: NotesLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // deckcraft: crates/ui-egui/src/panels/notes.rs:189 `editor`, :117 `para_layout`, :143
    // `job`, :162 `galley`, :215 `paint_bullets`.
    pub fn show(self, ui: &mut Ui) -> NotesResponse {
        todo!()
    }
}

/// The notes read-only, wrapped to the width and selectable like a label.
pub struct NotesView<'a> {
    id_salt: egui::IdSalt,
    notes: &'a NotesText,
    zoom: f32,
}

impl<'a> NotesView<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, notes: &'a NotesText) -> Self {
        NotesView { id_salt: egui::IdSalt::new(id_salt), notes, zoom: 1.0 }
    }

    /// A factor over the body type role's size, as [`NotesEditor::zoom`].
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    // deckcraft: crates/ui-egui/src/panels/notes.rs:231 `view` (the presenter's read-only notes).
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
