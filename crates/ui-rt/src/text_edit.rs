//! The text editing engine: one editor state over a string that the toolkit's text fields and
//! editors share. It applies craft-behavior's ordered [`EditorInput`] (typing, input method
//! preedits and commits, moves and deletes by the platform's chords, edit commands), the pointer
//! (click counts select a word or a line, a drag selects and scrolls at the view's edge), undo and
//! redo through [`EditHistory`] with typing coalesced into one step, masking and a character
//! limit. Layout and hit-testing go through the backend's [`crate::TextEngine`] ([`EditLayout`]); the
//! caret, selection and preedit underline are recorded once per build ([`EditPaint`]) and the
//! caret blinks at paint time ([`PaintCx::time_until`]), so a blink repaints and builds nothing.
//!
//! The state lives in the owning node's memory and runs inside the owning node's run: there is no
//! node per caret. Positions are byte offsets into the text, always on character boundaries.

use std::ops::Range;
use std::sync::Arc;

use craft_behavior::{EditCommand, EditEvent, EditHistory, EditorInput, GestureId, Granularity, Mixed, NavMove};
use craft_host::target::Os;
use craft_theme::Rgba;
use emath::{Pos2, Rect, Vec2};

use crate::paint::PaintCx;
use crate::text::{TextLayout, TextMetrics};

/// How long a blinking caret shows or hides its stroke, in seconds: the platforms' usual rate.
pub const CARET_HALF_PERIOD: f64 = 0.53;

/// The character a masked text shows for each of its characters.
pub const MASK: char = '\u{2022}';

/// What an editor accepts, set by its owner each run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditConfig {
    /// Line breaks are text and Up and Down move between rows; a single line drops typed breaks.
    pub multiline: bool,
    /// The text shows as [`MASK`] characters, copies nothing and cuts nothing.
    pub mask: bool,
    /// The most characters the text takes; typing and pasting stop there.
    pub limit: Option<usize>,
    /// Only inputs that change no text apply.
    pub read_only: bool,
    /// Word moves follow this platform: on macOS a forward word move stops at the word's end,
    /// elsewhere at the next word's start.
    pub os: Os,
}

impl EditConfig {
    /// A single editable line on `os`.
    pub const fn line(os: Os) -> EditConfig {
        EditConfig { multiline: false, mask: false, limit: None, read_only: false, os }
    }
}

/// The input method's composition, shown at the caret and not yet part of the text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preedit {
    pub text: String,
    /// The input method's cursor, as a byte range of `text`.
    pub cursor: Option<Range<usize>>,
}

/// What one input did; the owner turns it into its value, the clipboard and the platform.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub text_changed: bool,
    pub caret_moved: bool,
    /// Text for the clipboard (Copy, or Cut of an unmasked selection).
    pub copy: Option<String>,
    /// Paste asked for: the owner requests the clipboard's text and hands it to [`TextEdit::paste`].
    pub paste: bool,
    /// An Undo or Redo the editor has no step for: the owner passes it on to the app's command.
    pub unhandled: Option<EditCommand>,
    /// A character was dropped (the limit, a refused paste).
    pub refused: bool,
}

/// What a press and the drag after it select by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Unit {
    #[default]
    Char,
    Word,
    Line,
}

/// The pointer's selection under way.
#[derive(Clone, Debug, PartialEq)]
struct Selecting {
    unit: Unit,
    /// The unit the press went down in: the selection always covers it.
    origin: Range<usize>,
}

/// The kind of the open undo step: typing coalesces with typing, deleting with deleting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunKind {
    Typing,
    Deleting,
}

/// One editor's state.
#[derive(Clone, Debug, Default)]
pub struct TextEdit {
    caret: usize,
    anchor: usize,
    preedit: Option<Preedit>,
    /// The x a run of Up and Down moves keeps, from where it started.
    pref_x: Option<f32>,
    /// How far the text is scrolled inside its view: x for a single line, y for several.
    scroll: Vec2,
    history: Option<EditHistory<String>>,
    /// The open undo step: its kind, gesture and the text before it.
    open: Option<(RunKind, GestureId, String)>,
    gestures: u64,
    selecting: Option<Selecting>,
    /// The frame time the caret last moved or text was typed: the blink restarts there.
    moved_at: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Punct,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

fn boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn prev_char(text: &str, at: usize) -> usize {
    text[..at].char_indices().next_back().map_or(0, |(i, _)| i)
}

fn next_char(text: &str, at: usize) -> usize {
    text[at..].chars().next().map_or(text.len(), |c| at + c.len_utf8())
}

/// The start of the word before `at`: back over spaces, then over one class of characters.
fn word_back(text: &str, at: usize) -> usize {
    let mut chars = text[..at].char_indices().rev().peekable();
    while chars.next_if(|(_, c)| class(*c) == Class::Space).is_some() {}
    let Some(&(mut start, first)) = chars.peek() else { return 0 };
    let kind = class(first);
    while let Some((i, _)) = chars.next_if(|(_, c)| class(*c) == kind) {
        start = i;
    }
    start
}

/// Where a forward word move from `at` stops: on macOS the end of the next word (over spaces, then
/// over the word), elsewhere the start of the next word (over the word, then over spaces).
fn word_forward(text: &str, at: usize, os: Os) -> usize {
    let rest = &text[at..];
    let mut chars = rest.char_indices().peekable();
    let skip_spaces =
        |chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>| while chars.next_if(|(_, c)| class(*c) == Class::Space).is_some() {};
    let skip_class = |chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>| {
        if let Some(&(_, first)) = chars.peek() {
            let kind = class(first);
            if kind != Class::Space {
                while chars.next_if(|(_, c)| class(*c) == kind).is_some() {}
            }
        }
    };
    if os == Os::Mac {
        skip_spaces(&mut chars);
        skip_class(&mut chars);
    } else {
        skip_class(&mut chars);
        skip_spaces(&mut chars);
    }
    chars.peek().map_or(text.len(), |(i, _)| at + i)
}

/// The word (or run of spaces or punctuation) around `at`.
fn word_at(text: &str, at: usize) -> Range<usize> {
    let Some(c) = text[at..].chars().next().or_else(|| text[..at].chars().next_back()) else { return at..at };
    let kind = class(c);
    let start = text[..at].char_indices().rev().take_while(|(_, c)| class(*c) == kind).last().map_or(at, |(i, _)| i);
    let end = text[at..].char_indices().find(|(_, c)| class(*c) != kind).map_or(text.len(), |(i, _)| at + i);
    start..end
}

/// The hard line (between line breaks) around `at`, without its break.
fn line_at(text: &str, at: usize) -> Range<usize> {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    start..end
}

impl TextEdit {
    /// An editor over `text` with the caret at its end and an empty history.
    pub fn new(text: &str) -> TextEdit {
        TextEdit { caret: text.len(), anchor: text.len(), history: Some(EditHistory::new(&text.to_owned())), ..TextEdit::default() }
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Where the selection started: the end that stays while the caret moves.
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    /// The selected byte range; empty at the caret when nothing is selected.
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.preedit.as_ref()
    }

    /// The scroll offset of the text inside its view.
    pub fn scroll(&self) -> Vec2 {
        self.scroll
    }

    /// The frame time the caret last moved: the blink restarts there.
    pub fn moved_at(&self) -> f64 {
        self.moved_at
    }

    /// A pointer selection is under way (a press that has not come up).
    pub fn selecting(&self) -> bool {
        self.selecting.is_some()
    }

    /// Whether an undo, and a redo, would change the text.
    pub fn history(&self) -> (bool, bool) {
        let open = self.open.is_some();
        self.history.as_ref().map_or((open, false), |h| (open || h.can_undo(), !open && h.can_redo()))
    }

    /// Selects `anchor..caret` of `text` (bytes, moved onto character boundaries).
    pub fn select(&mut self, text: &str, anchor: usize, caret: usize) {
        self.anchor = boundary(text, anchor);
        self.caret = boundary(text, caret);
        self.pref_x = None;
    }

    pub fn select_all(&mut self, text: &str) {
        self.select(text, 0, text.len());
    }

    /// The text changed outside the editor (the app wrote its value): the caret and anchor stay
    /// inside it, the composition ends and the history starts again from `text`.
    pub fn reset(&mut self, text: &str) {
        self.caret = boundary(text, self.caret);
        self.anchor = boundary(text, self.anchor);
        self.preedit = None;
        self.open = None;
        self.history = Some(EditHistory::new(&text.to_owned()));
    }

    /// Ends the composition without inserting it (an Escape over it, a refused edit).
    pub fn end_preedit(&mut self) {
        self.preedit = None;
    }

    /// Notes the frame time of a caret move, where the blink restarts.
    pub fn moved(&mut self, now: f64) {
        self.moved_at = now;
    }

    fn history_mut(&mut self, text: &str) -> &mut EditHistory<String> {
        self.history.get_or_insert_with(|| EditHistory::new(&text.to_owned()))
    }

    /// Closes the open undo step at `text`, the text after it.
    pub fn close_step(&mut self, text: &str) {
        if let Some((_, id, before)) = self.open.take() {
            self.history_mut(text).record(&EditEvent::commit(id, Mixed::Same(before), text.to_owned()));
        }
    }

    /// Opens (or continues) an undo step of `kind` before an edit of `text`.
    fn step(&mut self, kind: RunKind, text: &str) {
        if self.open.as_ref().is_some_and(|(open, ..)| *open == kind) {
            return;
        }
        self.close_step(text);
        self.gestures += 1;
        let id = GestureId::new(self.gestures);
        self.history_mut(text).record(&EditEvent::begin(id, Mixed::Same(text.to_owned()), Arc::from(""), None));
        self.open = Some((kind, id, text.to_owned()));
    }

    /// An edit that is one undo step by itself (paste, cut, a delete of a selection).
    fn single_step(&mut self, before: &str, after: &str) {
        self.close_step(before);
        self.history_mut(before).record(&EditEvent::set(Mixed::Same(before.to_owned()), after.to_owned(), Arc::from(""), None));
    }

    fn replace_selection(&mut self, text: &mut String, insert: &str) {
        let range = self.selection();
        text.replace_range(range.clone(), insert);
        self.caret = range.start + insert.len();
        self.anchor = self.caret;
        self.pref_x = None;
    }

    /// `insert` cut to what the limit leaves room for in `text` over the selection; the flag says
    /// whether characters were dropped.
    fn fitted<'s>(&self, text: &str, insert: &'s str, config: &EditConfig) -> (&'s str, bool) {
        let Some(limit) = config.limit else { return (insert, false) };
        let kept = text.chars().count() - text[self.selection()].chars().count();
        let room = limit.saturating_sub(kept);
        match insert.char_indices().nth(room) {
            Some((cut, _)) => (&insert[..cut], true),
            None => (insert, false),
        }
    }

    /// Types `typed` over the selection: one coalesced undo step with the typing around it. A
    /// single line drops line breaks and every editor drops other control characters.
    pub fn type_text(&mut self, text: &mut String, typed: &str, config: &EditConfig) -> Applied {
        let mut out = Applied::default();
        if config.read_only {
            return out;
        }
        self.preedit = None;
        let clean: String = typed.chars().filter(|c| !c.is_control() || (config.multiline && *c == '\n')).collect();
        let (insert, dropped) = self.fitted(text, &clean, config);
        out.refused = dropped;
        if insert.is_empty() {
            return out;
        }
        self.step(RunKind::Typing, text);
        self.replace_selection(text, insert);
        out.text_changed = true;
        out.caret_moved = true;
        out
    }

    /// Pastes `pasted` over the selection as one undo step; a single line makes each line break a
    /// space.
    pub fn paste(&mut self, text: &mut String, pasted: &str, config: &EditConfig) -> Applied {
        let mut out = Applied::default();
        if config.read_only {
            return out;
        }
        let pasted = if config.multiline { pasted.replace("\r\n", "\n") } else { pasted.replace("\r\n", " ").replace(['\n', '\r'], " ") };
        let clean: String = pasted.chars().filter(|c| !c.is_control() || *c == '\n').collect();
        let (insert, dropped) = self.fitted(text, &clean, config);
        out.refused = dropped;
        if insert.is_empty() && self.selection().is_empty() {
            return out;
        }
        let before = text.clone();
        self.replace_selection(text, insert);
        self.single_step(&before, text);
        out.text_changed = true;
        out.caret_moved = true;
        out
    }

    /// Replaces the whole text with `with` as one undo step, the caret at its end (a completion, a
    /// draft the owner puts in).
    pub fn replace_all(&mut self, text: &mut String, with: &str) {
        let before = text.clone();
        text.clear();
        text.push_str(with);
        self.caret = text.len();
        self.anchor = self.caret;
        self.preedit = None;
        self.single_step(&before, text);
    }

    /// Replaces `range` of `text` with `with` as one undo step, the caret after it.
    pub fn replace_range(&mut self, text: &mut String, range: Range<usize>, with: &str) {
        let before = text.clone();
        self.anchor = boundary(text, range.start);
        self.caret = boundary(text, range.end);
        self.replace_selection(text, with);
        self.single_step(&before, text);
    }

    /// Steps the history back (`redo` off) or forward; `false` when there is no step. The caret
    /// lands at the end of what the step changed.
    pub fn undo(&mut self, text: &mut String, redo: bool) -> bool {
        self.close_step(text);
        let before = text.clone();
        let step = if redo { craft_behavior::edit_history::Step::Redo } else { craft_behavior::edit_history::Step::Undo };
        if !self.history_mut(&before).step(text, step) {
            return false;
        }
        let same_head = before.bytes().zip(text.bytes()).take_while(|(a, b)| a == b).count();
        let same_tail = before.bytes().rev().zip(text.bytes().rev()).take_while(|(a, b)| a == b).count().min(text.len() - same_head.min(text.len()));
        let at = boundary(text, text.len() - same_tail);
        self.caret = at.max(boundary(text, same_head));
        self.anchor = self.caret;
        self.preedit = None;
        true
    }

    /// The offset a move of `mv` by `by` from the caret reaches.
    fn moved_to(&mut self, text: &str, mv: NavMove, by: Granularity, config: &EditConfig, geometry: Option<&Geometry<'_>>) -> usize {
        let at = self.caret;
        let row_move = |rows: isize, this: &mut Self| -> usize {
            let Some(g) = geometry.filter(|_| config.multiline) else {
                return if rows < 0 { 0 } else { text.len() };
            };
            let shown = g.layout.to_shown(at);
            let x = *this.pref_x.get_or_insert_with(|| g.layout.caret_rect(shown).min.x);
            let row = g.layout.row_of(shown) as isize + rows;
            if row < 0 {
                return 0;
            }
            if row as usize >= g.layout.rows.len() {
                return text.len();
            }
            g.layout.to_text(g.layout.hit_row(row as usize, x))
        };
        let keeps_x = matches!((mv, by), (NavMove::Up | NavMove::Down | NavMove::PageUp | NavMove::PageDown, _));
        if !keeps_x {
            self.pref_x = None;
        }
        let page = geometry.map_or(1, |g| g.page_rows()) as isize;
        match (mv, by) {
            (NavMove::Back, Granularity::Word) => word_back(text, at),
            (NavMove::Forward, Granularity::Word) => word_forward(text, at, config.os),
            (NavMove::Back, Granularity::Unit) => prev_char(text, at),
            (NavMove::Forward, Granularity::Unit) => next_char(text, at),
            (NavMove::Up, Granularity::Unit) => row_move(-1, self),
            (NavMove::Down, Granularity::Unit) => row_move(1, self),
            (NavMove::PageUp, _) => row_move(-page, self),
            (NavMove::PageDown, _) => row_move(page, self),
            (NavMove::Back | NavMove::Start, Granularity::Line) => self.row_edge(text, at, false, geometry),
            (NavMove::Forward | NavMove::End, Granularity::Line) => self.row_edge(text, at, true, geometry),
            (NavMove::Back | NavMove::Start, Granularity::Paragraph) => line_at(text, at).start,
            (NavMove::Forward | NavMove::End, Granularity::Paragraph) => line_at(text, at).end,
            (NavMove::Back | NavMove::Start | NavMove::Up, _) => 0,
            (NavMove::Forward | NavMove::End | NavMove::Down, _) => text.len(),
        }
    }

    /// The start or end of the row the caret is on: the laid-out row when there is a layout, else
    /// the hard line.
    fn row_edge(&self, text: &str, at: usize, end: bool, geometry: Option<&Geometry<'_>>) -> usize {
        match geometry {
            Some(g) => {
                let row = &g.layout.rows[g.layout.row_of(g.layout.to_shown(at))];
                g.layout.to_text(if end { row.end } else { row.start })
            }
            None if end => line_at(text, at).end,
            None => line_at(text, at).start,
        }
    }

    /// Applies one input to `text`. `geometry` is last build's layout and view, for row moves.
    pub fn apply(&mut self, text: &mut String, input: &EditorInput, config: &EditConfig, geometry: Option<&Geometry<'_>>) -> Applied {
        self.caret = boundary(text, self.caret);
        self.anchor = boundary(text, self.anchor);
        let before = (self.caret, self.anchor);
        if config.read_only && !input.is_read_only_safe() {
            return Applied::default();
        }
        let mut out = Applied::default();
        match input {
            EditorInput::Text(typed) => out = self.type_text(text, typed, config),
            EditorInput::Move(mv, mods) => {
                let selection = self.selection();
                let to = match (mv, mods.by) {
                    // A plain Left or Right over a selection collapses it to that side.
                    (NavMove::Back, Granularity::Unit) if !mods.extend && !selection.is_empty() => selection.start,
                    (NavMove::Forward, Granularity::Unit) if !mods.extend && !selection.is_empty() => selection.end,
                    (mv, by) => self.moved_to(text, *mv, by, config, geometry),
                };
                self.caret = to;
                if !mods.extend {
                    self.anchor = to;
                }
                self.close_step(text);
            }
            EditorInput::Delete { forward, by, .. } => {
                if self.caret != self.anchor {
                    let before = text.clone();
                    self.replace_selection(text, "");
                    self.single_step(&before, text);
                    out.text_changed = true;
                } else {
                    let mv = if *forward { NavMove::Forward } else { NavMove::Back };
                    let other = self.moved_to(text, mv, *by, config, geometry);
                    if other != self.caret {
                        self.step(RunKind::Deleting, text);
                        self.anchor = other;
                        self.replace_selection(text, "");
                        out.text_changed = true;
                    }
                }
            }
            EditorInput::DeleteSurrounding { before: b, after: a, unit } => {
                let chars_before = unit.chars_before(*b, &text[..self.caret]);
                let chars_after = unit.chars_after(*a, &text[self.caret..]);
                let start = text[..self.caret].char_indices().rev().take(chars_before).last().map_or(self.caret, |(i, _)| i);
                let end = text[self.caret..].char_indices().take(chars_after).last().map_or(self.caret, |(i, c)| self.caret + i + c.len_utf8());
                if start < end {
                    self.step(RunKind::Typing, text);
                    self.anchor = start;
                    self.caret = end;
                    self.replace_selection(text, "");
                    out.text_changed = true;
                }
            }
            EditorInput::Select { .. } => {}
            EditorInput::Preedit { text: composing, cursor } => {
                if composing.is_empty() {
                    self.preedit = None;
                } else {
                    // A composition starts at the selection's start and replaces the selection.
                    if self.preedit.is_none() && self.caret != self.anchor {
                        let before = text.clone();
                        self.replace_selection(text, "");
                        self.single_step(&before, text);
                        out.text_changed = true;
                    }
                    self.preedit = Some(Preedit { text: composing.to_string(), cursor: cursor.clone() });
                }
                out.caret_moved = true;
            }
            EditorInput::ImeCommit => self.preedit = None,
            EditorInput::Edit(command) => match command {
                EditCommand::SelectAll => self.select_all(text),
                EditCommand::Copy if !config.mask && self.caret != self.anchor => out.copy = Some(text[self.selection()].to_owned()),
                EditCommand::Cut if !config.mask && self.caret != self.anchor => {
                    out.copy = Some(text[self.selection()].to_owned());
                    if !config.read_only {
                        let before = text.clone();
                        self.replace_selection(text, "");
                        self.single_step(&before, text);
                        out.text_changed = true;
                    }
                }
                EditCommand::Delete if self.caret != self.anchor => {
                    let before = text.clone();
                    self.replace_selection(text, "");
                    self.single_step(&before, text);
                    out.text_changed = true;
                }
                EditCommand::Paste => out.paste = true,
                EditCommand::Undo | EditCommand::Redo => {
                    if self.undo(text, *command == EditCommand::Redo) {
                        out.text_changed = true;
                    } else {
                        out.unhandled = Some(*command);
                    }
                }
                _ => {}
            },
            EditorInput::Newline { .. } if config.multiline => out = self.type_text(text, "\n", config),
            _ => {}
        }
        out.caret_moved |= out.text_changed || before != (self.caret, self.anchor);
        out
    }

    /// A press at `pos` (the view's coordinates) with `clicks` its click count: one places the
    /// caret (extending the selection with `extend`), two select the word, three the line (the
    /// laid-out row, or the whole text of a single line). A press over a composition commits
    /// nothing here: the IME engine committed it before the press reached the run.
    pub fn press(&mut self, text: &str, geometry: &Geometry<'_>, pos: Pos2, clicks: u32, extend: bool) {
        self.close_step(text);
        let at = geometry.layout.to_text(geometry.hit(pos));
        let unit = match clicks {
            0 | 1 => Unit::Char,
            2 => Unit::Word,
            _ => Unit::Line,
        };
        let origin = self.unit_at(text, geometry, at, unit);
        self.pref_x = None;
        if extend && unit == Unit::Char {
            self.caret = at;
        } else {
            self.anchor = origin.start;
            self.caret = origin.end;
        }
        self.selecting = Some(Selecting { unit, origin });
    }

    fn unit_at(&self, text: &str, geometry: &Geometry<'_>, at: usize, unit: Unit) -> Range<usize> {
        match unit {
            Unit::Char => at..at,
            Unit::Word => word_at(text, at),
            Unit::Line => {
                let row = &geometry.layout.rows[geometry.layout.row_of(geometry.layout.to_shown(at))];
                geometry.layout.to_text(row.start)..geometry.layout.to_text(row.end)
            }
        }
    }

    /// The pointer moved to `pos` while the press is held: the selection runs from the press's
    /// unit to the unit under the pointer. Past the view's edge the text scrolls toward the
    /// pointer by the distance past the edge; returns whether it scrolled, so the owner asks for
    /// another build while the pointer stays out.
    pub fn drag(&mut self, text: &str, geometry: &Geometry<'_>, pos: Pos2, config: &EditConfig) -> bool {
        let Some(selecting) = self.selecting.clone() else { return false };
        let view = geometry.view;
        let past = if config.multiline {
            Vec2::new(0.0, if pos.y < view.min.y { pos.y - view.min.y } else { (pos.y - view.max.y).max(0.0) })
        } else {
            Vec2::new(if pos.x < view.min.x { pos.x - view.min.x } else { (pos.x - view.max.x).max(0.0) }, 0.0)
        };
        let scrolled = self.scroll_by(past, geometry);
        let at = geometry.layout.to_text(geometry.hit(pos));
        let here = self.unit_at(text, geometry, at, selecting.unit);
        if here.start < selecting.origin.start {
            self.anchor = selecting.origin.end;
            self.caret = here.start;
        } else {
            self.anchor = selecting.origin.start;
            self.caret = here.end.max(selecting.origin.end);
        }
        scrolled
    }

    /// The press came up or was cancelled.
    pub fn release(&mut self) {
        self.selecting = None;
    }

    /// Moves the scroll offset by `delta`, kept inside the text; returns whether it moved.
    pub fn scroll_by(&mut self, delta: Vec2, geometry: &Geometry<'_>) -> bool {
        let max = geometry.max_scroll();
        let next = (self.scroll + delta).clamp(Vec2::ZERO, max);
        let moved = next != self.scroll;
        self.scroll = next;
        moved
    }

    /// Scrolls just enough for the caret to show in the view, and keeps the offset inside the
    /// text.
    pub fn reveal(&mut self, geometry: &Geometry<'_>) {
        let caret = geometry.layout.caret_rect(geometry.layout.to_shown(self.caret)).translate(geometry.text_origin().to_vec2());
        let view = geometry.view;
        let mut delta = Vec2::ZERO;
        if caret.min.x < view.min.x {
            delta.x = caret.min.x - view.min.x;
        } else if caret.max.x > view.max.x {
            delta.x = caret.max.x - view.max.x;
        }
        if caret.min.y < view.min.y {
            delta.y = caret.min.y - view.min.y;
        } else if caret.max.y > view.max.y {
            delta.y = caret.max.y - view.max.y;
        }
        self.scroll_by(delta, geometry);
        self.scroll = self.scroll.clamp(Vec2::ZERO, geometry.max_scroll());
    }

    /// The text as laid out and drawn: masked, with the composition at the caret.
    pub fn shown(&self, text: &str, config: &EditConfig) -> Shown {
        let caret = boundary(text, self.caret);
        let masked = |s: &str| s.chars().map(|_| MASK).collect::<String>();
        let (head, tail) = (&text[..caret], &text[caret..]);
        let (head, tail) = if config.mask { (masked(head), masked(tail)) } else { (head.to_owned(), tail.to_owned()) };
        let preedit = self.preedit.as_ref().filter(|_| !config.mask);
        let mut shown = head.clone();
        let composing = preedit.map(|p| {
            shown.push_str(&p.text);
            head.len()..head.len() + p.text.len()
        });
        shown.push_str(&tail);
        let starts = config.mask.then(|| text.char_indices().map(|(i, _)| i).chain([text.len()]).collect());
        Shown { text: shown, preedit: composing, caret_at: caret, starts }
    }

    /// What the build records for the paint: the selection behind the text, the text, the
    /// composition's underline and the caret, at `geometry`'s place and scroll.
    pub fn paint(&self, layout_text: &TextLayout, geometry: &Geometry<'_>, look: EditLook) -> EditPaint {
        let origin = geometry.text_origin();
        let layout = geometry.layout;
        let shown = |at: usize| layout.to_shown(at);
        let selection = self.selection();
        let selection =
            if selection.is_empty() || !look.focused { Vec::new() } else { layout.range_rects(shown(selection.start)..shown(selection.end)) };
        let underline = layout.preedit.clone().map(|range| layout.range_rects(range));
        let caret = look.focused.then(|| {
            let at = match (&self.preedit, &layout.preedit) {
                (Some(p), Some(range)) => range.start + p.cursor.as_ref().map_or(p.text.len(), |c| c.end),
                _ => shown(self.caret),
            };
            let rect = layout.caret_rect(at);
            Rect::from_min_size(rect.min, Vec2::new(look.caret_width, rect.height()))
        });
        EditPaint {
            text: layout_text.clone(),
            origin,
            clip: geometry.view,
            selection,
            underline: underline.unwrap_or_default(),
            caret,
            moved_at: self.moved_at,
            look,
        }
    }
}

/// The shown text of an editor and how it maps back.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub text: String,
    /// The composition's bytes in `text`.
    pub preedit: Option<Range<usize>>,
    /// The caret in the real text, where the composition sits.
    caret_at: usize,
    /// For a masked text, the real byte where each character starts (and the end).
    starts: Option<Vec<usize>>,
}

/// One laid-out row: its rect and the caret stops along it (shown byte, x).
#[derive(Clone, Debug, PartialEq)]
pub struct EditRow {
    pub rect: Rect,
    /// The first and past-the-last shown byte of the row (its break excluded).
    pub start: usize,
    pub end: usize,
    stops: Vec<(usize, f32)>,
}

/// A laid-out editor text with its caret stops, in layout coordinates: what the run hit-tests and
/// moves by, built once per build from the backend's [`crate::TextEngine`].
#[derive(Clone, Debug)]
pub struct EditLayout {
    pub layout: TextLayout,
    pub rows: Vec<EditRow>,
    preedit: Option<Range<usize>>,
    preedit_len: usize,
    caret_at: usize,
    starts: Option<Vec<usize>>,
    shown_len: usize,
}

impl EditLayout {
    /// The caret stops of `layout`, the backend's layout of `shown`, read from the glyphs of its
    /// `metrics` ([`crate::TextEngine::metrics`]). An engine without glyph metrics gives each row only its
    /// two ends as stops.
    pub fn new(layout: TextLayout, metrics: &TextMetrics, shown: &Shown) -> EditLayout {
        let text: &str = &shown.text;
        let mut rows = Vec::new();
        let mut at = 0;
        for row in &metrics.rows {
            let start = at;
            let mut stops = Vec::with_capacity(row.glyphs.len() + 1);
            for glyph in &row.glyphs {
                // Glyphs follow the text in order; a line break has none.
                while text[at..].starts_with('\n') {
                    at += 1;
                }
                let x = row.rect.min.x + if glyph.rtl { glyph.x + glyph.advance } else { glyph.x };
                stops.push((at, x));
                at = next_char(text, at.min(text.len()));
            }
            if row.glyphs.is_empty() && metrics.rows.len() == 1 {
                // No glyph metrics: the row spans the whole text.
                stops.push((0, row.rect.min.x));
                at = text.len();
            }
            let end_x = row.glyphs.last().map_or(row.rect.max.x, |g| row.rect.min.x + if g.rtl { g.x } else { g.x + g.advance });
            stops.push((at, end_x));
            rows.push(EditRow { rect: row.rect, start, end: at, stops });
            if text[at..].starts_with('\n') {
                at += 1;
            }
        }
        if rows.is_empty() {
            rows.push(EditRow { rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(0.0, layout.size.y)), start: 0, end: 0, stops: vec![(0, 0.0)] });
        }
        EditLayout {
            layout,
            rows,
            preedit: shown.preedit.clone(),
            preedit_len: shown.preedit.as_ref().map_or(0, |r| r.len()),
            caret_at: shown.caret_at,
            starts: shown.starts.clone(),
            shown_len: shown.text.len(),
        }
    }

    pub fn size(&self) -> Vec2 {
        self.layout.size
    }

    /// The shown byte of real byte `at`.
    pub fn to_shown(&self, at: usize) -> usize {
        let at = match &self.starts {
            Some(starts) => starts.iter().position(|s| *s >= at).unwrap_or(starts.len().saturating_sub(1)) * MASK.len_utf8(),
            None => at,
        };
        let caret = match &self.starts {
            Some(starts) => starts.iter().position(|s| *s >= self.caret_at).unwrap_or(0) * MASK.len_utf8(),
            None => self.caret_at,
        };
        if self.preedit.is_some() && at > caret { at + self.preedit_len } else { at }
    }

    /// The real byte of shown byte `at`; inside the composition it is the caret.
    pub fn to_text(&self, at: usize) -> usize {
        let at = match &self.preedit {
            Some(range) if at >= range.end => at - self.preedit_len,
            Some(range) if at > range.start => range.start,
            _ => at,
        }
        .min(self.shown_len);
        match &self.starts {
            Some(starts) => starts.get(at / MASK.len_utf8()).copied().unwrap_or_else(|| starts.last().copied().unwrap_or(0)),
            None => at,
        }
    }

    /// The row holding shown byte `at` (a byte at a row's end belongs to that row).
    pub fn row_of(&self, at: usize) -> usize {
        self.rows.iter().position(|row| at <= row.end).unwrap_or(self.rows.len() - 1)
    }

    fn x_of(&self, row: &EditRow, at: usize) -> f32 {
        row.stops.iter().find(|(b, _)| *b >= at).or(row.stops.last()).map_or(row.rect.min.x, |(_, x)| *x)
    }

    /// The caret rect before shown byte `at`, one point wide.
    pub fn caret_rect(&self, at: usize) -> Rect {
        let row = &self.rows[self.row_of(at)];
        let x = self.x_of(row, at);
        Rect::from_min_max(Pos2::new(x, row.rect.min.y), Pos2::new(x + 1.0, row.rect.max.y))
    }

    /// The caret stop of row `row` nearest `x`.
    pub fn hit_row(&self, row: usize, x: f32) -> usize {
        let row = &self.rows[row.min(self.rows.len() - 1)];
        row.stops.iter().min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs())).map_or(row.start, |(at, _)| *at)
    }

    /// The caret stop nearest `pos` (layout coordinates): the row under it (the first or last
    /// above or below the text), then the nearest stop along it.
    pub fn hit(&self, pos: Pos2) -> usize {
        let row = self.rows.iter().position(|row| pos.y < row.rect.max.y).unwrap_or(self.rows.len() - 1);
        self.hit_row(row, pos.x)
    }

    /// The rects covering shown bytes `range`, one per row it crosses.
    pub fn range_rects(&self, range: Range<usize>) -> Vec<Rect> {
        self.rows
            .iter()
            .filter(|row| range.start <= row.end && range.end >= row.start && !(range.start == row.end && range.end > row.end))
            .map(|row| {
                let (a, b) = (self.x_of(row, range.start.max(row.start)), self.x_of(row, range.end.min(row.end)));
                Rect::from_min_max(Pos2::new(a.min(b), row.rect.min.y), Pos2::new(a.max(b), row.rect.max.y))
            })
            .filter(|rect| rect.width() > 0.0)
            .collect()
    }
}

/// An editor's layout placed in its view, for the run's hit tests, moves and scrolling.
#[derive(Clone, Copy, Debug)]
pub struct Geometry<'a> {
    pub layout: &'a EditLayout,
    /// The view the text shows in (the owner's coordinates); the text clips to it.
    pub view: Rect,
    /// Where the unscrolled text's top left sits.
    pub origin: Pos2,
    /// The current scroll offset.
    pub scroll: Vec2,
}

impl Geometry<'_> {
    /// Where the text's top left sits, scrolled.
    pub fn text_origin(&self) -> Pos2 {
        self.origin - self.scroll
    }

    /// The shown byte nearest `pos` (the owner's coordinates).
    pub fn hit(&self, pos: Pos2) -> usize {
        self.layout.hit(pos - self.text_origin().to_vec2())
    }

    /// The most the text scrolls: what it is taller (or, on one line, wider) than the view.
    pub fn max_scroll(&self) -> Vec2 {
        let size = self.layout.size();
        let room = self.view.size() - (self.origin - self.view.min);
        (size - room).max(Vec2::ZERO)
    }

    /// The rows a page move crosses: the view's height in rows, at least one.
    pub fn page_rows(&self) -> usize {
        let row = self.layout.rows.first().map_or(1.0, |row| row.rect.height().max(1.0));
        ((self.view.height() / row).floor() as usize).max(1)
    }
}

/// How an editor's selection, composition and caret look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditLook {
    pub selection: Rgba,
    pub caret: Rgba,
    pub caret_width: f32,
    pub underline: Rgba,
    pub underline_width: f32,
    /// The editor holds key focus: the caret and selection show.
    pub focused: bool,
    /// The caret blinks; off where the motion settings hold it solid.
    pub blinks: bool,
}

/// An editor's paint as one build recorded it; the owning node replays it every paint, and the
/// caret's blink is read there, so a blink edge repaints the node and runs no build.
#[derive(Clone, Debug)]
pub struct EditPaint {
    pub text: TextLayout,
    /// The text's top left, scrolled, in the node's coordinates.
    pub origin: Pos2,
    /// The view the editor clips to.
    pub clip: Rect,
    /// Selection rects in layout coordinates.
    pub selection: Vec<Rect>,
    /// Composition rects in layout coordinates, underlined.
    pub underline: Vec<Rect>,
    /// The caret in layout coordinates, while focused.
    pub caret: Option<Rect>,
    pub moved_at: f64,
    pub look: EditLook,
}

impl EditPaint {
    /// Whether a caret that last moved at `moved_at` shows at `now`, and the next blink edge.
    pub fn blink(moved_at: f64, now: f64) -> (bool, f64) {
        let step = ((now - moved_at).max(0.0) / CARET_HALF_PERIOD).floor();
        (step % 2.0 == 0.0, moved_at + (step + 1.0) * CARET_HALF_PERIOD)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>) {
        let at = self.origin.to_vec2();
        cx.push_clip(self.clip);
        for rect in &self.selection {
            cx.fill_rect(rect.translate(at), self.look.selection);
        }
        cx.text(&self.text, self.origin, None);
        for rect in &self.underline {
            let y = rect.max.y - self.look.underline_width;
            let rect = Rect::from_min_max(Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, rect.max.y)).translate(at);
            cx.fill_rect(rect, self.look.underline);
        }
        if let Some(caret) = self.caret {
            let shows = if self.look.blinks {
                let moved_at = self.moved_at;
                let now = cx.time_until(|now| Some(EditPaint::blink(moved_at, now).1));
                EditPaint::blink(moved_at, now).0
            } else {
                true
            };
            if shows {
                cx.fill_rect(caret.translate(at), self.look.caret);
            }
        }
        cx.pop_clip();
    }
}
