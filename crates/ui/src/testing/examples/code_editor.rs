//! `CodeEditor`: a document editor on the editor contract (18.4, 20.6).
//!
//! The editor holds its document by `&mut` and applies `cx.editor_input()` to it in `update`, so the same pass's
//! `paint` shows each keystroke. `paint` lays out the visible lines, draws text, selection and caret, and reports the
//! caret, the lines with their runs and the revision through `cx.editor_state`, which feeds the IME candidate window
//! and AccessKit. The editor reads no raw keys and no text events.
//!
//! The example has no pointer caret placement, clipboard commands or undo: pointer placement and the clipboard
//! belong to the text UI family, and without a history `can_undo` is false, which sends undo to the app command.

use crate::contract::{Common, Component, Cx, Focus as _, MeasureCx, NoValue, Outcome, PaintUi, PartsCx, Role};
use crate::primitives::{EditorLine, EditorPurpose, EditorRun, EditorState, PaintShape, StrokeWidth, TextRun};
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, Ink, StyleColor, StyleCx, SurfaceLook, TypeRole};
use craft_behavior::{EditCommand, EditorInput, Granularity, Insets, Metrics, MoveMods, NavMove, PartKey, Sense, TextPos};
use egui::{CornerRadius, Pos2, Rect, Response, Vec2};
use std::borrow::Cow;

const EDITOR: PartKey = PartKey::new("editor");
/// The columns and lines the editor measures when nothing else sizes it.
const MIN_COLUMNS: f32 = 24.0;
const MIN_LINES: f32 = 4.0;
/// The text one character cell is measured from.
const CELL: &str = "m";
/// What Tab inserts.
const INDENT: &str = "    ";

/// The text of a code editor: lines, a caret, an optional selection anchor, the composition in progress and a
/// revision bumped on every change.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeDocument {
    lines: Vec<String>,
    caret: TextPos,
    anchor: Option<TextPos>,
    preedit: Option<String>,
    revision: u64,
    /// The first line the last paint showed; positions AccessKit sends are relative to it.
    first: usize,
}

fn order(a: TextPos, b: TextPos) -> (TextPos, TextPos) {
    if (a.line, a.byte) <= (b.line, b.byte) { (a, b) } else { (b, a) }
}

/// The byte offset of the word start before `byte`: whitespace is skipped first, then one run of word characters.
fn word_start(line: &str, byte: usize) -> usize {
    let head = &line[..byte];
    let trimmed = head.trim_end().len();
    head[..trimmed].trim_end_matches(|c: char| c.is_alphanumeric() || c == '_').len()
}

/// The byte offset of the word end after `byte`.
fn word_end(line: &str, byte: usize) -> usize {
    let tail = &line[byte..];
    let skipped = tail.len() - tail.trim_start().len();
    let rest = &tail[skipped..];
    byte + skipped + (rest.len() - rest.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_').len())
}

impl Default for CodeDocument {
    fn default() -> Self {
        CodeDocument::new("")
    }
}

impl CodeDocument {
    /// A document of `text` with the caret at its end.
    pub fn new(text: &str) -> Self {
        let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
        let caret = TextPos::new(lines.len() - 1, lines[lines.len() - 1].len());
        CodeDocument { lines, caret, anchor: None, preedit: None, revision: 0, first: 0 }
    }

    /// The whole text.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// The caret.
    pub fn caret(&self) -> TextPos {
        self.caret
    }

    /// The selection as anchor and caret, when it covers anything.
    pub fn selection(&self) -> Option<(TextPos, TextPos)> {
        self.anchor.filter(|anchor| *anchor != self.caret).map(|anchor| (anchor, self.caret))
    }

    /// Bumped on every change to the text.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn end(&self) -> TextPos {
        TextPos::new(self.lines.len() - 1, self.lines[self.lines.len() - 1].len())
    }

    /// `pos` moved to the nearest position the text has.
    fn clamped(&self, pos: TextPos) -> TextPos {
        let line = pos.line.min(self.lines.len() - 1);
        let text = &self.lines[line];
        let mut byte = pos.byte.min(text.len());
        while !text.is_char_boundary(byte) {
            byte -= 1;
        }
        TextPos::new(line, byte)
    }

    fn delete_range(&mut self, from: TextPos, to: TextPos) {
        if from.line == to.line {
            self.lines[from.line].replace_range(from.byte..to.byte, "");
        } else {
            let tail = self.lines[to.line][to.byte..].to_owned();
            self.lines[from.line].truncate(from.byte);
            self.lines[from.line].push_str(&tail);
            self.lines.drain(from.line + 1..=to.line);
        }
        self.caret = from;
        self.anchor = None;
        self.revision += 1;
    }

    fn delete_selection(&mut self) -> bool {
        let Some((anchor, caret)) = self.selection() else { return false };
        let (from, to) = order(anchor, caret);
        self.delete_range(from, to);
        true
    }

    fn insert(&mut self, text: &str) {
        self.delete_selection();
        for (at, part) in text.split('\n').enumerate() {
            if at > 0 {
                let tail = self.lines[self.caret.line].split_off(self.caret.byte);
                self.lines.insert(self.caret.line + 1, tail);
                self.caret = TextPos::new(self.caret.line + 1, 0);
            }
            self.lines[self.caret.line].insert_str(self.caret.byte, part);
            self.caret.byte += part.len();
        }
        self.revision += 1;
    }

    /// The position one character or word from the caret toward the end or the start, crossing line breaks;
    /// `None` at the ends of the document.
    fn step_target(&self, forward: bool, word: bool) -> Option<TextPos> {
        let TextPos { line, byte } = self.caret;
        let text = &self.lines[line];
        if forward {
            if byte < text.len() {
                let next = if word { word_end(text, byte) } else { byte + text[byte..].chars().next().map_or(0, char::len_utf8) };
                Some(TextPos::new(line, next))
            } else {
                (line + 1 < self.lines.len()).then(|| TextPos::new(line + 1, 0))
            }
        } else if byte > 0 {
            let previous = if word { word_start(text, byte) } else { text[..byte].char_indices().next_back().map_or(0, |(at, _)| at) };
            Some(TextPos::new(line, previous))
        } else {
            line.checked_sub(1).map(|above| TextPos::new(above, self.lines[above].len()))
        }
    }

    /// The position in the line above or below with the caret's column.
    fn vertical_target(&self, down: bool) -> TextPos {
        let TextPos { line, byte } = self.caret;
        let target = if down { Some(line + 1).filter(|below| *below < self.lines.len()) } else { line.checked_sub(1) };
        let Some(target) = target else { return self.caret };
        let column = self.lines[line][..byte].chars().count();
        let text = &self.lines[target];
        TextPos::new(target, text.char_indices().nth(column).map_or(text.len(), |(at, _)| at))
    }

    fn delete(&mut self, forward: bool, by: Granularity) {
        if self.delete_selection() {
            return;
        }
        if let Some(target) = self.step_target(forward, by == Granularity::Word) {
            let (from, to) = order(self.caret, target);
            self.delete_range(from, to);
        }
    }

    fn move_by(&mut self, step: NavMove, mods: MoveMods) {
        let at = self.caret;
        let whole = mods.by == Granularity::Document;
        let to = match step {
            NavMove::Back => self.step_target(false, mods.by == Granularity::Word).unwrap_or(at),
            NavMove::Forward => self.step_target(true, mods.by == Granularity::Word).unwrap_or(at),
            NavMove::Up => self.vertical_target(false),
            NavMove::Down => self.vertical_target(true),
            NavMove::Start if whole => TextPos::new(0, 0),
            NavMove::Start => TextPos::new(at.line, 0),
            NavMove::End if whole => self.end(),
            NavMove::End => TextPos::new(at.line, self.lines[at.line].len()),
            _ => at,
        };
        if mods.extend {
            self.anchor.get_or_insert(at);
        } else {
            self.anchor = None;
        }
        self.caret = to;
    }

    /// Replaces the whole text and puts the caret at its end.
    pub(crate) fn set_text(&mut self, text: &str) {
        let revision = self.revision + 1;
        *self = CodeDocument { revision, ..CodeDocument::new(text) };
    }

    /// Applies one input; returns whether the text changed.
    pub(crate) fn apply(&mut self, input: EditorInput) -> bool {
        let was = self.revision;
        match input {
            EditorInput::Text(text) => {
                self.preedit = None;
                self.insert(&text);
            }
            EditorInput::Newline => self.insert("\n"),
            EditorInput::Tab => self.insert(INDENT),
            EditorInput::Delete { forward, by, .. } => self.delete(forward, by),
            EditorInput::DeleteSurrounding { before, after, .. } => {
                for _ in 0..before {
                    self.delete(false, Granularity::Unit);
                }
                for _ in 0..after {
                    self.delete(true, Granularity::Unit);
                }
            }
            EditorInput::Move(step, mods) => self.move_by(step, mods),
            EditorInput::Select { anchor, focus, .. } => {
                let first = self.first;
                self.anchor = Some(self.clamped(TextPos::new(anchor.line + first, anchor.byte)));
                self.caret = self.clamped(TextPos::new(focus.line + first, focus.byte));
            }
            EditorInput::Preedit { text, .. } => self.preedit = Some(text.to_string()).filter(|text| !text.is_empty()),
            EditorInput::ImeCommit => self.preedit = None,
            EditorInput::Edit(EditCommand::SelectAll) => {
                self.anchor = Some(TextPos::new(0, 0));
                self.caret = self.end();
            }
            EditorInput::Cancel => self.anchor = None,
            _ => {}
        }
        self.revision != was
    }

    /// The caret as positioned in the shown text, after the composition.
    pub(crate) fn caret_shown(&self) -> TextPos {
        TextPos::new(self.caret.line, self.caret.byte + self.preedit.as_ref().map_or(0, String::len))
    }

    /// The text of line `at` as shown: the composition sits at the caret.
    pub(crate) fn shown(&self, at: usize) -> Cow<'_, str> {
        match &self.preedit {
            Some(preedit) if at == self.caret.line => {
                let line = &self.lines[at];
                Cow::Owned(format!("{}{preedit}{}", &line[..self.caret.byte], &line[self.caret.byte..]))
            }
            _ => Cow::Borrowed(&self.lines[at]),
        }
    }
}

/// The style of the editor: the field's look, the control padding, the text and selection colours.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CodeStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    ink: StyleColor,
    selection: StyleColor,
}

/// What the editor returns.
#[non_exhaustive]
pub struct CodeEditorOutput {
    /// The editor's response.
    pub response: Response,
    /// The text changed in this pass.
    pub edited: bool,
}

impl Outcome for CodeEditorOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A multi-line code editor over a [`CodeDocument`].
#[derive(crate::contract::Component)]
pub struct CodeEditor<'a> {
    #[common]
    common: Common,
    doc: &'a mut CodeDocument,
    edited: bool,
}

impl<'a> CodeEditor<'a> {
    /// An editor of `doc`.
    pub fn new(doc: &'a mut CodeDocument) -> Self {
        CodeEditor { common: Common::new(), doc, edited: false }
    }
}

/// The x offset of byte `byte` of `text` in a line laid out as `runs`; code reads left to right.
pub(crate) fn caret_x(runs: &[EditorRun<'_>], text: &str, byte: usize) -> f32 {
    let Some(run) = runs.iter().find(|run| run.bytes.contains(&byte)).or(runs.last()) else { return 0.0 };
    let chars = text[run.bytes.start..byte.max(run.bytes.start)].chars().count();
    match (run.positions.get(chars), run.positions.last().zip(run.widths.last())) {
        (Some(x), _) => *x,
        (None, Some((x, width))) => x + width,
        _ => 0.0,
    }
}

impl Component for CodeEditor<'_> {
    const ROLE: Role = Role::MULTILINE_TEXT_INPUT;
    const INPUT: crate::contract::Input = crate::contract::Input::Editor;
    const KEYS: KeyUse = KeyUse::CODE;
    type Style = CodeStyle;
    type Output = CodeEditorOutput;

    fn style(cx: &StyleCx<'_>, _: CodeEditorStyleKey) -> CodeStyle {
        CodeStyle {
            surface: cx.field(),
            metrics: Metrics::new().padding(Insets::uniform(cx.control().pad_x)),
            ink: cx.ink(Ink::OnPanel),
            selection: cx.color(cx.tokens.color.state.text_selection),
        }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let cell = cx.layout(&TextRun::new(CELL).role(TypeRole::Mono)).size();
        Vec2::new(cell.x * MIN_COLUMNS, cell.y * MIN_LINES)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.editor(EDITOR, rect, EditorPurpose::Code);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let inputs: Vec<EditorInput> = cx.editor_input().collect();
        for input in inputs {
            self.edited |= self.doc.apply(input);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let draw = ui.draw();
        let area = cx.content_rect;
        let mono = |text: &str| TextRun::new(text.to_owned()).role(TypeRole::Mono);
        let line_height = draw.layout(&mono(CELL), f32::INFINITY).size().y;
        let rows = ((area.height() / line_height).floor() as usize).max(1);
        let caret = self.doc.caret;
        let first = (caret.line + 1).saturating_sub(rows);
        self.doc.first = first;
        let last = (first + rows).min(self.doc.lines.len());
        let layouts: Vec<_> = (first..last).map(|at| draw.layout(&mono(&self.doc.shown(at)), area.width())).collect();
        let runs: Vec<Vec<EditorRun<'_>>> = layouts.iter().map(|layout| layout.editor_runs()).collect();
        let shown: Vec<Cow<'_, str>> = (first..last).map(|at| self.doc.shown(at)).collect();
        let rects: Vec<Rect> = (0..layouts.len())
            .map(|row| Rect::from_min_size(area.min + Vec2::new(0.0, row as f32 * line_height), Vec2::new(area.width(), line_height)))
            .collect();
        let selection = self.doc.selection().map(|(anchor, head)| order(anchor, head));
        for row in 0..layouts.len() {
            let at = first + row;
            if let Some((from, to)) = selection.filter(|(from, to)| (from.line..=to.line).contains(&at)) {
                let start = if at == from.line { caret_x(&runs[row], &shown[row], from.byte) } else { 0.0 };
                let end = if at == to.line { caret_x(&runs[row], &shown[row], to.byte) } else { layouts[row].size().x };
                let band = Rect::from_min_max(
                    Pos2::new(rects[row].left() + start, rects[row].top()),
                    Pos2::new(rects[row].left() + end, rects[row].bottom()),
                );
                draw.fill(&PaintShape::Rect(band, CornerRadius::ZERO), cx.style.selection);
            }
            draw.galley(rects[row].min, &layouts[row]);
        }
        let shown_caret = TextPos::new(caret.line - first, self.doc.caret_shown().byte);
        let x = caret_x(&runs[shown_caret.line], &shown[shown_caret.line], shown_caret.byte);
        let top = rects[shown_caret.line].min + Vec2::new(x, 0.0);
        let caret_rect = Rect::from_min_size(top, Vec2::new(0.0, line_height));
        if cx.is_focused() {
            draw.stroke(&PaintShape::Path(vec![top, top + Vec2::new(0.0, line_height)]), StrokeWidth::Medium, cx.style.ink);
        }
        let lines: Vec<EditorLine<'_>> = (0..layouts.len())
            .map(|row| EditorLine::new(&shown[row], rects[row], rects[row].top() + layouts[row].first_baseline, &runs[row]))
            .collect();
        let mut state = EditorState::new(caret_rect, &lines, shown_caret, self.doc.revision);
        state.selection = self.doc.selection().map(|(anchor, head)| {
            let shown_at = |pos: TextPos| TextPos::new(pos.line.saturating_sub(first), pos.byte);
            (shown_at(anchor), shown_at(head))
        });
        cx.editor_state(state);
    }

    fn finish(self, cx: Cx<'_, Self>) -> CodeEditorOutput {
        CodeEditorOutput { response: cx.response.clone(), edited: self.edited }
    }
}
