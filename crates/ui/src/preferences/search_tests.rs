//! The search rule: which pages are listed, which rows show and which page is shown.

use super::search::{list_pages, row_shown, shown_position};
use super::stories::FakeHost;
use super::{PreferencesHost, matches};

fn listed_ids(host: &FakeHost, query: &str) -> Vec<String> {
    let pages = host.pages();
    list_pages(&pages, query).iter().map(|l| pages[l.index].id.to_owned()).collect()
}

#[test]
fn the_rule_is_a_trimmed_case_insensitive_substring_of_label_keywords_or_key() {
    assert!(matches("  LANG ", "Language:", "", "general.language"));
    assert!(matches("locale", "Language:", "locale", "x"));
    assert!(matches("general.l", "Language:", "", "general.language"));
    assert!(!matches("zzz", "Language:", "locale", "general.language"));
    assert!(matches("", "anything", "", ""));
}

#[test]
fn a_title_match_lists_the_page_with_every_row_and_a_row_match_only_that_row() {
    let host = FakeHost::new();
    let pages = host.pages();
    let listed = list_pages(&pages, "tools");
    assert_eq!(listed.len(), 1);
    assert!(pages[listed[0].index].groups[0].rows.iter().filter(|r| row_shown("tools", &listed[0], r)).count() == 2);
    let listed = list_pages(&pages, "smooth");
    assert_eq!(listed.len(), 1);
    let rows: Vec<_> = pages[listed[0].index].groups[0].rows.iter().filter(|r| row_shown("smooth", &listed[0], r)).map(|r| r.key).collect();
    assert_eq!(rows, ["tools.smoothing"]);
    assert_eq!(listed_ids(&host, "locale"), ["general"]);
}

#[test]
fn empty_pages_are_dimmed_and_drop_out_of_a_search_unless_the_title_matches() {
    let host = FakeHost::new();
    let pages = host.pages();
    assert!(list_pages(&pages, "").iter().any(|l| l.dimmed && pages[l.index].id == "plugins"));
    assert!(!listed_ids(&host, "show").contains(&"plugins".to_owned()));
    assert!(listed_ids(&host, "plug").contains(&"plugins".to_owned()));
}

#[test]
fn the_selected_page_falls_back_to_the_first_listed_and_returns() {
    let host = FakeHost::new();
    let pages = host.pages();
    let hidden = list_pages(&pages, "smooth");
    assert_eq!(shown_position(&hidden, &pages, "general"), Some(0));
    assert_eq!(pages[hidden[0].index].id, "tools");
    let all = list_pages(&pages, "");
    assert_eq!(shown_position(&all, &pages, "tools"), Some(2));
    assert_eq!(shown_position(&list_pages(&pages, "nothing matches"), &pages, "tools"), None);
}
