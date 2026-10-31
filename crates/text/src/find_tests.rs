//! Find patterns, replacements and the matches across pages.

// "cat" finds three ranges in "Cat cat concat" ignoring case, two with whole words, one with match case and whole words.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn plain_search_with_case_and_whole_words() {
    todo!()
}

// ^p matches a paragraph mark and makes the pattern span paragraphs; <(c?t)> with wildcards finds "cat" and "cut" as whole words; an unclosed [ is a Wildcards error.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn word_codes_and_wildcards_compile() {
    todo!()
}

// "ab" finds "a.b" and "a b" with the two ignore options on.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn ignore_punctuation_and_space_match_across_them() {
    todo!()
}

// "^& \2\1" with wildcards over (a)(b) expands to "ab ba"; ^c inserts the clipboard text.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn replacement_expands_found_groups_and_clipboard() {
    todo!()
}

// Pages 3 then 1 arriving after a search from page 2 make page 3's first match current; after stop_following page 2's late matches don't move it; step wraps round.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn matches_follow_the_start_page_until_the_user_moves() {
    todo!()
}
