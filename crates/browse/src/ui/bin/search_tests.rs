//! `name_matches`.

use super::name_matches;

// FilmCraft's `every_typed_word_must_appear_in_any_order`; `ÉTÉ` matches `été`.
#[test]
fn a_search_matches_words_in_any_order() {
    assert!(name_matches("Summer Beach.mov", "beach summer"));
    assert!(name_matches("Summer Beach.mov", "  MOV "));
    assert!(name_matches("été.mov", "ÉTÉ"));
    assert!(name_matches("anything", ""));
    assert!(name_matches("anything", "   "));
    assert!(!name_matches("Summer Beach.mov", "beach winter"));
}
