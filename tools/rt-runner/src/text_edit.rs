//! A single-line text editor driven by craft-behavior's [`EditorInput`] stream: keys go through
//! `craft_behavior::keys::intent` with a text role, IME preedit and commit arrive as their own
//! inputs. Positions are byte indices on char boundaries.

use std::ops::Range;

use craft_behavior::keys::intent;
use craft_behavior::{EditCommand, EditorInput, Granularity, Intent, KeyRole, NavMove};
use craft_host::target::Os;
use craft_keymap::chord::{Key, Mods};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preedit {
    pub text: String,
    /// The IME's cursor as a byte range of `text`.
    pub cursor: Option<Range<usize>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextEdit {
    text: String,
    caret: usize,
    anchor: usize,
    preedit: Option<Preedit>,
}

/// What one input did. The field turns the requests into its output and platform calls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub text_changed: bool,
    pub caret_moved: bool,
    /// Text for the clipboard (copy or cut).
    pub copy: Option<String>,
    pub paste: bool,
    /// Enter; `true` with Shift.
    pub submit: Option<bool>,
    /// Escape.
    pub cancel: bool,
    /// Tab; `true` for Shift+Tab.
    pub tab: Option<bool>,
}

/// The editor input a key press means in a single-line text field, if any.
pub fn editor_input(os: Os, key: Key, mods: Mods) -> Option<EditorInput> {
    match key {
        Key::Enter => return Some(EditorInput::newline(mods.shift)),
        Key::Tab if !(mods.cmd || mods.ctrl || mods.alt) => {
            return Some(if mods.shift { EditorInput::BackTab } else { EditorInput::Tab });
        }
        Key::Escape => return Some(EditorInput::Cancel),
        _ => {}
    }
    if mods.cmd && !mods.alt {
        let command = match key {
            Key::A => Some(EditCommand::SelectAll),
            Key::C => Some(EditCommand::Copy),
            Key::X => Some(EditCommand::Cut),
            Key::V => Some(EditCommand::Paste),
            Key::Z if mods.shift => Some(EditCommand::Redo),
            Key::Z => Some(EditCommand::Undo),
            _ => None,
        };
        if let Some(command) = command {
            return Some(EditorInput::Edit(command));
        }
    }
    let role = KeyRole::new().text(true).moves(true).deletes(true);
    match intent(os, key, mods, role)? {
        Intent::Move(mv, by) => Some(EditorInput::Move(mv, by)),
        Intent::Delete { forward, by, shift, .. } => Some(EditorInput::delete_with_shift(forward, by, shift)),
        Intent::Edit(command) => Some(EditorInput::Edit(command)),
        _ => None,
    }
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

impl TextEdit {
    /// `text` with the caret at its end.
    pub fn new(text: impl Into<String>) -> TextEdit {
        let text = text.into();
        let end = text.len();
        TextEdit { text, caret: end, anchor: end, preedit: None }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Where the selection started: the end that stays while the caret moves.
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.preedit.as_ref()
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.caret = self.text.len();
        self.anchor = self.caret;
        self.preedit = None;
    }

    /// Places the caret at `index`; with `extend` the anchor stays.
    pub fn place(&mut self, index: usize, extend: bool) {
        self.caret = self.boundary(index);
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// The text as drawn: the preedit sits at the caret. Returns the text, the caret position
    /// in it, and the preedit's range in it.
    pub fn shown(&self) -> (String, usize, Option<Range<usize>>) {
        match &self.preedit {
            None => (self.text.clone(), self.caret, None),
            Some(p) => {
                let mut s = String::with_capacity(self.text.len() + p.text.len());
                s.push_str(&self.text[..self.caret]);
                s.push_str(&p.text);
                s.push_str(&self.text[self.caret..]);
                let at = self.caret + p.cursor.as_ref().map_or(p.text.len(), |c| c.end);
                (s, at, Some(self.caret..self.caret + p.text.len()))
            }
        }
    }

    /// A shown-text index mapped back to the text (inside a preedit it is the caret).
    pub fn from_shown(&self, index: usize) -> usize {
        match &self.preedit {
            None => self.boundary(index),
            Some(_) if index <= self.caret => self.boundary(index),
            Some(p) if index < self.caret + p.text.len() => self.caret,
            Some(p) => self.boundary(index - p.text.len()),
        }
    }

    pub fn apply(&mut self, input: &EditorInput) -> Applied {
        let mut out = Applied::default();
        let before = (self.caret, self.anchor);
        match input {
            EditorInput::Text(text) => {
                let clean: String = text.chars().filter(|c| !c.is_control()).collect();
                if !clean.is_empty() {
                    self.replace_selection(&clean);
                    out.text_changed = true;
                }
            }
            EditorInput::Move(mv, mods) => self.move_caret(*mv, mods.by, mods.extend),
            EditorInput::Delete { forward, by, .. } => {
                out.text_changed = self.delete(*forward, *by);
            }
            EditorInput::DeleteSurrounding { before: b, after: a, unit, .. } => {
                let chars_before = unit.chars_before(*b, &self.text[..self.caret]);
                let chars_after = unit.chars_after(*a, &self.text[self.caret..]);
                let start = self.text[..self.caret].char_indices().rev().take(chars_before).last().map_or(self.caret, |(i, _)| i);
                let end = self.text[self.caret..].char_indices().take(chars_after).last().map_or(self.caret, |(i, c)| self.caret + i + c.len_utf8());
                if start < end {
                    self.text.replace_range(start..end, "");
                    self.caret = start;
                    self.anchor = start;
                    out.text_changed = true;
                }
            }
            EditorInput::Select { anchor, focus, .. } => {
                self.anchor = self.boundary(anchor.byte);
                self.caret = self.boundary(focus.byte);
            }
            EditorInput::Preedit { text, cursor, .. } => {
                if text.is_empty() {
                    self.preedit = None;
                } else {
                    // A composition starts at the selection's start and keeps the selected text, as craft's
                    // text fields do.
                    if self.preedit.is_none() && self.caret != self.anchor {
                        let start = self.caret.min(self.anchor);
                        self.caret = start;
                        self.anchor = start;
                    }
                    self.preedit = Some(Preedit { text: text.to_string(), cursor: cursor.clone() });
                }
                out.caret_moved = true;
            }
            EditorInput::ImeCommit => self.preedit = None,
            EditorInput::Edit(command) => match command {
                EditCommand::Copy => out.copy = Some(self.selected().to_owned()),
                EditCommand::Cut => {
                    out.copy = Some(self.selected().to_owned());
                    if self.caret != self.anchor {
                        self.replace_selection("");
                        out.text_changed = true;
                    }
                }
                EditCommand::Paste => out.paste = true,
                EditCommand::SelectAll => self.select_all(),
                EditCommand::Delete if self.caret != self.anchor => {
                    self.replace_selection("");
                    out.text_changed = true;
                }
                _ => {}
            },
            EditorInput::Newline { shift, .. } => out.submit = Some(*shift),
            EditorInput::Tab => out.tab = Some(false),
            EditorInput::BackTab => out.tab = Some(true),
            EditorInput::Cancel => out.cancel = true,
            _ => {}
        }
        out.caret_moved |= out.text_changed || before != (self.caret, self.anchor);
        out
    }

    fn selected(&self) -> &str {
        &self.text[self.selection()]
    }

    fn replace_selection(&mut self, with: &str) {
        let range = self.selection();
        self.text.replace_range(range.clone(), with);
        self.caret = range.start + with.len();
        self.anchor = self.caret;
    }

    fn boundary(&self, index: usize) -> usize {
        let mut i = index.min(self.text.len());
        while !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn prev_char(&self, i: usize) -> usize {
        self.text[..i].char_indices().next_back().map_or(0, |(at, _)| at)
    }

    fn next_char(&self, i: usize) -> usize {
        self.text[i..].chars().next().map_or(self.text.len(), |c| i + c.len_utf8())
    }

    fn prev_word(&self, i: usize) -> usize {
        let mut chars = self.text[..i].char_indices().rev().peekable();
        while chars.next_if(|(_, c)| class(*c) == Class::Space).is_some() {}
        let Some(&(mut at, first)) = chars.peek() else {
            return 0;
        };
        let kind = class(first);
        while let Some((j, _)) = chars.next_if(|(_, c)| class(*c) == kind) {
            at = j;
        }
        at
    }

    fn next_word(&self, i: usize) -> usize {
        let rest = &self.text[i..];
        let mut chars = rest.char_indices().peekable();
        if let Some(&(_, first)) = chars.peek() {
            let kind = class(first);
            if kind != Class::Space {
                while chars.next_if(|(_, c)| class(*c) == kind).is_some() {}
            }
        }
        while chars.next_if(|(_, c)| class(*c) == Class::Space).is_some() {}
        chars.peek().map_or(self.text.len(), |(j, _)| i + j)
    }

    fn move_caret(&mut self, mv: NavMove, by: Granularity, extend: bool) {
        let sel = self.selection();
        let to = match (mv, by) {
            (NavMove::Back, Granularity::Unit) if !extend && !sel.is_empty() => sel.start,
            (NavMove::Forward, Granularity::Unit) if !extend && !sel.is_empty() => sel.end,
            (NavMove::Back, Granularity::Unit) => self.prev_char(self.caret),
            (NavMove::Forward, Granularity::Unit) => self.next_char(self.caret),
            (NavMove::Back, Granularity::Word) => self.prev_word(self.caret),
            (NavMove::Forward, Granularity::Word) => self.next_word(self.caret),
            (NavMove::Back | NavMove::Start | NavMove::Up | NavMove::PageUp, _) => 0,
            (NavMove::Forward | NavMove::End | NavMove::Down | NavMove::PageDown, _) => self.text.len(),
            _ => return,
        };
        self.place(to, extend);
    }

    fn delete(&mut self, forward: bool, by: Granularity) -> bool {
        if self.caret != self.anchor {
            self.replace_selection("");
            return true;
        }
        let at = self.caret;
        let other = match (forward, by) {
            (false, Granularity::Unit) => self.prev_char(at),
            (true, Granularity::Unit) => self.next_char(at),
            (false, Granularity::Word) => self.prev_word(at),
            (true, Granularity::Word) => self.next_word(at),
            (false, _) => 0,
            (true, _) => self.text.len(),
        };
        if other == at {
            return false;
        }
        self.anchor = other;
        self.replace_selection("");
        true
    }
}
