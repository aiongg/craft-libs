//! [`ReportDialog`]: a summary line plus a scrolling list of notes with a count (an import's
//! warnings, a batch's results).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;

crate::words! {
    /// The report dialog's words, English by default.
    #[non_exhaustive]
    pub struct ReportLabels {
        notes_one: "report.notesOne" = "1 note";
        notes_many: "report.notesMany" = "{count} notes";
        copy: "report.copy" = "Copy";
    }
}

/// The weight of one note; it picks the icon before the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NoteTone {
    #[default]
    Info,
    Warning,
    Error,
}

/// One line of the report.
#[derive(Clone, Debug, PartialEq)]
pub struct ReportNote {
    pub tone: NoteTone,
    pub text: String,
}

impl ReportNote {
    pub fn new(tone: NoteTone, text: impl Into<String>) -> Self {
        Self { tone, text: text.into() }
    }
}

/// A report: the user sees the title, one summary line ("Imported 12 pages; 3 fonts were
/// substituted."), the count of notes, a scrolling list of notes with tone icons
/// ([`Body::Scroll`](super::Body::Scroll)), a Copy button that puts the notes on the clipboard
/// as text, and OK. Enter and Esc both close it. Result: `()`.
pub struct ReportDialog {
    id: DialogId,
    title: String,
    summary: String,
    notes: Vec<ReportNote>,
    width: Width,
    state: DialogState<()>,
}

impl ReportDialog {
    pub fn new(key: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>, summary: impl Into<String>, notes: Vec<ReportNote>) -> Self {
        Self { id: dialog_id(key), title: title.into(), summary: summary.into(), notes, width: Width::Standard, state: DialogState::new() }
    }

    pub fn width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    /// The notes as the Copy button writes them, one per line with the tone word first.
    // new: FigCraft and LightCraft show the list without copying.
    pub fn as_text(&self) -> String {
        todo!()
    }
}

impl StackDialog for ReportDialog {
    type Result = ();

    fn id(&self) -> DialogId {
        self.id
    }

    // new: from FigCraft's and LightCraft's result lists (homes-in F01.11: figcraft 57 lines,
    // lightcraft 50); on `Dialog` with `Body::Scroll`, `Footer::standard().without_cancel()`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        todo!()
    }
}
