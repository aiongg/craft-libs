//! Smart quotes (F14.07): straight quotes typed into text become the language's quotation marks
//! (Photoshop's Preferences › Type › Use Smart Quotes, on by default). The marks follow the
//! language of the text at the caret, as Photoshop's follow the Character panel's language.
//! Only typed text is converted; pasted text keeps its quotes (Photoshop).
//!
//! ```ignore
//! let style = QuoteStyle::for_language("de");
//! assert_eq!(smart_quotes("\"Ja\"", None, style), "„Ja“");
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// A language's quotation marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuoteStyle {
    /// The opening and closing marks a typed `"` becomes.
    pub double: (char, char),
    /// The opening and closing marks of a quotation inside a quotation, which a typed `'` becomes
    /// where it opens one.
    pub single: (char, char),
    /// What a typed `'` after a letter becomes: the apostrophe (’ in every language here), which
    /// also closes an English single quotation.
    pub apostrophe: char,
}

impl QuoteStyle {
    /// “English” and ‘English’.
    pub const ENGLISH: QuoteStyle = QuoteStyle { double: ('\u{201C}', '\u{201D}'), single: ('\u{2018}', '\u{2019}'), apostrophe: '\u{2019}' };
    /// „Deutsch“ and ‚Deutsch‘.
    pub const GERMAN: QuoteStyle = QuoteStyle { double: ('\u{201E}', '\u{201C}'), single: ('\u{201A}', '\u{2018}'), apostrophe: '\u{2019}' };
    /// «français» and “français” (CLDR's French marks; no spaces are inserted).
    pub const FRENCH: QuoteStyle = QuoteStyle { double: ('\u{00AB}', '\u{00BB}'), single: ('\u{201C}', '\u{201D}'), apostrophe: '\u{2019}' };
    /// 「日本語」 and 『日本語』.
    pub const JAPANESE: QuoteStyle = QuoteStyle { double: ('\u{300C}', '\u{300D}'), single: ('\u{300E}', '\u{300F}'), apostrophe: '\u{2019}' };

    /// The marks for a BCP 47 language tag (`"de"`, `"de-CH"`, `"fr-CA"`), by its primary subtag,
    /// region where CLDR differs (`de-CH` uses «» and ‹›). Unknown or empty tags give
    /// [`QuoteStyle::ENGLISH`].
    // new: PhotoCraft always uses English marks (pc: crates/ui-egui/src/tools/type_tool/editing.rs:186).
    // The table is CLDR's `delimiters` (quotationStart/End, alternateQuotationStart/End) for the
    // languages craft-text hyphenates, Japanese, Chinese and Korean.
    pub fn for_language(tag: &str) -> QuoteStyle {
        todo!()
    }
}

/// `s` as typed after the character `prev` (the one before the caret, `None` at the start of the
/// text), with straight quotes replaced: a quote opens after whitespace, an opening bracket or a
/// dash (or at the start), and closes elsewhere; `'` after a letter is the apostrophe. Each
/// converted character becomes `prev` for the next.
// pc: crates/ui-egui/src/tools/type_tool/editing.rs:186 `smart_quotes` and :212 `opens_quote`.
pub fn smart_quotes(s: &str, prev: Option<char>, style: QuoteStyle) -> String {
    todo!()
}
