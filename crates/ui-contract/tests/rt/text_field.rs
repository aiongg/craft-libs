//! A test-local text field on craft-ui-rt (moved out of the library; the real text inputs are the
//! widgets' fields over the text step): an editor state that applies `EditorInput` to a string
//! (typed and committed text, IME preedits, caret moves and selection, deletes, Select All, line
//! breaks), and `TextField`, the stand-in text field over it until the integration task moves the
//! `ui-widgets` fields onto the rt cut. Its input comes from the node's queue in arrival order
//! (`Cx::editor_input`): the key stage's intents and the IME engine's preedits and commits
//! ([`super::ime`]).

use std::ops::Range;

use craft_behavior::{EditCommand, EditorInput, Granularity, NavMove};
use craft_ui_rt::{ImePurpose, Pos2, Rect, Sense, Vec2};

use super::access::Access;
use super::key::ComponentKey;
use super::run::{Component, Cx, MeasureCx, Paint, show_leaf};
use super::ui::Ui;
use crate::contract::EditResponse;
use crate::contract::{Bind, BindMut, IdSource, Input, MemoKey, MemoProps};
use crate::primitives::{PaintShape, TextRun};
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::style::{StyleColor, UiTokens, rgba_color};

/// An editor's caret, selection anchor and composition, as byte offsets into its text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditState {
    caret: usize,
    anchor: usize,
    preedit: Option<(String, Option<Range<usize>>)>,
}

fn prev_boundary(text: &str, at: usize) -> usize {
    text[..at].char_indices().next_back().map_or(0, |(i, _)| i)
}

fn next_boundary(text: &str, at: usize) -> usize {
    text[at..].chars().next().map_or(at, |c| at + c.len_utf8())
}

/// The start of the word before `at`: back over spaces, then over the word.
fn word_back(text: &str, at: usize) -> usize {
    let mut i = at;
    while i > 0 && text[..i].ends_with(char::is_whitespace) {
        i = prev_boundary(text, i);
    }
    while i > 0 && !text[..i].ends_with(char::is_whitespace) {
        i = prev_boundary(text, i);
    }
    i
}

/// The end of the word after `at`: over spaces, then over the word.
fn word_forward(text: &str, at: usize) -> usize {
    let mut i = at;
    while i < text.len() && text[i..].starts_with(char::is_whitespace) {
        i = next_boundary(text, i);
    }
    while i < text.len() && !text[i..].starts_with(char::is_whitespace) {
        i = next_boundary(text, i);
    }
    i
}

impl EditState {
    /// The caret's byte offset.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selected byte range; empty at the caret when nothing is selected.
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    /// The text the input method composes at the caret, while it composes.
    pub fn preedit(&self) -> Option<&str> {
        self.preedit.as_ref().map(|(text, _)| text.as_str())
    }

    /// Keeps the caret and anchor inside `text` and on character boundaries.
    fn clamp(&mut self, text: &str) {
        let fit = |at: usize| {
            let mut at = at.min(text.len());
            while !text.is_char_boundary(at) {
                at -= 1;
            }
            at
        };
        self.caret = fit(self.caret);
        self.anchor = fit(self.anchor);
    }

    fn replace_selection(&mut self, text: &mut String, insert: &str) {
        let range = self.selection();
        text.replace_range(range.clone(), insert);
        self.caret = range.start + insert.len();
        self.anchor = self.caret;
    }

    /// The offset a move of `mv` by `by` from the caret reaches in `text`.
    fn moved(&self, text: &str, mv: NavMove, by: Granularity) -> usize {
        let at = self.caret;
        match (mv, by) {
            (NavMove::Back, Granularity::Word) => word_back(text, at),
            (NavMove::Forward, Granularity::Word) => word_forward(text, at),
            (NavMove::Back, Granularity::Unit) => prev_boundary(text, at),
            (NavMove::Forward, Granularity::Unit) => next_boundary(text, at),
            (NavMove::Back | NavMove::Up | NavMove::Start | NavMove::PageUp, _) => 0,
            _ => text.len(),
        }
    }

    /// Applies `input` to `text`; returns whether `text` changed. A single-line editor
    /// (`multiline` off) drops line breaks.
    pub fn apply(&mut self, text: &mut String, input: &EditorInput, multiline: bool) -> bool {
        self.clamp(text);
        match input {
            EditorInput::Text(typed) => {
                self.preedit = None;
                let typed: String = if multiline { typed.to_string() } else { typed.chars().filter(|c| *c != '\n' && *c != '\r').collect() };
                if typed.is_empty() {
                    return false;
                }
                self.replace_selection(text, &typed);
                true
            }
            EditorInput::Preedit { text: composing, cursor, .. } => {
                self.preedit = (!composing.is_empty()).then(|| (composing.to_string(), cursor.clone()));
                false
            }
            EditorInput::ImeCommit => {
                self.preedit = None;
                false
            }
            EditorInput::Move(mv, mods) => {
                self.caret = self.moved(text, *mv, mods.by);
                if !mods.extend {
                    self.anchor = self.caret;
                }
                false
            }
            EditorInput::Delete { forward, by, .. } => {
                if self.caret == self.anchor {
                    let mv = if *forward { NavMove::Forward } else { NavMove::Back };
                    self.anchor = self.moved(text, mv, *by);
                }
                if self.caret == self.anchor {
                    return false;
                }
                self.replace_selection(text, "");
                true
            }
            EditorInput::Edit(EditCommand::SelectAll) => {
                self.anchor = 0;
                self.caret = text.len();
                false
            }
            EditorInput::Edit(EditCommand::Cut | EditCommand::Delete) if self.caret != self.anchor => {
                self.replace_selection(text, "");
                true
            }
            EditorInput::Newline { .. } if multiline => {
                self.replace_selection(text, "\n");
                true
            }
            _ => false,
        }
    }
}

/// The glyph advance the stand-in field assumes, as a share of the button size, until its caret
/// comes from the shaped layout (B2's text engine binding in the integration task).
const ADVANCE: f32 = 0.4;

/// A text field over a string: typing, the input method, caret moves, deletes and Select All.
pub struct TextField<'a> {
    id: IdSource,
    value: Bind<'a, String>,
    multiline: bool,
    password: bool,
}

impl<'a> TextField<'a> {
    /// A single-line field editing `value`.
    pub fn new(value: Bind<'a, String>) -> TextField<'a> {
        TextField { id: IdSource::Auto, value, multiline: false, password: false }
    }

    /// Enter makes a line break and Up and Down move the caret.
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// The field hides its text and asks the platform for a password session.
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Where the field's id comes from.
    pub fn id_source(mut self, id: IdSource) -> Self {
        self.id = id;
        self
    }

    /// Shows the field.
    pub fn show(self, ui: &mut Ui<'_, '_>) -> EditResponse<String> {
        show_leaf(self, ui)
    }

    fn keep(&self) -> KeepKeys {
        if self.multiline { KeepKeys::BlockArrows } else { KeepKeys::InlineArrows }
    }

    fn caret_rect(&self, rect: Rect, text: &str, state: &EditState, button: f32) -> Rect {
        let before = text[..state.caret().min(text.len())].chars().count() + state.preedit().map_or(0, |p| p.chars().count());
        let pad = button / 4.0;
        let x = rect.min.x + pad + before as f32 * button * ADVANCE;
        Rect::from_min_size(Pos2::new(x.min(rect.max.x - pad), rect.min.y + pad), Vec2::new(1.0, rect.height() - 2.0 * pad))
    }
}

impl ComponentKey for TextField<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.value.key(key);
        self.multiline.key(key);
        self.password.key(key);
    }
}

impl Component for TextField<'_> {
    const ROLE: accesskit::Role = accesskit::Role::TextInput;
    type Output = EditResponse<String>;

    fn id_source(&self) -> IdSource {
        self.id
    }

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        let lines = if self.multiline { 3.0 } else { 1.0 };
        Vec2::new(cx.room.x.min(12.0 * cx.tokens.size.button), lines * cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    /// A multi-line field makes Enter a line break, so it takes Submit.
    fn keys(&self) -> Option<KeyUse> {
        let keys = KeyUse::of(Input::Text).with_keep(self.keep());
        Some(if self.multiline { keys.with_submits() } else { keys })
    }

    fn text_input(&self) -> Option<ImePurpose> {
        Some(if self.password { ImePurpose::Password } else { ImePurpose::Normal })
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let mut text = cx.value().cloned().unwrap_or_default();
        let mut state = cx.recall::<EditState>();
        let mut changed = false;
        for input in cx.editor_input(self.keep()) {
            changed |= state.apply(&mut text, &input, self.multiline);
        }
        cx.ime_cursor(self.caret_rect(cx.rect, &text, &state, cx.tokens.size.button));
        *cx.memory::<EditState>() = state;
        if changed {
            cx.set(text);
        }
    }

    fn paint(&mut self, paint: &Paint<'_>, cx: &Cx<'_, Self>) {
        let bounds = paint.bounds();
        let tokens = paint.tokens;
        let draw = paint.draw();
        draw.fill(&PaintShape::Rect(bounds, UiTokens::corner(tokens.size.button / 4.0)), StyleColor(rgba_color(tokens.color.state.hover)));
        let text = cx.value().cloned().unwrap_or_default();
        let state = cx.recall::<EditState>();
        let shown = match (self.password, state.preedit()) {
            (true, _) => "\u{2022}".repeat(text.chars().count()),
            (false, Some(preedit)) => {
                let at = state.caret().min(text.len());
                format!("{}{preedit}{}", &text[..at], &text[at..])
            }
            (false, None) => text.clone(),
        };
        let pad = tokens.size.button / 4.0;
        draw.text(bounds.shrink(pad), &TextRun::new(shown));
        let caret = self.caret_rect(bounds, &text, &state, tokens.size.button);
        draw.fill(&PaintShape::Rect(caret, UiTokens::corner(0.0)), StyleColor(rgba_color(tokens.color.text.normal)));
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if !self.password {
            access.value(cx.value().cloned().unwrap_or_default());
        }
    }

    fn finish(self, cx: &Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}
