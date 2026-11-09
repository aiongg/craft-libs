//! Smart quotes on typed text.

use crate::quotes::*;

// English: `"` after a space opens “, after a letter closes ”; `'` inside "don't" is ’.
#[test]
fn quotes_open_after_space_and_close_after_letters() {
    let e = QuoteStyle::ENGLISH;
    assert_eq!(smart_quotes("say \"hi\" now", Some('x'), e), "say \u{201C}hi\u{201D} now");
    assert_eq!(smart_quotes("\"hi\"", Some(' '), e), "\u{201C}hi\u{201D}");
    assert_eq!(smart_quotes("don't", Some(' '), e), "don\u{2019}t");
    assert_eq!(smart_quotes("'a'", Some(' '), e), "\u{2018}a\u{2019}");
    assert_eq!(smart_quotes("x.\"", None, e), "x.\u{201D}");
}

// A quote after "(", "[", "{" or a dash opens; at the start of the text (`prev` None) it opens.
#[test]
fn quotes_open_after_brackets_dashes_and_at_the_start() {
    let e = QuoteStyle::ENGLISH;
    for p in ['(', '[', '{', '-', '\u{2013}', '\u{2014}'] {
        assert_eq!(smart_quotes("\"", Some(p), e), "\u{201C}", "after {p:?}");
    }
    assert_eq!(smart_quotes("\"a\"", None, e), "\u{201C}a\u{201D}");
    assert_eq!(smart_quotes("'", None, e), "\u{2018}");
    assert_eq!(smart_quotes("\"", Some('a'), e), "\u{201D}");
}

// `for_language`: "de" gives „…“, "de-CH" «…», "fr" «…», "ja" 「…」, "xx" and "" English.
#[test]
fn marks_follow_the_language() {
    assert_eq!(smart_quotes("\"Ja\"", None, QuoteStyle::for_language("de")), "\u{201E}Ja\u{201C}");
    assert_eq!(smart_quotes("\"Ja\"", None, QuoteStyle::for_language("de-CH")), "\u{201E}Ja\u{201C}");
    assert_eq!(smart_quotes("\"a\"", None, QuoteStyle::for_language("fr_CA")), "\u{00AB}a\u{00BB}");
    assert_eq!(QuoteStyle::for_language("ja"), QuoteStyle::JAPANESE);
    assert_eq!(QuoteStyle::for_language("zh-Hant"), QuoteStyle::JAPANESE);
    assert_eq!(QuoteStyle::for_language("zh-Hans"), QuoteStyle::ENGLISH);
    assert_eq!(QuoteStyle::for_language("xx"), QuoteStyle::ENGLISH);
    assert_eq!(QuoteStyle::for_language(""), QuoteStyle::ENGLISH);
}
