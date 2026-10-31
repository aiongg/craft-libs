//! The spelling engine on small word lists and the bundled dictionary.

// The bundled list decodes once, has about 72,000 words, and knows "the" and "Paris".
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn bundled_dictionary_loads() {
    todo!()
}

// "The" and "THE" pass for "the"; "paris" fails for "Paris"; "NASA's", "don’t" and "H2O" pass.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn case_possessive_and_digit_rules() {
    todo!()
}

// "teh" suggests "the" first (a transposition), and nearer words come before more common ones.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn suggestions_rank_by_distance_then_frequency() {
    todo!()
}

// `misspellings` gives byte ranges in order and skips the user's words in any case.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn misspellings_skip_user_words() {
    todo!()
}

// `UserWords::add` refuses a word already there in another case and keeps the list sorted.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn user_words_are_case_insensitive_and_sorted() {
    todo!()
}
