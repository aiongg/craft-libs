//! Spelling (F14.06): Edit › Check Spelling, and the suggestions in the type tool's context menu
//! over a misspelled word.
//!
//! [`Dictionary::english`] is SCOWL size 50 (American English, about 72,000 words), deflated into
//! the binary and decoded on first use. Words are grouped by SCOWL level (10 = most common … 50),
//! which ranks suggestions. Other languages load their own list with [`Dictionary::from_list`].
//!
//! Case rules follow common spell checkers: a lowercase dictionary word may be written
//! capitalised or in all caps; a capitalised entry (a proper noun) must keep its capital. A
//! trailing possessive `'s` is ignored, typographic apostrophes count as `'`, and tokens with
//! digits are skipped. The user's own words ([`UserWords`], the dialog's Add button) are accepted
//! in any case.
//!
//! ```ignore
//! let user = UserWords::from_words(settings.user_words.clone());
//! for m in Dictionary::english().misspellings("Teh cat", &user) {
//!     let suggestions = Dictionary::english().suggest(&m.word, 8);
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::ops::Range;

/// A word list with frequency levels.
pub struct Dictionary {
    /// Word (as listed) → SCOWL level.
    words: HashMap<String, u8>,
    /// Lowercase form → listed forms, for case-insensitive lookups and suggestions.
    by_lower: HashMap<String, Vec<String>>,
    /// Lowercase words grouped by char length, with their chars and level, for suggestions.
    by_len: Vec<Vec<(String, Vec<char>, u8)>>,
}

/// A misspelled word in a checked string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misspelling {
    /// Byte range in the checked text.
    pub range: Range<usize>,
    /// The word as written.
    pub word: String,
}

/// The user's own dictionary: words added with the Check Spelling dialog's Add button, accepted
/// in any case. Kept in the order the dialog lists them (case-insensitive alphabetical); the app
/// stores [`words`](Self::words) in its settings (`ui::spelling::SpellingSettings`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserWords {
    /// As added, sorted by their lowercase form, no two equal ignoring case.
    words: Vec<String>,
}

impl UserWords {
    /// From a stored list; duplicates (ignoring case) are dropped and the rest sorted.
    // pc: crates/engine/src/cmd/type_spell_cmds.rs:75 (the lowercase set built from
    // `prefs.user_dictionary`).
    pub fn from_words(words: impl IntoIterator<Item = String>) -> Self {
        todo!()
    }

    /// Add `word`; `false` when it is already there in any case.
    // pc: crates/engine/src/cmd/type_spell_cmds.rs:185 (case-insensitive check, then sorted by
    // lowercase).
    pub fn add(&mut self, word: &str) -> bool {
        todo!()
    }

    /// Remove `word` in any case; `false` when it wasn't there.
    // new: PhotoCraft has no removal; Photoshop's dictionary editing is outside the app.
    pub fn remove(&mut self, word: &str) -> bool {
        todo!()
    }

    /// Whether `word` (possessive and apostrophes normalised as in [`Dictionary::check`]) is one
    /// of the user's words, ignoring case.
    // pc: crates/text/src/spell.rs:89 `check` (the `user` set lookup).
    pub fn contains(&self, word: &str) -> bool {
        todo!()
    }

    /// The words, for the app's settings.
    pub fn words(&self) -> &[String] {
        &self.words
    }
}

impl Dictionary {
    /// The bundled US English dictionary (SCOWL 50), decoded once.
    // pc: crates/text/src/spell.rs:39 `english`. The asset is
    // `assets/spell/en-us-scowl-50.txt.deflate` (raw deflate, as `hyphen::Dictionary`'s data;
    // PhotoCraft ships gzip), decoded with `miniz_oxide::inflate::decompress_to_vec`.
    pub fn english() -> &'static Dictionary {
        todo!()
    }

    /// A dictionary from one word per line; a `#NN` line sets the SCOWL level of the words after
    /// it (50 before any).
    // pc: crates/text/src/spell.rs:49 `from_list`.
    pub fn from_list(list: &str) -> Dictionary {
        todo!()
    }

    /// The number of words.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether `word` is spelled correctly: in the dictionary under the case rules above, one of
    /// `user`'s words, or a token the checker skips (one with a digit, or with no letter).
    // pc: crates/text/src/spell.rs:89 `check`.
    pub fn check(&self, word: &str, user: &UserWords) -> bool {
        todo!()
    }

    /// Up to `max` suggestions for `word`: dictionary words within edit distance 1 (words of up to
    /// four letters) or 2 (Damerau: a transposition counts 1), nearest first; among equals, the
    /// same letters in another order first, then the most common (lowest SCOWL level). Written in
    /// `word`'s case (capitalised or all caps).
    // pc: crates/text/src/spell.rs:114 `suggest` and :220 `distance`.
    pub fn suggest(&self, word: &str, max: usize) -> Vec<String> {
        todo!()
    }

    /// Every misspelled word in `text`, in order.
    // pc: crates/text/src/spell.rs:169 `misspellings`.
    pub fn misspellings(&self, text: &str, user: &UserWords) -> Vec<Misspelling> {
        todo!()
    }
}

/// The words of `text` as byte ranges: runs of letters and digits with inner apostrophes
/// (straight, typographic or U+02BC), so "don’t" is one word; hyphens split ("well-known" is two).
// pc: crates/text/src/spell.rs:192 `words`.
pub fn words(text: &str) -> Vec<Range<usize>> {
    todo!()
}
