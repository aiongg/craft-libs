//! [`ReportDialog`]: a summary line plus a scrolling list of notes with a count (an import's
//! warnings, a batch's results).

use craft_theme::TypeRole;
use egui::Context;

use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::{Body, Width};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{Notice, Tone};
use crate::controls::Button;
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};

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

impl NoteTone {
    /// The word that starts the note's line when the notes are copied as text.
    fn word(self) -> &'static str {
        match self {
            NoteTone::Info => "Info",
            NoteTone::Warning => "Warning",
            NoteTone::Error => "Error",
        }
    }

    fn tone(self) -> Tone {
        match self {
            NoteTone::Info => Tone::Info,
            NoteTone::Warning => Tone::Warn,
            NoteTone::Error => Tone::Bad,
        }
    }
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
        self.notes.iter().map(|note| format!("{}: {}", note.tone.word(), note.text)).collect::<Vec<_>>().join("\n")
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
        let labels = ReportLabels::of_ctx(ctx);
        let count = match self.notes.len() {
            0 => None,
            1 => Some(labels.notes_one.clone()),
            n => Some(labels.notes_many.replace("{count}", &n.to_string())),
        };
        let copy = labels.copy;
        let text = self.as_text();
        let (summary, notes, id) = (&self.summary, &self.notes, self.id);
        let footer = Footer::standard().without_cancel().leading(|ui| {
            if ui.add(Button::new(copy)).clicked() {
                ui.ctx().copy_text(text);
            }
        });
        let shown = Dialog::with_id(self.id, self.title.as_str()).stack(stack).width(self.width).body(Body::Scroll).footer(footer).show(
            ctx,
            &mut self.state,
            &mut (),
            |ui, ()| {
                ui.add(Text::new(summary.as_str()));
                if let Some(count) = &count {
                    ui.add(Text::new(count.as_str()).role(TypeRole::Caption).content(Content::Muted));
                }
                for (i, note) in notes.iter().enumerate() {
                    Notice::new(note.text.as_str()).tone(note.tone.tone()).announce(false).id_salt((id, i)).show(ui);
                }
            },
        );
        matches!(shown.outcome, Some(Outcome::Ok | Outcome::Cancel | Outcome::Close)).then_some(DialogResult::Ok(()))
    }
}
