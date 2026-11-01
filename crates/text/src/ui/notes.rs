//! [`NotesEditor`] (N3.18): a multi-line rich-text box for speaker or page notes (PowerPoint's
//! Notes pane): the notes' bold, italic, underline and colour runs drawn while editing, bullets
//! and numbers at each paragraph's start with wrapped lines under the text, not under the bullet;
//! typing keeps the formatting around the caret. [`NotesView`] draws the same notes read-only and
//! selectable (the presenter's notes pane).
//!
//! The notes are the app's; [`NotesText`] is the shape the editor reads and writes.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use craft_ui::primitives::Content;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _, scale};
use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{Align2, Color32, FontId, Galley, Label, Pos2, Response, Ui, WidgetText, vec2};

use crate::lists::{ListMarker, ListParagraph, line_starts, markers};

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
        let paragraphs: Vec<String> = self.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect()).collect();
        paragraphs.join("\n")
    }

    /// The notes as they read with `text` typed in, one edit (an insertion or a deletion) away
    /// from [`plain`](Self::plain): the runs around the edit keep their formatting and typed text
    /// takes the formatting before it; `None` when `text` isn't one edit away (the app then
    /// replaces the notes with plain paragraphs).
    // deckcraft: crates/ui-egui/src/panels/notes.rs:41 `typed`.
    pub fn typed(&self, text: &str) -> Option<NotesText> {
        let old: Vec<char> = self.plain().chars().collect();
        let new: Vec<char> = text.chars().collect();
        if old == new {
            return Some(self.clone());
        }
        if new.contains(&'\r') {
            return None;
        }
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let room = old.len().min(new.len()) - prefix;
        let suffix = old.iter().rev().zip(new.iter().rev()).take(room).take_while(|(a, b)| a == b).count();
        if prefix == 0 && suffix == 0 {
            return None;
        }
        let mut flat = Flat::of(self);
        flat.replace(prefix..old.len() - suffix, &new[prefix..new.len() - suffix]);
        Some(flat.into_notes())
    }
}

/// The formatting of one character.
type Style = NotesRun;

/// One element of the notes as a flat sequence: a character with its formatting, or the break
/// that starts the next paragraph (carrying that paragraph's own settings).
#[derive(Clone)]
enum Item {
    Char(char, Style),
    Break(NotesParagraph),
}

/// The notes as the sequence of characters and paragraph breaks that `plain` writes out.
struct Flat {
    first: NotesParagraph,
    items: Vec<Item>,
}

impl Flat {
    fn of(notes: &NotesText) -> Flat {
        let bare = |p: &NotesParagraph| NotesParagraph { runs: Vec::new(), ..p.clone() };
        let mut flat = Flat { first: notes.paragraphs.first().map(bare).unwrap_or_default(), items: Vec::new() };
        for (i, p) in notes.paragraphs.iter().enumerate() {
            if i > 0 {
                flat.items.push(Item::Break(bare(p)));
            }
            for run in &p.runs {
                let style = NotesRun { text: String::new(), ..run.clone() };
                flat.items.extend(run.text.chars().map(|c| Item::Char(c, style.clone())));
            }
        }
        flat
    }

    /// Replaces the items in `span` with `text`: the text takes the formatting of the character
    /// before it in its paragraph, else of the one after; a typed line break starts a paragraph
    /// with the settings of the one it splits; a deleted break joins the paragraphs under the
    /// first one's settings.
    fn replace(&mut self, span: std::ops::Range<usize>, text: &[char]) {
        let paragraph_start = |items: &[Item], at: usize| items[..at].iter().rposition(|i| matches!(i, Item::Break(_))).map_or(0, |b| b + 1);
        let style = {
            let from = paragraph_start(&self.items, span.start);
            let before = (span.start > from).then(|| &self.items[span.start - 1]);
            let after = self.items.get(span.end).filter(|_| span.start == from);
            match before.or(after) {
                Some(Item::Char(_, style)) => style.clone(),
                _ => Style::default(),
            }
        };
        let split = {
            let from = paragraph_start(&self.items, span.start);
            match from {
                0 => self.first.clone(),
                b => match &self.items[b - 1] {
                    Item::Break(p) => p.clone(),
                    Item::Char(..) => NotesParagraph::default(),
                },
            }
        };
        let typed: Vec<Item> = text.iter().map(|&c| if c == '\n' { Item::Break(split.clone()) } else { Item::Char(c, style.clone()) }).collect();
        self.items.splice(span, typed);
    }

    fn into_notes(self) -> NotesText {
        let mut paragraphs = Vec::new();
        let mut current = self.first;
        for item in self.items {
            match item {
                Item::Char(c, style) => match current.runs.last_mut() {
                    Some(run) if NotesRun { text: String::new(), ..run.clone() } == style => run.text.push(c),
                    _ => current.runs.push(NotesRun { text: c.to_string(), ..style }),
                },
                Item::Break(next) => paragraphs.push(std::mem::replace(&mut current, next)),
            }
        }
        paragraphs.push(current);
        NotesText { paragraphs }
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
        let look = Look::of(ui, self.zoom);
        let galley = look.galley(ui, self.notes, ui.available_width());
        ui.push_id(self.id_salt, |ui| {
            let response = ui.add(Label::new(WidgetText::Galley(galley.clone())).selectable(true));
            look.paint_markers(ui, self.notes, &galley, response.rect.min);
            response
        })
        .inner
    }
}

/// The notes' type and measures: the body role's face at the pane's zoom, document points as
/// logical pixels at that zoom.
struct Look {
    regular: FontId,
    strong: FontId,
    color: Color32,
    zoom: f32,
    underline: egui::Stroke,
}

/// One paragraph laid out: its lines start at `margin`, the first line's text at `text` (both
/// logical pixels from the notes' start edge) and its marker at `first`.
struct Layout {
    job: LayoutJob,
    first: f32,
    margin: f32,
    text: f32,
}

impl Look {
    fn of(ui: &Ui, zoom: f32) -> Look {
        let tokens = Tokens::of(ui);
        let body = tokens.families().font_id(tokens.typography.body);
        let strong = tokens.families().font_id(tokens.typography.body_strong);
        let color = Content::Normal.color(&tokens).to_egui();
        Look {
            regular: FontId::new(body.size * zoom, body.family),
            strong: FontId::new(body.size * zoom, strong.family),
            color,
            zoom,
            underline: scale::Stroke::W1.egui(ui.ctx(), color),
        }
    }

    fn format(&self, run: &NotesRun) -> TextFormat {
        let color = run.color.unwrap_or(self.color);
        let mut format = TextFormat::simple(if run.bold { self.strong.clone() } else { self.regular.clone() }, color);
        format.italics = run.italic;
        if run.underline {
            format.underline = egui::Stroke { color, ..self.underline };
        }
        if run.strikethrough {
            format.strikethrough = egui::Stroke { color, ..self.underline };
        }
        format
    }

    fn layout(&self, ui: &Ui, p: &NotesParagraph, marker: Option<&str>, wrap: f32) -> Layout {
        let (first, margin) = line_starts(p.level, p.margin, p.indent);
        let points = |v: f64| if v.is_finite() { v.max(0.0) as f32 * self.zoom } else { 0.0 };
        let (first, margin) = (points(first), points(margin));
        let width = |text: &str| ui.painter().layout_no_wrap(text.to_owned(), self.regular.clone(), self.color).size().x;
        // The text starts at the margin, after the marker and one space if the marker is wider
        // than the hanging indent.
        let text = match marker {
            Some(m) => margin.max(first + width(m) + width(" ")),
            None => first,
        };
        let mut job = LayoutJob::default();
        job.wrap.max_width = (wrap - margin).max(0.0);
        let mut lead = (text - margin).max(0.0);
        for run in p.runs.iter().filter(|r| !r.text.is_empty()) {
            job.append(&run.text, lead, self.format(run));
            lead = 0.0;
        }
        if job.text.is_empty() {
            job.append("", 0.0, self.format(&NotesRun::default()));
        }
        Layout { job, first, margin, text }
    }

    fn markers(notes: &NotesText) -> Vec<Option<String>> {
        let list: Vec<ListParagraph> = notes
            .paragraphs
            .iter()
            .map(|p| ListParagraph { level: p.level, marker: p.marker.clone(), empty: p.runs.iter().all(|r| r.text.is_empty()) })
            .collect();
        markers(&list)
    }

    /// The notes laid out paragraph by paragraph, each moved to its indent, so wrapped lines
    /// line up under the text and not under the marker.
    fn galley(&self, ui: &Ui, notes: &NotesText, wrap: f32) -> Arc<Galley> {
        let markers = Self::markers(notes);
        let layouts: Vec<Layout> = notes.paragraphs.iter().zip(&markers).map(|(p, m)| self.layout(ui, p, m.as_deref(), wrap)).collect();
        let mut whole = LayoutJob::default();
        whole.wrap.max_width = wrap;
        for (i, layout) in layouts.iter().enumerate() {
            if i > 0 {
                whole.append("\n", 0.0, self.format(&NotesRun::default()));
            }
            for section in &layout.job.sections {
                if let Some(text) = layout.job.text.get(section.byte_range.start.0..section.byte_range.end.0) {
                    whole.append(text, section.leading_space, section.format.clone());
                }
            }
        }
        if layouts.is_empty() {
            return ui.fonts_mut(|f| f.layout_job(whole));
        }
        let parts: Vec<Arc<Galley>> = ui.fonts_mut(|f| {
            layouts
                .into_iter()
                .map(|layout| {
                    let mut galley = (*f.layout_job(layout.job)).clone();
                    for (i, row) in galley.rows.iter_mut().enumerate() {
                        // The first line already holds the gap after a marker; a first line
                        // hanging left of the margin without one starts there.
                        row.pos.x += if i == 0 { layout.margin.min(layout.text) } else { layout.margin };
                    }
                    Arc::new(galley)
                })
                .collect()
        });
        Arc::new(Galley::concat(Arc::new(whole), &parts, ui.ctx().pixels_per_point()))
    }

    /// The markers at the start of each paragraph's first line of `galley` (laid out from
    /// `notes`), drawn at `origin`.
    fn paint_markers(&self, ui: &Ui, notes: &NotesText, galley: &Galley, origin: Pos2) {
        let mut at = 0;
        for (p, marker) in notes.paragraphs.iter().zip(Self::markers(notes)) {
            if let Some(marker) = marker {
                let row = galley.pos_from_cursor(CCursor::new(at));
                let (first, _) = line_starts(p.level, p.margin, p.indent);
                let x = if first.is_finite() { first.max(0.0) as f32 * self.zoom } else { 0.0 };
                ui.painter().text(origin + vec2(x, row.center().y), Align2::LEFT_CENTER, marker, self.regular.clone(), self.color);
            }
            at += p.runs.iter().map(|r| r.text.chars().count()).sum::<usize>() + 1;
        }
    }
}

/// The notes laid out as the view and the editor draw them, wrapped to `width`.
pub(super) fn notes_galley(ui: &Ui, notes: &NotesText, zoom: f32, width: f32) -> Arc<Galley> {
    Look::of(ui, zoom).galley(ui, notes, width)
}
