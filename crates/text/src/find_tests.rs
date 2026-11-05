//! Find patterns, replacements and the matches across pages.

use crate::find::{
    FindError, FindMatches, FindOptions, FindPattern, PARAGRAPH_MARK, RegexError, ReplacePiece, WildcardError, expand_replacement, parse_replacement,
    resolve_group_names,
};

fn span(start: usize, end: usize) -> std::ops::Range<usize> {
    start..end
}

#[expect(clippy::unwrap_used, reason = "a test pattern that does not compile fails the test")]
fn compile(query: &str, options: FindOptions) -> FindPattern {
    let compiled = FindPattern::compile(query, &options);
    assert!(compiled.is_ok(), "{query:?} does not compile: {compiled:?}");
    compiled.ok().unwrap()
}

// "cat" finds three ranges in "Cat cat concat" ignoring case, two with whole words, one with match case and whole words.
#[test]
fn plain_search_with_case_and_whole_words() {
    let text = "Cat cat concat";
    assert_eq!(compile("cat", FindOptions::default()).find_all(text, 100), [0..3, 4..7, 11..14]);
    assert_eq!(compile("cat", FindOptions { whole_word: true, ..Default::default() }).find_all(text, 100), [0..3, 4..7]);
    assert_eq!(compile("cat", FindOptions { whole_word: true, match_case: true, ..Default::default() }).find_all(text, 100), [span(4, 7)]);
    assert_eq!(compile("cat", FindOptions::default()).find_all(text, 2).len(), 2);
}

// ^p matches a paragraph mark and makes the pattern span paragraphs; <(c?t)> with wildcards finds "cat" and "cut" as whole words; an unclosed [ is a Wildcards error.
#[test]
fn word_codes_and_wildcards_compile() {
    let paragraph = compile("a^pb", FindOptions::default());
    assert!(paragraph.spans_paragraphs());
    assert_eq!(paragraph.find_all(&format!("xa{PARAGRAPH_MARK}b"), 10), [span(1, 4)]);

    let wild = FindOptions { wildcards: true, ..Default::default() };
    let pattern = compile("<(c?t)>", wild);
    assert_eq!(pattern.find_all("cat cut cart concat", 10), [0..3, 4..7]);

    assert!(matches!(FindPattern::compile("a[b", &wild), Err(FindError::Wildcards(_))));
    assert!(matches!(FindPattern::compile("(", &FindOptions { regex: true, ..Default::default() }), Err(FindError::Regex(_))));
    assert_eq!(FindPattern::compile(&"a".repeat(2000), &FindOptions::default()).unwrap_err(), FindError::TooLong);
}

// "ab" finds "a.b" and "a b" with the two ignore options on.
#[test]
fn ignore_punctuation_and_space_match_across_them() {
    let options = FindOptions { ignore_punctuation: true, ignore_space: true, ..Default::default() };
    assert_eq!(compile("ab", options).find_all("a.b a b ab a-x", 10), [0..3, 4..7, 8..10]);
    assert!(compile("  ", options).is_empty());
}

// "^& \2\1" with wildcards over (a)(b) expands to "ab ba"; ^c inserts the clipboard text.
#[test]
fn replacement_expands_found_groups_and_clipboard() {
    let options = FindOptions { wildcards: true, ..Default::default() };
    let pattern = compile("(a)(b)", options);
    let text = "xab";
    let found = pattern.find_all(text, 10)[0].clone();
    let groups = pattern.groups(text, found.clone());
    let chars: Vec<char> = text.chars().collect();
    let group = |n: usize| groups.get(n).cloned().flatten().map(|r| chars[r].iter().collect::<String>());
    let pieces = parse_replacement("^& \\2\\1", &options);
    assert_eq!(expand_replacement(&pieces, "ab", &group, ""), "ab ba");
    assert_eq!(expand_replacement(&parse_replacement("<^c>", &options), "ab", &group, "clip"), "<clip>");

    let named = FindOptions { regex: true, ..Default::default() };
    let pieces = resolve_group_names(parse_replacement("${x}$1", &named), &|name| (name == "x").then_some(2));
    assert_eq!(pieces, [ReplacePiece::Group(2), ReplacePiece::Group(1)]);
}

// Pages 3 then 1 arriving after a search from page 2 make page 3's first match current; after stop_following page 2's late matches don't move it; step wraps round.
#[test]
fn matches_follow_the_start_page_until_the_user_moves() {
    let mut matches = FindMatches::new(2);
    assert!(matches.set_page(3, vec![5..8, 9..12]));
    assert_eq!(matches.current_match().map(|m| m.page), Some(3));
    assert!(!matches.set_page(1, vec![span(0, 3)]));
    assert_eq!(matches.current(), Some(1));

    matches.stop_following();
    assert!(!matches.set_page(2, vec![span(4, 7)]));
    assert_eq!(matches.current_match().map(|m| (m.page, m.range.clone())), Some((3, 5..8)));

    assert_eq!(matches.step(false).map(|m| m.range.clone()), Some(9..12));
    assert_eq!(matches.step(false).map(|m| m.page), Some(1));
    assert_eq!(matches.step(true).map(|m| m.page), Some(3));
    matches.pick(1);
    assert_eq!(matches.current(), Some(1));
}

fn refused(query: &str, options: FindOptions) -> FindError {
    FindPattern::compile(query, &options).err().unwrap_or(FindError::TooLong)
}

// Each refusal is a kind the UI words itself: wildcard problems as WordCraft names them, regex
// refusals by what is wrong.
#[test]
fn refusals_are_kinds_not_english() {
    let wild = FindOptions { wildcards: true, ..Default::default() };
    let regex = FindOptions { regex: true, ..Default::default() };
    let wildcard = |query: &str| refused(query, wild);
    assert_eq!(wildcard("a{2"), FindError::Wildcards(WildcardError::UnclosedBrace));
    assert_eq!(wildcard("a[b"), FindError::Wildcards(WildcardError::UnclosedBracket));
    assert_eq!(wildcard("(a)\\1"), FindError::Wildcards(WildcardError::BackReference));
    assert_eq!(wildcard("a{x}"), FindError::Wildcards(WildcardError::BadCount("x".into())));
    assert_eq!(wildcard("a{2,1}"), FindError::Wildcards(WildcardError::BadCount("2,1".into())));
    assert_eq!(wildcard("a{5000}"), FindError::Wildcards(WildcardError::CountTooLarge));
    assert_eq!(wildcard("[]"), FindError::Wildcards(WildcardError::EmptyClass));
    assert_eq!(wildcard("[z-a]"), FindError::Wildcards(WildcardError::ReversedRange { from: 'z', to: 'a' }));
    assert_eq!(wildcard("a)"), FindError::Wildcards(WildcardError::Refused));

    let expression = |query: &str| refused(query, regex);
    assert_eq!(expression("(a"), FindError::Regex(RegexError::UnclosedGroup));
    assert_eq!(expression("a)"), FindError::Regex(RegexError::UnopenedGroup));
    assert_eq!(expression("[a"), FindError::Regex(RegexError::UnclosedClass));
    assert_eq!(expression("*a"), FindError::Regex(RegexError::NothingToRepeat));
    assert_eq!(expression("a{2,1}"), FindError::Regex(RegexError::BadRepetition));
    assert_eq!(expression("[z-a]"), FindError::Regex(RegexError::ReversedRange));
    assert_eq!(expression("\\q"), FindError::Regex(RegexError::BadEscape));
    assert_eq!(expression("(?=a)"), FindError::Regex(RegexError::Unsupported));
    assert_eq!(expression("(?P<>a)"), FindError::Regex(RegexError::BadGroupName));
    assert_eq!(expression("a{1000}{1000}{1000}"), FindError::Regex(RegexError::TooBig));
}

// A page with more matches than the limit keeps `limit` of them and says so; setting the page again
// without the signal clears it.
#[test]
fn a_page_over_the_limit_says_so() {
    let pattern = compile("a", FindOptions::default());
    let at_limit = pattern.find_page("aaa", 3);
    assert_eq!((at_limit.ranges.len(), at_limit.limit_reached), (3, false));
    let over = pattern.find_page("aaaa", 3);
    assert_eq!((over.ranges.len(), over.limit_reached), (3, true));

    let mut matches = FindMatches::new(0);
    matches.set_page(0, over);
    matches.set_page(1, at_limit);
    assert!(matches.limit_reached());
    matches.set_page(0, vec![span(0, 1)]);
    assert!(!matches.limit_reached());
}

// A pattern lists its named groups with their numbers; the lookup resolves `${name}` in a replacement.
#[test]
fn named_groups_resolve_in_replacements() {
    let options = FindOptions { regex: true, ..Default::default() };
    let pattern = compile(r"(?P<year>\d+)-(?P<month>\d+)", options);
    assert_eq!(pattern.group_names(), [(1, "year"), (2, "month")]);
    assert_eq!(pattern.group_index("month"), Some(2));
    assert_eq!(pattern.group_index("day"), None);
    assert!(compile("plain", FindOptions::default()).group_names().is_empty());

    let pieces = resolve_group_names(parse_replacement("${month}/${year}", &options), &|name| pattern.group_index(name));
    let text = "2026-11";
    let groups = pattern.groups(text, 0..7);
    let group = |n: usize| groups.get(n).cloned().flatten().map(|range| text[range].to_owned());
    assert_eq!(expand_replacement(&pieces, text, &group, ""), "11/2026");
}
