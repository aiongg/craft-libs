//! The About dialog's own rules: the order of the contributors, the names each mode shows, the share of a model and
//! the written date.

use craft_i18n::DateFormat;
use craft_ui::patterns::form_dialogs::{CommitShare, Contributor, ContributorSort, NameMode, commit_day, commit_share, sorted_contributors};
use craft_ui::testing::trial;

fn person(login: &'static str, prs: u64, commits: u64, first: &'static str) -> Contributor {
    Contributor::new(login).counts(prs, commits).lines(1_200, 200).active(first, first)
}

fn logins(list: Vec<&Contributor>) -> Vec<String> {
    list.into_iter().map(|c| c.login.to_string()).collect()
}

/// Contributors sort by first commit oldest first by default, counts biggest first, names without case or `@`; someone
/// credited only through a pull request (no date) sorts last; ties fall back to the name, then the login, in either
/// direction.
#[trial]
fn contributors_sort_by_the_rules_of_the_table() {
    let list = vec![
        person("alice", 1, 12, "2025-01-02T00:00:00Z"),
        person("bob", 3, 40, "2025-03-01T00:00:00Z").display_name("Zed"),
        person("Carol", 2, 0, ""),
    ];
    let sort = |mode, key: ContributorSort, ascending| logins(sorted_contributors(&list, mode, key, ascending));
    let key = ContributorSort::default();
    assert_eq!(sort(NameMode::Username, key, key.default_ascending()), ["alice", "bob", "Carol"], "oldest first, the undated last");
    assert_eq!(sort(NameMode::Username, key, false), ["Carol", "bob", "alice"], "reversed, the undated come first");
    let commits = ContributorSort::Commits;
    assert_eq!(sort(NameMode::Username, commits, commits.default_ascending()), ["bob", "alice", "Carol"], "biggest first");
    assert_eq!(sort(NameMode::Username, ContributorSort::Name, true), ["alice", "bob", "Carol"], "case and @ ignored");
    assert_eq!(sort(NameMode::DisplayName, ContributorSort::Name, true), ["alice", "Carol", "bob"], "Zed sorts after @alice and @Carol");

    let twins = vec![person("b", 1, 5, ""), person("a", 1, 5, ""), person("c", 1, 5, "")];
    for ascending in [true, false] {
        assert_eq!(
            logins(sorted_contributors(&twins, NameMode::Username, ContributorSort::Commits, ascending)),
            ["a", "b", "c"],
            "ties fall back to the name whichever way the figures go"
        );
    }
    assert_eq!(list[1].name(NameMode::RealName), "@bob", "no real name shows the login");
    assert_eq!(list[1].name(NameMode::DisplayName), "Zed");
}

/// A model with any commits never reads 0 %.
#[trial]
fn a_credited_model_never_reads_zero_percent() {
    assert_eq!(commit_share(1, 1_000), CommitShare::UnderOne);
    assert_eq!(commit_share(0, 1_000), CommitShare::Ratio(0.0));
    assert_eq!(commit_share(5, 1_000), CommitShare::Ratio(0.005));
    assert_eq!(commit_share(10, 0), CommitShare::Ratio(1.0), "no total reads as the whole");
}

/// Dates are written in the region's date style; a timestamp that does not parse shows its first ten characters.
#[trial]
fn commit_dates_follow_the_region() {
    let us = DateFormat::from_tag("en-US");
    let de = DateFormat::from_tag("de-DE");
    assert_eq!(commit_day(&us, "2025-01-02T10:00:00Z"), "Jan 2, 2025");
    assert_eq!(commit_day(&de, "2025-01-02T10:00:00Z"), "02.01.2025");
    assert_eq!(commit_day(&us, ""), "—", "no date");
    assert_eq!(commit_day(&us, "2025-13-45 and more"), "2025-13-45", "not a date: the ISO slice");
}
