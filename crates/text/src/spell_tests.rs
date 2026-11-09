//! The spelling engine on small word lists and the bundled dictionary.

use crate::spell::{Dictionary, UserWords, words};

// The bundled list decodes once, has about 72,000 words, and knows "the" and "Paris".
#[test]
fn bundled_dictionary_loads() {
    let d = Dictionary::english();
    assert!(d.len() > 70_000, "{}", d.len());
    let none = UserWords::default();
    for w in ["the", "color", "running", "Paris", "photographer's"] {
        assert!(d.check(w, &none), "{w}");
    }
    for w in ["teh", "recieve", "zxqv"] {
        assert!(!d.check(w, &none), "{w}");
    }
}

// "The" and "THE" pass for "the"; "paris" fails for "Paris"; "NASA's", "don’t" and "H2O" pass.
#[test]
fn case_possessive_and_digit_rules() {
    let d = Dictionary::english();
    let none = UserWords::default();
    for w in ["The", "THE", "PARIS", "NASA's", "don’t", "MOTHER’S", "H2O", "2026"] {
        assert!(d.check(w, &none), "{w}");
    }
    for w in ["paris", "MOTHER'SX"] {
        assert!(!d.check(w, &none), "{w}");
    }
}

// "teh" suggests "the" first (a transposition), and nearer words come before more common ones.
#[test]
fn suggestions_rank_by_distance_then_frequency() {
    let d = Dictionary::english();
    assert_eq!(d.suggest("teh", 3)[0], "the");
    assert!(d.suggest("recieve", 5).contains(&"receive".to_string()));
    assert_eq!(d.suggest("Wrold", 1), vec!["World".to_string()]);
    assert_eq!(d.suggest("brwon", 1), vec!["brown".to_string()]);
    assert!(d.suggest("HELO", 5)[0].chars().all(char::is_uppercase));
    // Among equal distances the lowest SCOWL level wins; a nearer word beats a more common one.
    let d = Dictionary::from_list("#10\ncart\n#50\ncare\ncarp\n");
    assert_eq!(d.suggest("carx", 3), vec!["cart", "care", "carp"]);
    let d = Dictionary::from_list("#10\nabcdef\n#50\nabcxyz\n");
    assert_eq!(d.suggest("abcxyf", 2), vec!["abcxyz", "abcdef"]);
}

// `misspellings` gives byte ranges in order and skips the user's words in any case.
#[test]
fn misspellings_skip_user_words() {
    let d = Dictionary::from_list("#10\nhello\nworld\nParis\n");
    let text = "Hello wrld, Paris paris Photocraft 42x";
    let found = d.misspellings(text, &UserWords::default());
    let got: Vec<&str> = found.iter().map(|m| m.word.as_str()).collect();
    assert_eq!(got, ["wrld", "paris", "Photocraft"]);
    assert_eq!(&text[found[0].range.clone()], "wrld");
    let user = UserWords::from_words(["photocraft".to_string()]);
    assert_eq!(d.misspellings(text, &user).len(), 2);
    let t = "it's well-known 'quoted' café";
    let w: Vec<&str> = words(t).into_iter().map(|r| &t[r]).collect();
    assert_eq!(w, ["it's", "well", "known", "quoted", "café"]);
}

// `UserWords::add` refuses a word already there in another case and keeps the list sorted.
#[test]
fn user_words_are_case_insensitive_and_sorted() {
    let mut u = UserWords::from_words(["zebra".to_string(), "Apple".to_string(), "APPLE".to_string()]);
    assert_eq!(u.words(), ["Apple", "zebra"]);
    assert!(!u.add("ZEBRA"));
    assert!(u.add("mango"));
    assert_eq!(u.words(), ["Apple", "mango", "zebra"]);
    assert!(u.contains("Mango's") && u.contains("APPLE"));
    assert!(u.remove("MANGO") && !u.remove("mango"));
    assert_eq!(u.words(), ["Apple", "zebra"]);
}

// A word added with a curly apostrophe is accepted when the text uses either apostrophe, is stored
// straight, and is found again by the curly spelling for removal.
