//! The text adapter: places the egui text widget at a text part's rect, applies the IME engine's edits and the
//! edit commands to it, and records what happened for the pass's typing run (18.2).

use crate::primitives::{TextLayout, TextOptions};
use crate::runtime::access::text::{Line, Run};
use crate::runtime::egui::{clipboard, focus as egui_focus, text_edit};
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::runtime::keys::{self, Intents};
use crate::runtime::{commands, ime};
use craft_behavior::{EditCommand, EditorInput, Intent, PartKey, TextPos};
use egui::{Context, Id, Pos2, Rect, Ui};
use std::any::Any;
use std::sync::Arc;

/// How a typing run ended, as the adapter saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    /// Enter.
    Submit,
    /// Focus left the part.
    Blur,
    /// Escape.
    Escape,
}

/// The composing text and where it sits in the draft, in characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Preedit {
    pub at: usize,
    pub text: String,
}

/// What the adapter keeps per text part between passes.
#[derive(Clone, Default)]
pub(crate) struct TextState {
    pub draft: Option<String>,
    /// The value when the run began (a `K::Value`).
    pub start: Option<Arc<dyn Any + Send + Sync>>,
    /// The draft changed since the typing run last read it.
    pub dirty: bool,
    pub end: Option<End>,
    pub invalid: bool,
    /// The text a commit left, shown for the pass after it.
    pub normalised: Option<String>,
    pub preedit: Option<Preedit>,
    /// The rows of last pass's text, for mapping an AccessKit selection back to characters; kept only while
    /// AccessKit runs.
    pub rows: Option<Rows>,
    pub caret: Option<usize>,
    /// The pass time the caret last moved or text was typed, for the blink.
    pub caret_moved: f64,
    pub focused: bool,
}

/// The id of a text part's egui widget, which holds key focus.
pub(crate) fn text_id(id: Id, key: PartKey) -> Id {
    crate::runtime::memory::sub(id, ("text", key))
}

/// The keys the text part keeps, from its options.
pub(crate) fn keys_of(base: KeyUse, options: &TextOptions) -> KeyUse {
    if options.tab_inserts {
        base.with_keep(KeepKeys::Tab)
    } else if options.multiline() {
        base.with_keep(KeepKeys::BlockArrows)
    } else {
        base
    }
}

pub(crate) struct AdapterIn<'a> {
    pub ctx: &'a Context,
    pub ui: &'a mut Ui,
    pub id: Id,
    pub key: PartKey,
    pub rect: Rect,
    pub shown: &'a str,
    pub options: &'a TextOptions,
    pub keys: KeyUse,
    pub read_only: bool,
    pub rtl: bool,
    /// The text widget holds key focus.
    pub focused: bool,
    pub ink: egui::Color32,
    pub intents: &'a mut Intents,
    pub placeholder: Option<&'a str>,
    /// Whether the caret stroke shows, from the blink.
    pub caret_visible: bool,
    pub now: f64,
}

pub(crate) struct AdapterOut {
    pub layout: TextLayout,
    pub focused: bool,
}

/// The rows of a laid-out text as AccessKit lines, with the byte where each starts in the text.
#[derive(Clone)]
pub(crate) struct Rows {
    pub lines: Vec<Line>,
    starts: Vec<usize>,
}

impl Rows {
    /// The rows of `layout` over `text`, placed from `origin`. A newline between two rows belongs to neither.
    pub(crate) fn of(layout: &TextLayout, text: &str, origin: Pos2) -> Rows {
        let (mut lines, mut starts, mut at) = (Vec::new(), Vec::new(), 0);
        for (row, rect, runs) in layout.row_lines() {
            starts.push(at);
            at += row.len();
            if text.get(at..).is_some_and(|rest| rest.starts_with('\n')) {
                at += 1;
            }
            let runs = runs.into_iter().map(|run| Run { bytes: run.bytes, rtl: run.rtl, positions: run.positions, widths: run.widths }).collect();
            lines.push(Line { text: row, rect: rect.translate(origin.to_vec2()), runs });
        }
        Rows { lines, starts }
    }

    /// The position of character `chars` of `text`.
    pub(crate) fn pos(&self, text: &str, chars: usize) -> TextPos {
        let byte = byte_of(text, chars);
        let line = self.starts.iter().rposition(|start| *start <= byte).unwrap_or(0);
        TextPos::new(line, byte.saturating_sub(self.starts.get(line).copied().unwrap_or(0)))
    }

    /// The character index of `pos` in `text`.
    pub(crate) fn chars(&self, text: &str, pos: TextPos) -> usize {
        let byte = self.starts.get(pos.line).copied().unwrap_or(0) + pos.byte;
        text.get(..byte).map_or_else(|| text.chars().count(), |head| head.chars().count())
    }
}

fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(byte, _)| byte)
}

/// Inserts `insert` over the selection `(a, b)` (characters) of `text`; returns the caret after it.
fn replace(text: &mut String, selection: (usize, usize), insert: &str) -> usize {
    let (lo, hi) = (selection.0.min(selection.1), selection.0.max(selection.1));
    let (start, end) = (byte_of(text, lo), byte_of(text, hi));
    text.replace_range(start..end, insert);
    lo + insert.chars().count()
}

/// Applies the IME engine's edits and this pass's edit commands to the draft; returns whether the text changed.
fn apply(input: &mut AdapterIn<'_>, state: &mut TextState, text: &mut String, widget: Id) -> bool {
    let ctx = input.ctx;
    let mut changed = false;
    let selection = |text: &str| text_edit::selection(ctx, widget).map_or((text.chars().count(), text.chars().count()), |s| s);
    if let Some(preedit) = ime::take_leaving(ctx, widget).filter(|preedit| !preedit.is_empty()) {
        let at = state.preedit.take().map_or_else(|| text.chars().count(), |p| p.at);
        let caret = replace(text, (at, at), &preedit);
        text_edit::select(ctx, widget, caret, caret);
        changed = true;
    }
    let edits = ime::take(ctx, widget, input.read_only);
    if !input.focused {
        return changed;
    }
    for edit in edits {
        match edit {
            EditorInput::Preedit { text: composing, .. } => {
                state.preedit = (!composing.is_empty())
                    .then(|| Preedit { at: state.preedit.as_ref().map_or_else(|| selection(text).1, |p| p.at), text: composing.to_string() });
            }
            EditorInput::ImeCommit => {}
            EditorInput::Text(typed) => {
                let at = state.preedit.take().map_or_else(|| selection(text), |p| (p.at, p.at));
                let caret = replace(text, at, &typed);
                text_edit::select(ctx, widget, caret, caret);
                changed = true;
            }
            _ => {}
        }
    }
    while let Some(command) = input.intents.take_edit() {
        if input.read_only && !EditorInput::Edit(command).is_read_only_safe() {
            continue;
        }
        match command {
            EditCommand::SelectAll => text_edit::select(ctx, widget, 0, text.chars().count()),
            EditCommand::Copy | EditCommand::Cut => {
                let (a, b) = selection(text);
                if a != b {
                    let (lo, hi) = (a.min(b), a.max(b));
                    clipboard::copy_text(ctx, text[byte_of(text, lo)..byte_of(text, hi)].to_owned());
                    if command == EditCommand::Cut {
                        let caret = replace(text, (lo, hi), "");
                        text_edit::select(ctx, widget, caret, caret);
                        changed = true;
                    }
                }
            }
            EditCommand::Paste => {
                if let Some(pasted) = keys::pasted_text(ctx, input.id) {
                    let pasted = if input.options.multiline() { pasted.to_string() } else { pasted.replace(['\n', '\r'], " ") };
                    let caret = replace(text, selection(text), &pasted);
                    text_edit::select(ctx, widget, caret, caret);
                    changed = true;
                }
            }
            EditCommand::Undo | EditCommand::Redo => {
                let redo = command == EditCommand::Redo;
                match text_edit::undo(ctx, widget, text, redo) {
                    Some(previous) => {
                        *text = previous;
                        changed = true;
                    }
                    None => commands::fall_through(ctx, command),
                }
            }
            _ => {}
        }
    }
    changed
}

/// Runs the text widget for one pass.
pub(crate) fn run(state: &mut TextState, mut input: AdapterIn<'_>) -> AdapterOut {
    let (ctx, widget) = (input.ctx, text_id(input.id, input.key));
    let focused_before = input.focused;
    let mut base = state.draft.clone().or_else(|| state.normalised.take()).unwrap_or_else(|| input.shown.to_owned());
    let edited = apply(&mut input, state, &mut base, widget);
    if focused_before && (state.draft.is_some() || state.preedit.is_some()) && input.intents.take(Intent::Cancel) {
        state.end = Some(End::Escape);
        state.preedit = None;
    }
    let mut display = base.clone();
    let preedit = state.preedit.as_ref().map(|p| {
        let at = byte_of(&display, p.at);
        display.insert_str(at, &p.text);
        at..at + p.text.len()
    });
    let filter = egui_focus::event_filter(input.keys.keep, false);
    let shown = text_edit::show(
        input.ui,
        text_edit::Edit {
            id: widget,
            rect: input.rect,
            text: &mut display,
            read_only: input.read_only,
            multiline: input.options.multiline(),
            password: input.options.mask,
            hint: input.placeholder,
            mono: input.options.mono,
            ink: input.ink,
            filter,
            char_limit: input.options.limit,
            preedit: preedit.clone(),
            rtl: input.rtl,
            caret_visible: input.caret_visible,
        },
    );
    // egui's own IME output is dropped: the engine writes the pass's.
    if let Some(caret) = shown.caret.filter(|_| shown.focused) {
        ime::note_caret(ctx, widget, input.rect, caret, input.options.mask);
    }
    let typed = shown.changed && state.preedit.is_none();
    if typed || edited {
        state.draft = Some(if typed { display.clone() } else { base });
        state.dirty = true;
        state.caret_moved = input.now;
    }
    let caret = text_edit::selection(ctx, widget).map(|selection| selection.0);
    if caret != state.caret {
        state.caret = caret;
        state.caret_moved = input.now;
    }
    if shown.entered && shown.focused && !input.options.multiline() {
        state.end = Some(End::Submit);
    } else if shown.lost_focus && state.draft.is_some() && state.end.is_none() {
        state.end = Some(End::Blur);
    }
    state.focused = shown.focused;
    let full = display;
    AdapterOut { layout: TextLayout::new(shown.galley, false, full), focused: shown.focused }
}
