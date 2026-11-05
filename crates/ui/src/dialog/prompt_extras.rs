//! The optional parts of a prompt's or alert's body under its message: a tone line, a lines
//! list and a collapsible details section.

use craft_theme::Tone;
use egui::{Event, Id, Key, ScrollArea, Ui};

use crate::composites::{Notice, SectionHeader};

/// How many rows of text the lines list shows before it scrolls.
const LIST_ROWS: f32 = 6.0;

/// What [`show`](Extras::show) draws under the message, in this order: the tone line, the lines
/// list, the details section.
#[derive(Default)]
pub(super) struct Extras {
    pub(super) tone: Option<(Tone, String)>,
    /// `(label, text)` of the collapsible section.
    pub(super) details: Option<(String, String)>,
    pub(super) lines: Vec<String>,
}

impl Extras {
    /// Draws the parts. The details section starts closed; its state is kept under `id`.
    pub(super) fn show(&self, ui: &mut Ui, id: Id) {
        if let Some((tone, text)) = &self.tone {
            Notice::new(text.as_str()).tone(*tone).announce(false).id_salt((id, "tone")).show(ui);
        }
        if !self.lines.is_empty() {
            let height = ui.spacing().interact_size.y * LIST_ROWS;
            ScrollArea::vertical().id_salt((id, "lines")).max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                for line in &self.lines {
                    ui.label(line.as_str());
                }
            });
        }
        if let Some((label, text)) = &self.details {
            let key = id.with("details-open");
            let mut open = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
            let header = SectionHeader::new((id, "details"), label.as_str()).open(&mut open).show(ui, |ui| {
                ui.label(text.as_str());
            });
            ui.data_mut(|d| d.insert_temp(key, open));
            // Enter on the focused header toggles it; the dialog must not also answer it.
            if header.response.has_focus() {
                ui.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { key: Key::Enter, pressed: true, .. })));
            }
        }
    }
}
