//! Smart quotes on typed text.

// English: `"` after a space opens “, after a letter closes ”; `'` inside "don't" is ’.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn quotes_open_after_space_and_close_after_letters() {
    todo!()
}

// A quote after "(", "[", "{" or a dash opens; at the start of the text (`prev` None) it opens.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn quotes_open_after_brackets_dashes_and_at_the_start() {
    todo!()
}

// `for_language`: "de" gives „…“, "de-CH" «…», "fr" «…», "ja" 「…」, "xx" and "" English.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn marks_follow_the_language() {
    todo!()
}
