//! Find and replace (N3.03): the matcher behind a find bar or a Find and Replace dialog, and the
//! matches of a search across a paged document.
//!
//! - [`FindPattern`] reads the Find what text with [`FindOptions`]: Acrobat's two options (Match
//!   case, Whole words only), Word's search options (Match prefix, Match suffix, Ignore
//!   punctuation, Ignore white-space), Word's `^` codes (`^p` paragraph mark, `^t` tab, `^#` any
//!   digit, `^?` any character, …), Word's wildcards (`?`, `*`, `[a-z]`, `{n,m}`, `@`, `<`, `>`,
//!   `( )`) and regular expressions. Wildcards and expressions compile to a linear-time engine,
//!   so no pattern can backtrack catastrophically.
//! - [`parse_replacement`] and [`expand_replacement`] read and expand Replace with text: `^&`
//!   (the text found), `^c` (the clipboard), `\1`–`\9` with wildcards, `$n` and `${name}` with an
//!   expression.
//! - [`FindMatches`] holds the matches of a document, page by page, in document order, with the
//!   current match and Acrobat's rule for the first match a search shows (the first at or after
//!   the page the search started from, wrapping round).
//!
//! The app reads its text into one string per story or page (a paragraph mark,
//! [`PARAGRAPH_MARK`], between paragraphs) and maps the char ranges back to its own positions.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

/// A paragraph mark in the searched text: `^p` finds it.
pub const PARAGRAPH_MARK: char = '\r';

/// The mark of a paragraph that ends a section, in the searched text only: `^b` finds it.
pub const SECTION_MARK: char = '\u{1D}';

/// The longest Find what text, in characters (Word's 255 is too short for pasted text; WordCraft
/// allows 1024).
pub const MAX_PATTERN: usize = 1024;

/// The largest count in a wildcard `{n,m}`.
pub const MAX_COUNT: u32 = 1000;

/// How the Find what text is read. The default is a plain search ignoring case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FindOptions {
    pub match_case: bool,
    /// Whole words only: a match starts and ends at a word boundary.
    pub whole_word: bool,
    /// Match prefix: a match starts at a word's start.
    pub prefix: bool,
    /// Match suffix: a match ends at a word's end.
    pub suffix: bool,
    /// Ignore punctuation characters between the letters of the text found.
    pub ignore_punctuation: bool,
    /// Ignore white-space characters between the letters of the text found.
    pub ignore_space: bool,
    /// Word's Use wildcards. Wildcards always match case and turn off the five options above, as
    /// Word greys them out.
    pub wildcards: bool,
    /// A regular expression (WordCraft's own option; wins over `wildcards`).
    pub regex: bool,
    /// Take the text literally: no `^` codes (Acrobat and PowerPoint, which have none).
    pub literal: bool,
}

/// Why a Find what text can't be searched for. The app shows it under the field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindError {
    /// Longer than [`MAX_PATTERN`] characters.
    TooLong,
    /// A wildcard pattern Word would refuse: an unclosed `[` or `{`, a reversed range, a count
    /// above [`MAX_COUNT`]. The text names the problem in English, as WordCraft's engine does.
    Wildcards(String),
    /// A regular expression the engine refuses, with its message.
    Regex(String),
}

impl std::fmt::Display for FindError {
    // wordcraft: crates/engine/src/cmd/find_pattern.rs:142 `compile` (its error strings).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for FindError {}

/// A Find what text compiled with its options. `Send + Sync`: one pattern searches the pages of a
/// document on several threads.
#[derive(Clone, Debug)]
pub struct FindPattern {
    options: FindOptions,
    /// The plain needle (after `^` codes), lowercased unless `match_case`, for the plain path.
    needle: Vec<char>,
    /// The translated expression for wildcards, regular expressions and the ignore options.
    expression: Option<String>,
    /// The pattern can match a paragraph mark, so the app must search across paragraphs.
    spans_paragraphs: bool,
}

impl FindPattern {
    /// The pattern `query` stands for. A plain query with no `^` codes and none of the ignore
    /// options takes the plain path (PdfCraft's prefix-table search over chars); everything else
    /// is translated to an expression: `^` codes as WordCraft's `find_code`, wildcards as its
    /// `wildcards`, the ignore options as optional runs of punctuation or spaces between the
    /// characters.
    // wordcraft: crates/engine/src/cmd/find_pattern.rs:142 `compile`, :174 `special`, :221
    // `wildcards`; pdfcraft: crates/render/src/text/search.rs (the plain search with case and
    // whole words).
    pub fn compile(query: &str, options: &FindOptions) -> Result<FindPattern, FindError> {
        todo!()
    }

    pub fn options(&self) -> &FindOptions {
        &self.options
    }

    /// Whether the pattern can match a paragraph mark (`^p`, `^b`, an expression with `\r`): the
    /// app then searches a whole story as one text instead of paragraph by paragraph.
    pub fn spans_paragraphs(&self) -> bool {
        self.spans_paragraphs
    }

    /// Whether the query was empty (or only white space with Ignore white-space): a search
    /// finds nothing and the find bar shows no count.
    pub fn is_empty(&self) -> bool {
        self.needle.is_empty() && self.expression.is_none()
    }

    /// The char ranges of the matches in `text`, in order, not overlapping, at most `limit`
    /// (PdfCraft caps a page at its `limit` so a one-letter search can't build millions).
    // pdfcraft: crates/render/src/text.rs:142 `find_opts_limited`; wordcraft:
    // crates/engine/src/cmd/find_pattern.rs:142 (the compiled expression's use).
    pub fn find_all(&self, text: &str, limit: usize) -> Vec<Range<usize>> {
        todo!()
    }

    /// The match's groups for a replacement: group 0 is the match; `None` past the last group or
    /// for a group that took part in no match. Plain patterns have group 0 only.
    pub fn groups(&self, text: &str, found: Range<usize>) -> Vec<Option<Range<usize>>> {
        todo!()
    }
}

/// A piece of Replace with text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplacePiece {
    Literal(String),
    /// `^&`: the text found.
    Found,
    /// `\n` with wildcards, or `$n` / `${n}` / `\n` with an expression: group `n` (0 is the
    /// match).
    Group(usize),
    /// `${name}` / `$name` with an expression; [`FindPattern::groups`] has none by name, so the
    /// app resolves it with [`resolve_group_names`] or it expands to nothing.
    Named(String),
    /// `^c`: the clipboard.
    Clipboard,
}

/// Read Replace with text: `^` codes (unless `literal`), `\1`–`\9` with wildcards, and with an
/// expression also `$n`, `${n}`, `${name}`, `$name`, `$$`, `\n` (paragraph mark), `\t` and `\\`.
// wordcraft: crates/engine/src/cmd/find_pattern.rs:384 `parse_replace`.
pub fn parse_replacement(with: &str, options: &FindOptions) -> Vec<ReplacePiece> {
    todo!()
}

/// [`ReplacePiece::Named`] turned into [`ReplacePiece::Group`] where `index` knows the name.
// wordcraft: crates/engine/src/cmd/find_pattern.rs:491 `resolve_names`.
pub fn resolve_group_names(pieces: Vec<ReplacePiece>, index: &dyn Fn(&str) -> Option<usize>) -> Vec<ReplacePiece> {
    todo!()
}

/// The replacement text for one match: `found` is the matched text, `group(n)` its group `n`
/// (`None` expands to nothing), `clipboard` the clipboard's text.
// wordcraft: crates/engine/src/cmd/find_pattern.rs:507 `expand`.
pub fn expand_replacement(pieces: &[ReplacePiece], found: &str, group: &dyn Fn(usize) -> Option<String>, clipboard: &str) -> String {
    todo!()
}

/// One match: the page (or story) it is on, by the app's index, and its char range there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindMatch {
    pub page: usize,
    pub range: Range<usize>,
}

/// The matches of a search across a document, in document order, and the current one.
///
/// Pages report their text as it becomes available (PdfCraft extracts text in the background):
/// [`set_page`](Self::set_page) replaces one page's matches and keeps the current match where it
/// was. Until the user picks a match or moves the view, the current match follows the page the
/// search started from (Acrobat): a page that arrives late may still supply an earlier match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindMatches {
    matches: Vec<FindMatch>,
    current: Option<usize>,
    /// The page the search started from, while the current match is chosen automatically.
    from: Option<usize>,
    /// The user ended automatic following (picked a match, navigated, scrolled).
    follow_ended: bool,
}

impl FindMatches {
    /// No matches; a new search starts from page `from`.
    pub fn new(from: usize) -> Self {
        FindMatches { matches: Vec::new(), current: None, from: Some(from), follow_ended: false }
    }

    pub fn matches(&self) -> &[FindMatch] {
        &self.matches
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// The current match's index, for "3 of 12".
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    pub fn current_match(&self) -> Option<&FindMatch> {
        self.current.and_then(|i| self.matches.get(i))
    }

    /// Replace page `page`'s matches with `ranges` (sorted by start). The current match stays on
    /// the same page and range when it is still there; while following, the current match becomes
    /// the first at or after the search's start page. Returns whether the current match changed,
    /// so the app scrolls to it (only while following: a match the user already moved away from
    /// is listed, never revealed).
    // pdfcraft: crates/ui-egui/src/canvas/find.rs:120 `refresh_find_page`, :54 `Find::follow_page`.
    pub fn set_page(&mut self, page: usize, ranges: Vec<Range<usize>>) -> bool {
        todo!()
    }

    /// The first match at or after `page`, or the first match when none follows: a search wraps
    /// round the document.
    // pdfcraft: crates/ui-egui/src/canvas/find.rs:45 `Find::first_at_or_after`.
    pub fn first_at_or_after(&self, page: usize) -> Option<usize> {
        todo!()
    }

    /// Next (or with `back` the previous) match, wrapping round; the user's choice, so following
    /// ends. `None` without matches.
    pub fn step(&mut self, back: bool) -> Option<&FindMatch> {
        todo!()
    }

    /// The user moved the view (page box, thumbnails, outline, scrolling): following ends, and a
    /// page that arrives later no longer moves the current match.
    pub fn stop_following(&mut self) {
        self.from = None;
        self.follow_ended = true;
    }

    /// The user picked match `index` (Search panel row); following ends.
    pub fn pick(&mut self, index: usize) {
        todo!()
    }
}
