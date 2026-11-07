//! The types of text parts and editors (18.2, 18.4): keys, options, completion and the own-caret editor state.

use crate::primitives::TextLayout;
use crate::runtime::env::Label;
use craft_behavior::{PartKey, TextPos};
use egui::Rect;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::Arc;

/// A text part's key, typed by the value its codec gives.
#[derive(Debug)]
pub struct TextKey<V>(pub(crate) PartKey, PhantomData<fn() -> V>);

impl<V> Clone for TextKey<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> Copy for TextKey<V> {}

impl<V> TextKey<V> {
    /// The key of the text part `key`.
    pub const fn new(key: PartKey) -> Self {
        TextKey(key, PhantomData)
    }

    /// The part key.
    pub const fn key(self) -> PartKey {
        self.0
    }
}

/// How a text part behaves beyond its codec.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct TextOptions {
    /// Shown while the text is empty.
    pub placeholder: Option<Label>,
    /// Lines: 1 is a single-line field.
    pub lines: Option<usize>,
    /// The most characters a draft holds.
    pub limit: Option<usize>,
    /// Characters show as dots.
    pub mask: bool,
    /// A parsable draft applies as typed whatever the codec says.
    pub live: bool,
    /// Escape clears a non-empty draft before it restores.
    pub clear_on_escape: bool,
    /// Monospace text.
    pub mono: bool,
    /// Tab inserts a tab instead of moving focus.
    pub tab_inserts: bool,
    /// Candidates for the text before the caret.
    pub complete: Option<Arc<dyn Completer>>,
}

impl TextOptions {
    /// Options with every default.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn multiline(&self) -> bool {
        self.lines.is_some_and(|lines| lines > 1)
    }
}

impl std::fmt::Debug for TextOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextOptions").field("lines", &self.lines).field("live", &self.live).field("complete", &self.complete.is_some()).finish_non_exhaustive()
    }
}

/// Completion candidates for a text part.
pub trait Completer: Send + Sync {
    /// The candidates for the text before the caret, and the byte range they replace.
    fn complete(&self, text: &str, caret: usize, out: &mut Vec<Completion>) -> Option<Range<usize>>;
}

/// One completion candidate.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Completion {
    /// The text shown in the list.
    pub label: Label,
    /// The text that replaces the range.
    pub insert: String,
    /// A second line of the row.
    pub detail: Option<Label>,
}

impl Completion {
    /// A candidate shown as `label` that inserts `insert`.
    pub fn new(label: impl Into<Label>, insert: impl Into<String>) -> Self {
        Completion { label: label.into(), insert: insert.into(), detail: None }
    }
}

/// What an editor part edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditorPurpose {
    /// Prose.
    Text,
    /// Source code.
    Code,
    /// A number.
    Number,
    /// A secret: the OS shows no suggestions.
    Password,
}

/// One bidi run of a line: its bytes, direction, each character's start offset and advance.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EditorRun<'a> {
    /// The run's bytes in the line's text.
    pub bytes: Range<usize>,
    /// The run reads right to left.
    pub rtl: bool,
    /// Each character's start offset, relative to the line's rect.
    pub positions: &'a [f32],
    /// Each character's advance.
    pub widths: &'a [f32],
}

impl<'a> EditorRun<'a> {
    /// A run over `bytes`.
    pub const fn new(bytes: Range<usize>, rtl: bool, positions: &'a [f32], widths: &'a [f32]) -> Self {
        EditorRun { bytes, rtl, positions, widths }
    }
}

/// One visible line of an own-caret editor.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct EditorLine<'a> {
    /// The line's text.
    pub text: &'a str,
    /// Where it is drawn.
    pub rect: Rect,
    /// The baseline's y.
    pub baseline: f32,
    /// The bidi runs; a single-direction line is one run.
    pub runs: &'a [EditorRun<'a>],
}

impl<'a> EditorLine<'a> {
    /// A line of `text` at `rect`.
    pub const fn new(text: &'a str, rect: Rect, baseline: f32, runs: &'a [EditorRun<'a>]) -> Self {
        EditorLine { text, rect, baseline, runs }
    }
}

/// What an own-caret editor reports once per frame for the IME and AccessKit.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EditorState<'a> {
    /// The caret, for the IME candidate window.
    pub caret: Rect,
    /// The visible lines in document order.
    pub lines: &'a [EditorLine<'a>],
    /// The caret's line (an index into `lines`) and byte offset.
    pub caret_at: TextPos,
    /// The selection.
    pub selection: Option<(TextPos, TextPos)>,
    /// Bumped on every document change; a change that follows no delivered edit resets the composition context.
    pub revision: u64,
    /// Whether undo and redo have something to do; false sends them to the app command (11.3).
    pub can_undo: bool,
    /// Whether redo has something to do.
    pub can_redo: bool,
}

impl<'a> EditorState<'a> {
    /// A state with no selection and nothing to undo.
    pub const fn new(caret: Rect, lines: &'a [EditorLine<'a>], caret_at: TextPos, revision: u64) -> Self {
        EditorState { caret, lines, caret_at, selection: None, revision, can_undo: false, can_redo: false }
    }
}

impl TextLayout {
    /// The layout's runs as the borrowed runs an editor line takes.
    pub fn editor_runs(&self) -> Vec<EditorRun<'_>> {
        self.runs.iter().map(|run| EditorRun::new(run.bytes.clone(), run.rtl, &run.positions, &run.widths)).collect()
    }
}
