//! [`NotesEditor`] (N3.18): a multi-line rich-text box for speaker or page notes (PowerPoint's Notes pane): the
//! notes' bold, italic, underline and colour runs drawn while editing, bullets and numbers at each paragraph's start
//! with wrapped lines under the text, not under the bullet; typing keeps the formatting around the caret. Read-only,
//! the same component is the presenter's notes pane.
//!
//! The notes are the app's; [`NotesText`] is the shape the editor reads and writes.

#[cfg(feature = "screen-testing")]
mod stories;

use std::ops::Range;

use craft_behavior::{EditCommand, EditorInput, Granularity, Metrics, NavMove, Sense, TextPos};
use craft_ui::contract::{
    Animate as _, Commands as _, Common, Component, Cx, Focus as _, MeasureCx, NoValue, Outcome, PaintUi, PartsCx, Props as _, Remember as _, Role,
    WordSetCx as _,
};
use craft_ui::fonts::FontRef;
use craft_ui::primitives::{DocRun, DocText, EditorLine, EditorPurpose, EditorRun, EditorState};
use craft_ui::runtime::commands::ClipItem;
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::runtime::pending::Pending;
use craft_ui::style::{ComponentStyle, DataColor, StyleColor, StyleCx};
use egui::{Color32, Pos2, Rect, Response, Stroke, Vec2};

use super::editor::word_boundary;
use crate::lists::{ListMarker, ListParagraph, line_starts, markers};

/// The largest indent, points.
const MAX_INDENT: f32 = 400.0;
/// The columns and lines the editor measures when nothing else sizes it.
const MIN_COLUMNS: f32 = 24.0;
const MIN_LINES: f32 = 4.0;
/// A text row's height as a share of the text size.
const LINE_HEIGHT_SHARE: f32 = 1.3;

craft_ui::words! {
    /// The notes pane's words (PowerPoint's).
    #[non_exhaustive]
    pub struct NotesLabels {
        notes: "notes.notes" = "Notes";
        placeholder: "notes.placeholder" = "Click to add notes";
    }
}

/// A run of notes text with its formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesRun {
    /// The run's text.
    pub text: String,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
    /// Strikethrough.
    pub strikethrough: bool,
    /// `None`: the editor's text colour.
    pub color: Option<Color32>,
}

/// One paragraph: its runs, list level and marker, and its margins (points; `None` takes
/// [`crate::lists::LEVEL_INDENT`] per level).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesParagraph {
    /// The runs of the paragraph, in order.
    pub runs: Vec<NotesRun>,
    /// List level; 0 is the outermost.
    pub level: u8,
    /// The list marker, if the paragraph is a list item.
    pub marker: Option<ListMarker>,
    /// Start-edge margin of the lines after the first, in points.
    pub margin: Option<f64>,
    /// First-line indent in points, added to `margin`; negative hangs the first line.
    pub indent: Option<f64>,
}

/// The notes as the editor sees them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesText {
    /// The paragraphs, in order.
    pub paragraphs: Vec<NotesParagraph>,
}

impl NotesText {
    /// Plain paragraphs, one per line of `text`.
    pub(super) fn from_plain(text: &str) -> NotesText {
        let paragraph = |line: &str| NotesParagraph {
            runs: if line.is_empty() { Vec::new() } else { vec![NotesRun { text: line.to_owned(), ..NotesRun::default() }] },
            ..NotesParagraph::default()
        };
        NotesText { paragraphs: text.split('\n').map(paragraph).collect() }
    }

    /// The plain text, paragraphs joined with `\n` (what the text field edits).
    pub fn plain(&self) -> String {
        let paragraphs: Vec<String> = self.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect()).collect();
        paragraphs.join("\n")
    }

    /// The notes as they read with `text` typed in as if in place: only the span between the
    /// longest common prefix and suffix of [`plain`](Self::plain) and `text` changes (so one
    /// insertion, a deletion, or a selection replaced by typing all count), untouched text keeps
    /// its runs and paragraph settings, and the new text takes the formatting where it lands.
    /// Line endings in `text` are read as `\n` first. `None` when the two share no prefix and no
    /// suffix: the app then replaces the notes with plain paragraphs.
    pub fn typed(&self, text: &str) -> Option<NotesText> {
        let old: Vec<char> = self.plain().chars().collect();
        let new: Vec<char> = text.replace("\r\n", "\n").replace('\r', "\n").chars().collect();
        if old == new {
            return Some(self.clone());
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
#[non_exhaustive]
pub struct NotesEditorOutput {
    /// The editor's response.
    pub response: Response,
    /// The notes changed this pass: the app stores them (one undo step per typing run).
    pub changed: bool,
}

impl Outcome for NotesEditorOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The notes' look: the body type at the pane's zoom, the text and caret colours and the selection fill. Indents are
/// in document points and scale with the text size.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct NotesEditorStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The text colour.
    ink: StyleColor,
    /// The selection fill.
    selection: StyleColor,
    /// The size of the body text, points.
    body_size: f32,
    /// The size of the notes' default text in document points: indents scale as `body_size / points`.
    points: f32,
    /// The largest indent, points.
    max_indent: f32,
    /// The caret's width, as a share of the text size.
    caret_share: f32,
}

/// The position of the caret and the selection anchor in chars of [`NotesText::plain`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cursor {
    caret: usize,
    anchor: usize,
    /// The pass time of the last edit or move: the blink restarts from it.
    blink_start: f64,
}

/// The editor, built per pass over the app's notes.
///
/// - Text at the body type role × [`zoom`](NotesEditor::zoom), wrapped to the width; each paragraph laid out at its
///   indent, its marker ([`crate::lists::markers`]) drawn at its first line's start.
/// - An empty editor shows the placeholder; the editor is named by the notes word.
/// - Read-only (`Props::read_only`) it filters input to what changes no text and is the presenter's view.
/// - Bold, italic and underline toggles are the app's commands, not the editor's.
#[derive(craft_ui::contract::Component)]
#[component(content_painter)]
pub struct NotesEditor<'a> {
    #[common]
    common: Common,
    notes: &'a mut NotesText,
    zoom: f32,
    changed: bool,
}

impl<'a> NotesEditor<'a> {
    /// An editor named `label` over `notes`, shown at the body size.
    pub fn new(label: impl Into<Label>, notes: &'a mut NotesText) -> Self {
        NotesEditor { common: Common::new(), notes, zoom: 1.0, changed: false }.label(label)
    }

    /// The notes pane's zoom (PowerPoint's Zoom of the notes pane), a factor over the body type role's size; 1 by
    /// default.
    #[must_use]
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }
}

fn markers_of(notes: &NotesText) -> Vec<Option<String>> {
    let list: Vec<ListParagraph> = notes
        .paragraphs
        .iter()
        .map(|p| ListParagraph { level: p.level, marker: p.marker.clone(), empty: p.runs.iter().all(|r| r.text.is_empty()) })
        .collect();
    markers(&list)
}

/// The chars of paragraph `at` of `plain`'s lines.
fn paragraph_range(plain: &str, at: usize) -> Range<usize> {
    let mut start = 0;
    for (index, line) in plain.split('\n').enumerate() {
        let end = start + line.chars().count();
        if index == at {
            return start..end;
        }
        start = end + 1;
    }
    start..start
}

/// The paragraph holding char `c` and the char's offset within it.
fn locate(plain: &str, c: usize) -> (usize, usize) {
    let mut start = 0;
    for (index, line) in plain.split('\n').enumerate() {
        let end = start + line.chars().count();
        if c <= end {
            return (index, c - start);
        }
        start = end + 1;
    }
    (0, 0)
}

/// Replaces the chars `range` of `plain` with `with`.
fn splice(plain: &mut String, range: Range<usize>, with: &str) {
    let byte = |chars: usize, text: &str| text.char_indices().nth(chars).map_or(text.len(), |(b, _)| b);
    let (from, to) = (byte(range.start, plain), byte(range.end, plain));
    plain.replace_range(from..to, with);
}

impl NotesEditor<'_> {
    /// Applies one input to the notes' plain text and the cursor; returns whether the text changed.
    fn apply(&self, plain: &mut String, cursor: &mut Cursor, input: EditorInput, clip: Option<&str>) -> bool {
        let chars = plain.chars().count();
        cursor.caret = cursor.caret.min(chars);
        cursor.anchor = cursor.anchor.min(chars);
        let selected = cursor.caret.min(cursor.anchor)..cursor.caret.max(cursor.anchor);
        let insert = |plain: &mut String, cursor: &mut Cursor, text: &str| {
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            splice(plain, selected.clone(), &text);
            cursor.caret = selected.start + text.chars().count();
            cursor.anchor = cursor.caret;
            true
        };
        match input {
            EditorInput::Text(text) => insert(plain, cursor, &text),
            EditorInput::Newline => insert(plain, cursor, "\n"),
            EditorInput::Tab => insert(plain, cursor, "\t"),
            EditorInput::Edit(EditCommand::Paste) => clip.is_some_and(|text| insert(plain, cursor, text)),
            EditorInput::Edit(EditCommand::Cut) if !selected.is_empty() => insert(plain, cursor, ""),
            EditorInput::Edit(EditCommand::SelectAll) => {
                cursor.anchor = 0;
                cursor.caret = chars;
                false
            }
            EditorInput::Delete { forward, by, .. } => {
                let range = if !selected.is_empty() {
                    selected
                } else {
                    let at = cursor.caret;
                    let to = match (forward, by) {
                        (false, Granularity::Line | Granularity::Paragraph | Granularity::Document) => {
                            paragraph_range(plain, locate(plain, at).0).start
                        }
                        (false, Granularity::Word) => word_boundary(plain, at, false),
                        (false, _) => at.saturating_sub(1),
                        (true, Granularity::Unit) => (at + 1).min(chars),
                        (true, _) => word_boundary(plain, at, true),
                    };
                    to.min(at)..to.max(at)
                };
                if range.is_empty() {
                    return false;
                }
                splice(plain, range.clone(), "");
                cursor.caret = range.start;
                cursor.anchor = range.start;
                true
            }
            EditorInput::DeleteSurrounding { before, after, .. } => {
                let (from, to) = (selected.start.saturating_sub(before), (selected.end + after).min(chars));
                if from == selected.start && to == selected.end {
                    return false;
                }
                splice(plain, selected.end..to, "");
                splice(plain, from..selected.start, "");
                cursor.caret = from + selected.len();
                cursor.anchor = from;
                true
            }
            EditorInput::Move(step, mods) => {
                let (index, column) = locate(plain, cursor.caret);
                let to = match step {
                    NavMove::Back | NavMove::Forward => {
                        let forward = step == NavMove::Forward;
                        if mods.by != Granularity::Unit {
                            word_boundary(plain, cursor.caret, forward)
                        } else if !selected.is_empty() && !mods.extend {
                            if forward { selected.end } else { selected.start }
                        } else if forward {
                            (cursor.caret + 1).min(chars)
                        } else {
                            cursor.caret.saturating_sub(1)
                        }
                    }
                    NavMove::Up | NavMove::Down if matches!(mods.by, Granularity::Paragraph | Granularity::Document) => {
                        if step == NavMove::Up {
                            0
                        } else {
                            chars
                        }
                    }
                    NavMove::Up | NavMove::Down => {
                        let paragraphs = plain.split('\n').count();
                        let target = if step == NavMove::Up { index.checked_sub(1) } else { Some(index + 1).filter(|next| *next < paragraphs) };
                        target.map_or(cursor.caret, |target| {
                            let range = paragraph_range(plain, target);
                            range.start + column.min(range.len())
                        })
                    }
                    NavMove::Start | NavMove::End => {
                        if mods.by == Granularity::Document {
                            if step == NavMove::End { chars } else { 0 }
                        } else {
                            let range = paragraph_range(plain, index);
                            if step == NavMove::End { range.end } else { range.start }
                        }
                    }
                    _ => cursor.caret,
                };
                cursor.caret = to;
                if !mods.extend {
                    cursor.anchor = to;
                }
                false
            }
            EditorInput::Select { anchor, focus, .. } => {
                let at = |pos: TextPos| {
                    let range = paragraph_range(plain, pos.line);
                    let line: String = plain.chars().skip(range.start).take(range.len()).collect();
                    range.start + line.get(..pos.byte.min(line.len())).map_or(0, |head| head.chars().count())
                };
                cursor.anchor = at(anchor);
                cursor.caret = at(focus);
                false
            }
            _ => false,
        }
    }
}

impl Component for NotesEditor<'_> {
    const ROLE: Role = Role::MULTILINE_TEXT_INPUT;
    const INPUT: craft_ui::contract::Input = craft_ui::contract::Input::Editor;
    const KEYS: KeyUse = KeyUse::CODE;
    type Style = NotesEditorStyle;
    type Output = NotesEditorOutput;

    fn style(cx: &StyleCx<'_>, _: NotesEditorStyleKey) -> NotesEditorStyle {
        NotesEditorStyle {
            metrics: Metrics::new(),
            ink: cx.color(cx.tokens.color.text.normal),
            selection: cx.color(cx.tokens.color.state.text_selection),
            body_size: cx.tokens.typography.body.size,
            points: cx.tokens.typography.caption.size,
            max_indent: MAX_INDENT,
            caret_share: CARET_SHARE,
        }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let style = cx.style;
        let size = style.body_size * self.zoom;
        let width = size * MIN_COLUMNS / 2.0;
        let room = cx.room.unwrap_or(width);
        let scale = size / style.points;
        let font = FontRef::proportional();
        let ink = DataColor::new(style.ink.egui());
        let height: f32 = self
            .notes
            .paragraphs
            .iter()
            .map(|paragraph| {
                let (_, margin) = line_starts(paragraph.level, paragraph.margin, paragraph.indent);
                let runs = paragraph_runs(paragraph, &font, size, ink);
                let mut text = DocText::new(&runs);
                text.wrap = Some((room - indent(style, margin, scale)).max(0.0));
                cx.layout_doc(&text).size().y.max(size)
            })
            .sum();
        Vec2::new(width, height.max(size * MIN_LINES * LINE_HEIGHT_SHARE))
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.editor(EDITOR, rect, EditorPurpose::Text);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let inputs: Vec<EditorInput> = cx.editor_input().collect();
        if inputs.is_empty() {
            return;
        }
        let clip = if inputs.iter().any(|input| matches!(input, EditorInput::Edit(EditCommand::Paste))) {
            match cx.clipboard().paste() {
                Pending::Ready(content) => content.text.as_ref().map(|text| text.to_string()),
                _ => None,
            }
        } else {
            None
        };
        let now = cx.now();
        let mut cursor = *cx.memory::<Cursor>();
        let mut plain = self.notes.plain();
        let mut copied = None;
        let mut text_changed = false;
        for input in inputs {
            if matches!(input, EditorInput::Edit(EditCommand::Copy | EditCommand::Cut)) {
                let range = cursor.caret.min(cursor.anchor)..cursor.caret.max(cursor.anchor);
                copied = Some(plain.chars().skip(range.start).take(range.len()).collect::<String>()).filter(|text| !text.is_empty());
            }
            text_changed |= self.apply(&mut plain, &mut cursor, input, clip.as_deref());
        }
        if text_changed {
            *self.notes = self.notes.typed(&plain).unwrap_or_else(|| NotesText::from_plain(&plain));
            self.changed = true;
        }
        cursor.blink_start = now;
        *cx.memory::<Cursor>() = cursor;
        if let Some(text) = copied {
            cx.clipboard().put(&[ClipItem::Text(text.into())]);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let painter = ui.content_painter(cx);
        let style = &cx.style;
        let area = cx.content_rect;
        let size = style.body_size * self.zoom;
        let scale = size / style.points;
        let ink_color = style.ink.egui();
        let ink = DataColor::new(ink_color);
        let cursor = cx.recall::<Cursor>();
        let plain = self.notes.plain();
        let chars = plain.chars().count();
        let selected = cursor.caret.min(cursor.anchor).min(chars)..cursor.caret.max(cursor.anchor).min(chars);
        let marks = markers_of(self.notes);
        let font = FontRef::proportional();
        let caret_width = (size * style.caret_share).max(1.0);

        let mut y = area.top();
        let mut at = 0usize;
        let mut editor_lines: Vec<(String, Rect, f32)> = Vec::new();
        let mut caret_rect = Rect::from_min_size(area.min, Vec2::new(0.0, size));
        for (paragraph, marker) in self.notes.paragraphs.iter().zip(&marks) {
            let (first, margin) = line_starts(paragraph.level, paragraph.margin, paragraph.indent);
            let (first, margin) = (indent(style, first, scale), indent(style, margin, scale));
            let origin = area.left() + margin;
            let runs = paragraph_runs(paragraph, &font, size, ink);
            let mut text = DocText::new(&runs);
            text.wrap = Some((area.width() - (origin - area.left())).max(0.0));
            let layout = painter.layout(&text);
            let length: usize = paragraph.runs.iter().map(|run| run.text.chars().count()).sum();
            let range = at..at + length;
            let top_left = Pos2::new(origin, y);
            // Selection bands under the text, one per row the selection touches.
            if !selected.is_empty() && selected.start <= range.end && selected.end >= range.start {
                let (from, to) = (selected.start.max(range.start) - range.start, selected.end.min(range.end) - range.start);
                let a = layout.galley.pos_from_cursor(egui::text::CCursor::new(from));
                let b = layout.galley.pos_from_cursor(egui::text::CCursor::new(to));
                let band = Rect::from_min_max(top_left + a.min.to_vec2(), top_left + b.max.to_vec2());
                painter.painter.rect_filled(band, 0.0, style.selection.egui());
            }
            painter.painter.galley(top_left, layout.galley.clone(), ink_color);
            if let Some(marker) = marker {
                let marker_run = [DocRun::new(marker, font.clone(), size, ink)];
                let laid = painter.layout(&DocText::new(&marker_run));
                painter.painter.galley(Pos2::new(area.left() + first.min(margin), y), laid.galley, ink_color);
            }
            if (range.start..=range.end).contains(&cursor.caret.min(chars)) {
                let cursor_pos = layout.galley.pos_from_cursor(egui::text::CCursor::new(cursor.caret.min(chars) - range.start));
                caret_rect = Rect::from_min_max(top_left + cursor_pos.min.to_vec2(), top_left + cursor_pos.max.to_vec2());
            }
            let height = layout.size().y;
            editor_lines.push((
                paragraph.runs.iter().map(|run| run.text.as_str()).collect(),
                Rect::from_min_size(top_left, Vec2::new(layout.size().x, height)),
                top_left.y + layout.first_baseline,
            ));
            y += height.max(size);
            at = range.end + 1;
        }
        if self.notes.paragraphs.iter().all(|p| p.runs.iter().all(|r| r.text.is_empty())) && !cx.is_focused() {
            let words = cx.words::<NotesLabels>();
            let hint = [DocRun::new(words.placeholder.as_str(), font.clone(), size, DataColor::new(ink_color.gamma_multiply(0.55)))];
            let laid = painter.layout(&DocText::new(&hint));
            painter.painter.galley(area.min, laid.galley, ink_color);
        }
        if cx.is_focused() && selected.is_empty() && cx.blink(cursor.blink_start) {
            painter.painter.line_segment([caret_rect.center_top(), caret_rect.center_bottom()], Stroke::new(caret_width, ink_color));
        }
        let runs: Vec<[EditorRun<'_>; 1]> = editor_lines.iter().map(|(text, _, _)| [EditorRun::new(0..text.len(), false, &[], &[])]).collect();
        let lines: Vec<EditorLine<'_>> =
            editor_lines.iter().zip(&runs).map(|((text, rect, baseline), runs)| EditorLine::new(text, *rect, *baseline, runs)).collect();
        let position = |c: usize| {
            let (line, column) = locate(&plain, c.min(chars));
            let text = editor_lines.get(line).map_or("", |l| l.0.as_str());
            TextPos::new(line, text.char_indices().nth(column).map_or(text.len(), |(b, _)| b))
        };
        let mut state = EditorState::new(caret_rect, &lines, position(cursor.caret), revision(&plain));
        state.selection = (!selected.is_empty()).then(|| (position(cursor.anchor), position(cursor.caret)));
        cx.editor_state(state);
    }

    fn finish(self, cx: Cx<'_, Self>) -> NotesEditorOutput {
        NotesEditorOutput { response: cx.response.clone(), changed: self.changed }
    }
}

/// The indent of `points` document points at `scale`, within the largest indent.
fn indent(style: &NotesEditorStyle, points: f64, scale: f32) -> f32 {
    if points.is_finite() { points.clamp(0.0, f64::from(style.max_indent)) as f32 * scale } else { 0.0 }
}

/// The runs of `paragraph` at `size`: one document run per styled run, or one empty run for an empty paragraph so
/// it keeps a line.
fn paragraph_runs<'a>(paragraph: &'a NotesParagraph, font: &FontRef, size: f32, ink: DataColor) -> Vec<DocRun<'a>> {
    let runs: Vec<DocRun<'a>> = paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| {
            let mut doc = DocRun::new(&run.text, font.clone(), size, run.color.map_or(ink, DataColor::new));
            if run.italic {
                doc = doc.italic();
            }
            if run.bold {
                doc = doc.bold();
            }
            if run.underline {
                doc = doc.underline();
            }
            if run.strikethrough {
                doc = doc.strikethrough();
            }
            doc
        })
        .collect();
    if runs.is_empty() { vec![DocRun::new("", font.clone(), size, ink)] } else { runs }
}

/// A number that changes with the notes' text, for the input method's revision.
fn revision(plain: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::hash::DefaultHasher::new();
    plain.hash(&mut hasher);
    hasher.finish()
}

/// The caret's width as a share of the text size.
const CARET_SHARE: f32 = 0.06;
/// The part key of the editor part.
const EDITOR: craft_behavior::PartKey = craft_behavior::PartKey::new("editor");
