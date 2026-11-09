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
