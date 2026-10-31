//! Smart quotes (F14.07): straight quotes typed into text become the language's quotation marks
//! (Photoshop's Preferences › Type › Use Smart Quotes, on by default). The marks follow the
//! language of the text at the caret, as Photoshop's follow the Character panel's language.
//! Only typed text is converted; pasted text keeps its quotes (Photoshop).
//!
//! ```ignore
//! let style = QuoteStyle::for_language("de");
//! assert_eq!(smart_quotes("\"Ja\"", None, style), "„Ja“");
//! ```

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
        let lower = tag.to_ascii_lowercase();
        let mut parts = lower.split(['-', '_']);
        let lang = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();
        let region = |r: &[&str]| rest.iter().any(|p| r.contains(p));
        let marks = |d: (char, char), s: (char, char)| QuoteStyle { double: d, single: s, apostrophe: '\u{2019}' };
        const GUILLEMETS: ((char, char), (char, char)) = (('\u{00AB}', '\u{00BB}'), ('\u{2039}', '\u{203A}'));
        const LOW_HIGH: ((char, char), (char, char)) = (('\u{201E}', '\u{201C}'), ('\u{201A}', '\u{2018}'));
        let pair = |(d, s): ((char, char), (char, char))| marks(d, s);
        match lang {
            "de" if region(&["ch", "li"]) => pair(GUILLEMETS),
            "fr" if region(&["ch"]) => pair(GUILLEMETS),
            "it" if region(&["ch"]) => pair(GUILLEMETS),
            "de" => QuoteStyle::GERMAN,
            "fr" => QuoteStyle::FRENCH,
            "es" | "it" | "el" => marks(('\u{00AB}', '\u{00BB}'), ('\u{201C}', '\u{201D}')),
            "pt" if region(&["pt"]) => marks(('\u{00AB}', '\u{00BB}'), ('\u{201C}', '\u{201D}')),
            "ja" => QuoteStyle::JAPANESE,
            "zh" if region(&["hant", "tw", "hk", "mo"]) => QuoteStyle::JAPANESE,
            "cs" | "sk" | "sl" | "hr" | "sr" | "bg" | "lt" | "et" | "ka" | "is" => pair(LOW_HIGH),
            "pl" => marks(('\u{201E}', '\u{201D}'), ('\u{201A}', '\u{2019}')),
            "hu" => marks(('\u{201E}', '\u{201D}'), ('\u{00BB}', '\u{00AB}')),
            "ro" => marks(('\u{201E}', '\u{201D}'), ('\u{00AB}', '\u{00BB}')),
            "ru" | "uk" | "be" | "kk" => marks(('\u{00AB}', '\u{00BB}'), ('\u{201E}', '\u{201C}')),
            "lv" => marks(('\u{00AB}', '\u{00BB}'), ('\u{201E}', '\u{201C}')),
            "hy" => marks(('\u{00AB}', '\u{00BB}'), ('\u{00AB}', '\u{00BB}')),
            "sv" | "fi" => marks(('\u{201D}', '\u{201D}'), ('\u{2019}', '\u{2019}')),
            "da" => marks(('\u{00BB}', '\u{00AB}'), ('\u{203A}', '\u{2039}')),
            "nb" | "no" | "nn" => marks(('\u{00AB}', '\u{00BB}'), ('\u{2018}', '\u{2019}')),
            _ => QuoteStyle::ENGLISH,
        }
    }
}

/// `s` as typed after the character `prev` (the one before the caret, `None` at the start of the
/// text), with straight quotes replaced: a quote opens after whitespace, an opening bracket or a
/// dash (or at the start), and closes elsewhere; `'` after a letter is the apostrophe. Each
/// converted character becomes `prev` for the next.
// pc: crates/ui-egui/src/tools/type_tool/editing.rs:186 `smart_quotes` and :212 `opens_quote`.
pub fn smart_quotes(s: &str, mut prev: Option<char>, style: QuoteStyle) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let c = match c {
            '"' if opens_quote(prev) => style.double.0,
            '"' => style.double.1,
            '\'' if opens_quote(prev) => style.single.0,
            '\'' if prev.is_some_and(char::is_alphanumeric) => style.apostrophe,
            '\'' => style.single.1,
            _ => c,
        };
        out.push(c);
        prev = Some(c);
    }
    out
}

fn opens_quote(prev: Option<char>) -> bool {
    prev.is_none_or(|p| p.is_whitespace() || matches!(p, '(' | '[' | '{' | '-' | '\u{2013}' | '\u{2014}'))
}
