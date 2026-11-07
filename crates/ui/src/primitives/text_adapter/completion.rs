//! The completion list of a text part (18.2): candidates from the options' `Completer` for the draft and caret, the
//! keys the list takes while it is open, and acceptance by replacing the candidate's range.

use super::byte_of;
use crate::primitives::{Completer, Completion};
use crate::runtime::keys;
use craft_keymap::{Chord, Key};
use egui::{Context, Id};
use std::ops::Range;

/// The keys the open list takes: Up and Down move its highlight, Enter and Tab accept, Escape hides it.
const KEYS: [Key; 5] = [Key::Up, Key::Down, Key::Enter, Key::Tab, Key::Escape];

/// What the adapter keeps of the list between passes.
#[derive(Clone, Default)]
pub(crate) struct Completions {
    /// The candidates shown; empty while the list is closed.
    pub items: Vec<Completion>,
    /// The highlighted candidate.
    pub index: usize,
    /// The list is open: it holds the keys of [`KEYS`].
    pub open: bool,
    /// A candidate the pointer picked in the list, accepted by the next pass.
    pub picked: Option<usize>,
    /// The bytes of `for_text` the candidates replace.
    range: Option<Range<usize>>,
    /// The draft the candidates were computed for.
    for_text: String,
    /// The draft Escape hid the list for; the list stays hidden until the draft changes.
    dismissed: Option<String>,
}

impl Completions {
    fn close(&mut self) {
        self.items.clear();
        self.index = 0;
        self.open = false;
        self.range = None;
        self.for_text.clear();
    }

    /// Reads the keys the list claimed last pass and the row the pointer picked, against the draft `text`. Returns
    /// the bytes of `text` the accepted candidate replaces and the text that replaces them.
    pub(crate) fn take(&mut self, ctx: &Context, id: Id, text: &str) -> Option<(Range<usize>, String)> {
        let claimed = |key| keys::take_claimed(ctx, id, Chord::plain(key));
        let (down, up, escape) = (claimed(Key::Down), claimed(Key::Up), claimed(Key::Escape));
        let accept = claimed(Key::Enter) | claimed(Key::Tab);
        let picked = self.picked.take();
        if !self.open {
            return None;
        }
        if escape {
            self.dismissed = Some(text.to_owned());
            self.close();
            return None;
        }
        if down {
            self.index = (self.index + 1).min(self.items.len().saturating_sub(1));
        }
        if up {
            self.index = self.index.saturating_sub(1);
        }
        let at = if accept { Some(self.index) } else { picked }?;
        if self.for_text != text {
            return None;
        }
        let range = self.range.clone()?;
        let insert = self.items.get(at)?.insert.clone();
        // What the draft becomes stays hidden: an accepted word does not offer itself again.
        self.dismissed = Some(format!("{}{insert}{}", text.get(..range.start)?, text.get(range.end..)?));
        self.close();
        Some((range, insert))
    }

    /// Asks `completer` for the candidates of the draft and caret in `at` (the text and the caret in characters; `None`
    /// while the part is not being typed in) and claims the list's keys while there are some.
    pub(crate) fn refresh(&mut self, ctx: &Context, id: Id, completer: Option<&dyn Completer>, at: Option<(&str, usize)>) {
        let Some((text, caret)) = at else {
            self.close();
            return;
        };
        if self.dismissed.as_deref().is_some_and(|dismissed| dismissed != text) {
            self.dismissed = None;
        }
        let mut items = Vec::new();
        let range = completer.and_then(|completer| completer.complete(text, byte_of(text, caret), &mut items));
        let Some(range) = range.filter(|_| !items.is_empty() && self.dismissed.is_none()) else {
            self.close();
            return;
        };
        if self.items.iter().map(|item| &item.insert).ne(items.iter().map(|item| &item.insert)) {
            self.index = 0;
        }
        self.index = self.index.min(items.len() - 1);
        self.items = items;
        self.range = Some(range);
        self.for_text = text.to_owned();
        self.open = true;
        for key in KEYS {
            keys::claim(ctx, id, Chord::plain(key));
        }
    }
}
