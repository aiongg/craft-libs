//! List numbering and number styles.

use super::lists::{ListMarker, ListParagraph, NumberPunctuation, NumberStyle, line_starts, markers};

// Each style writes 1, 4, 9, 27 and 4000 as DesignCraft does (roman, letters AA, symbols doubled, kanji digit by digit; 4000 in Arabic digits).
#[test]
fn number_styles_format_and_cap_at_3999() {
    let row = |style: NumberStyle| [1, 4, 9, 27, 4000].map(|n| style.format(n));
    assert_eq!(row(NumberStyle::Arabic), ["1", "4", "9", "27", "4000"]);
    assert_eq!(row(NumberStyle::UpperRoman), ["I", "IV", "IX", "XXVII", "4000"]);
    assert_eq!(row(NumberStyle::LowerRoman), ["i", "iv", "ix", "xxvii", "4000"]);
    assert_eq!(row(NumberStyle::UpperLetters), ["A", "D", "I", "AA", "4000"]);
    assert_eq!(row(NumberStyle::LowerLetters), ["a", "d", "i", "aa", "4000"]);
    assert_eq!(row(NumberStyle::ArabicLeadingZero), ["01", "04", "09", "27", "4000"]);
    assert_eq!(row(NumberStyle::ArabicThreeDigits), ["001", "004", "009", "027", "4000"]);
    assert_eq!(row(NumberStyle::ArabicFourDigits), ["0001", "0004", "0009", "0027", "4000"]);
    assert_eq!(row(NumberStyle::Symbols), ["*", "§", "‡‡", "‡‡‡‡‡", "4000"]);
    assert_eq!(row(NumberStyle::Kanji), ["一", "四", "九", "二七", "四〇〇〇"]);
    assert_eq!(NumberStyle::LowerRoman.sample(), "i, ii, iii, iv…");
    assert_eq!(NumberStyle::ArabicThreeDigits.sample(), "001, 002, 003…");
    assert_eq!(ListMarker::number_text(NumberStyle::LowerLetters, NumberPunctuation::ParenBoth, 3), "(c)");
}

// 1. 2. then a bullet at level 0 restarts; a level-1 run between level-0 numbers keeps them counting; empty paragraphs neither show nor count.
#[test]
fn numbers_count_per_level_and_restart_after_an_outer_unnumbered_paragraph() {
    let number = |start| ListMarker::Number { style: NumberStyle::Arabic, start, punctuation: NumberPunctuation::Period };
    let mut empty = ListParagraph::new(0, Some(number(1)));
    empty.empty = true;
    let paragraphs = [
        ListParagraph::new(0, Some(number(1))),
        ListParagraph::new(1, Some(number(1))),
        ListParagraph::new(1, Some(number(1))),
        empty,
        ListParagraph::new(0, Some(number(1))),
        ListParagraph::new(0, Some(ListMarker::Bullet("•".into()))),
        ListParagraph::new(0, Some(number(5))),
        ListParagraph::new(0, None),
        ListParagraph::new(0, Some(number(1))),
    ];
    let shown = [Some("1."), Some("1."), Some("2."), None, Some("2."), Some("•"), Some("5."), None, Some("1.")];
    assert_eq!(markers(&paragraphs).iter().map(Option::as_deref).collect::<Vec<_>>(), shown);
}

// A level-2 paragraph without a margin starts at 72 pt; a hanging indent of -18 puts the first line 18 pt left of the rest.
#[test]
fn line_starts_hang_the_marker() {
    assert_eq!(line_starts(2, None, None), (72.0, 72.0));
    assert_eq!(line_starts(1, Some(54.0), Some(-18.0)), (36.0, 54.0));
}
