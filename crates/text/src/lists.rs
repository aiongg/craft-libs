//! Lists (N3.23): the bullet or number at the start of each paragraph of a list, numbered per
//! level, and the number styles (1, i, A, 01…) that list numbers and page numbers are written in.
//!
//! The app lays its paragraphs out; this module says what marker each one gets and where its
//! first and following lines start. A notes editor ([`crate::ui::notes`]), a slide's text box and
//! a document's lists use the same rules, so the numbering matches PowerPoint's and Word's.
//!
//! ```
//! use craft_text::lists::{ListMarker, ListParagraph, NumberPunctuation, NumberStyle, markers};
//!
//! let numbered = ListMarker::Number { style: NumberStyle::Arabic, start: 1, punctuation: NumberPunctuation::Period };
//! let paragraphs = [
//!     ListParagraph::new(0, Some(numbered.clone())),
//!     ListParagraph::new(1, Some(ListMarker::Bullet("•".into()))),
//!     ListParagraph::new(0, Some(numbered)),
//! ];
//! # let _ = || {
//! assert_eq!(markers(&paragraphs), [Some("1.".to_string()), Some("•".to_string()), Some("2.".to_string())]);
//! # };
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// Numbers past this are written in Arabic digits by the styles that repeat characters (roman
/// numerals, letters, symbols), so a hostile start number can't build a huge label
/// (DesignCraft's `MAX_REPEATED_FORM`).
pub const MAX_REPEATED_FORM: u32 = 3999;

/// The deepest list level; deeper paragraphs count as this one (PowerPoint's nine levels).
pub const MAX_LEVEL: u8 = 8;

/// The indent a level without its own margin gets, per level, in points (PowerPoint's default
/// notes master).
pub const LEVEL_INDENT: f64 = 36.0;

/// How a number is written. The same styles serve list numbers and page numbers
/// (`craft_pages::numbering`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum NumberStyle {
    /// 1, 2, 3
    #[default]
    Arabic,
    /// I, II, III
    UpperRoman,
    /// i, ii, iii
    LowerRoman,
    /// A, B, … Z, AA, BB (InDesign's and Word's letters repeat the letter)
    UpperLetters,
    /// a, b, … z, aa, bb
    LowerLetters,
    /// 01, 02, …
    ArabicLeadingZero,
    /// 001, 002, …
    ArabicThreeDigits,
    /// 0001, 0002, …
    ArabicFourDigits,
    /// *, †, ‡, §, ¶, ‖, then doubled (**, ††, …): footnotes.
    Symbols,
    /// 一, 二, 三, …: Chinese numerals written digit by digit, so 10 is 一〇 and 38 is 三八.
    Kanji,
}

impl NumberStyle {
    /// Every style, in the order a Style menu lists them: DesignCraft's page numbering order,
    /// then Symbols and Kanji.
    pub const ALL: [NumberStyle; 10] = [
        NumberStyle::Arabic,
        NumberStyle::UpperRoman,
        NumberStyle::LowerRoman,
        NumberStyle::UpperLetters,
        NumberStyle::LowerLetters,
        NumberStyle::ArabicLeadingZero,
        NumberStyle::ArabicThreeDigits,
        NumberStyle::ArabicFourDigits,
        NumberStyle::Symbols,
        NumberStyle::Kanji,
    ];

    /// `n` in this style. A number above [`MAX_REPEATED_FORM`] is written in Arabic digits by
    /// the roman, letter and symbol styles. 0 is "0" in the Arabic styles and an empty string in
    /// the others.
    // designcraft: crates/doc/src/attrs/rules_lists.rs:144 `NumberStyle::format` (roman, letters,
    // symbols, kanji_digits; the 3999 cap).
    pub fn format(self, n: u32) -> String {
        todo!()
    }

    /// The menu sample: the first four numbers and "…" ("1, 2, 3, 4…", "i, ii, iii, iv…",
    /// "01, 02, 03…" for the zero-padded styles, which show three). Language-neutral, so menus
    /// need no translated word for a style.
    // designcraft: crates/ui-egui/src/section_options.rs:14 `STYLES` (the samples it spells out).
    pub fn sample(self) -> String {
        todo!()
    }
}

/// What follows (or surrounds) a list number: PowerPoint's autonumber schemes `arabicPeriod`,
/// `arabicParenR`, `arabicParenBoth`, `arabicPlain` and their roman and letter forms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum NumberPunctuation {
    /// "1."
    #[default]
    Period,
    /// "1)"
    ParenRight,
    /// "(1)"
    ParenBoth,
    /// "1"
    Plain,
}

/// The marker a paragraph of a list starts with.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum ListMarker {
    /// A bullet: the character (or short text) as the app resolved it from its bullet font; a
    /// picture bullet is "•".
    Bullet(String),
    /// A number counted per level from `start`.
    Number { style: NumberStyle, start: u32, punctuation: NumberPunctuation },
}

impl ListMarker {
    /// The number `n` (already counted from `start`) with its punctuation: "3.", "iv)", "(c)".
    // deckcraft: crates/ui-egui/src/panels/notes.rs:79 (`deckcraft_text::autonum(scheme, n)`).
    pub fn number_text(style: NumberStyle, punctuation: NumberPunctuation, n: u32) -> String {
        todo!()
    }
}

/// One paragraph as numbering sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListParagraph {
    /// 0 is the outermost level; deeper than [`MAX_LEVEL`] counts as [`MAX_LEVEL`].
    pub level: u8,
    pub marker: Option<ListMarker>,
    /// An empty paragraph shows no marker and doesn't count.
    pub empty: bool,
}

impl ListParagraph {
    /// A non-empty paragraph at `level`.
    pub fn new(level: u8, marker: Option<ListMarker>) -> Self {
        ListParagraph { level, marker, empty: false }
    }
}

/// Each paragraph's marker text, numbered as PowerPoint numbers a text box: per level, a number
/// counts on from the last number at its level and restarts after a paragraph at the same or a
/// lower (outer) level that isn't numbered; a deeper paragraph doesn't interrupt it. Empty
/// paragraphs get `None` and don't count. A number starts at its marker's `start`.
// deckcraft: crates/ui-egui/src/panels/notes.rs:62 `bullets` (the per-level counters and their
// reset rule).
pub fn markers(paragraphs: &[ListParagraph]) -> Vec<Option<String>> {
    todo!()
}

/// Where a paragraph's first line and its other lines start, in points from the text box's
/// start edge: `margin` (or [`LEVEL_INDENT`] × level when the paragraph has none) for the other
/// lines, `margin + indent` for the first. A negative `indent` is a hanging indent: the marker
/// sits in it and the text of every line lines up.
// deckcraft: crates/ui-egui/src/panels/notes.rs:54 `indents`.
pub fn line_starts(level: u8, margin: Option<f64>, indent: Option<f64>) -> (f64, f64) {
    todo!()
}
