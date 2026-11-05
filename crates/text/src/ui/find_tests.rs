//! The find bar and the Find and Replace dialog.

use craft_ui::dialog::DialogHost;
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::find::*;
use crate::find::{FindError, FindMatches, FindOptions, FindPattern, RegexError, WildcardError};

struct BarApp {
    query: String,
    options: FindOptions,
    matches: FindMatches,
    progress: Option<(usize, usize)>,
    state: FindBarState,
    steps: usize,
    closed: bool,
}

fn bar(query: &str, matches: FindMatches, progress: Option<(usize, usize)>) -> Harness<'static, BarApp> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(640.0, 120.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut BarApp| {
                let mut bar = FindBar::new("bar", &mut app.query, &mut app.options, &mut app.matches).state(&mut app.state);
                if let Some((searched, pages)) = app.progress {
                    bar = bar.progress(searched, pages);
                }
                let out = bar.show(ui);
                app.steps += usize::from(out.stepped);
                app.closed |= out.closed;
            },
            BarApp {
                query: query.to_owned(),
                options: FindOptions::default(),
                matches,
                progress,
                state: FindBarState::default(),
                steps: 0,
                closed: false,
            },
        )
}

fn matches_on(pages: &[usize]) -> FindMatches {
    let mut matches = FindMatches::new(0);
    for &page in pages {
        matches.set_page(page, (0..1).map(|i| i..i + 3).collect::<Vec<_>>());
    }
    matches
}

/// The text of the first label node that satisfies `wanted`, without bidi isolates.
fn label_text(harness: &Harness<'_, BarApp>, wanted: impl Fn(&str) -> bool) -> Option<String> {
    let all = harness.query_all_by_role(Role::Label);
    all.filter_map(|n| n.accesskit_node().value().or_else(|| n.accesskit_node().label()))
        .map(|text| text.chars().filter(|c| !('\u{2066}'..='\u{2069}').contains(c)).collect::<String>())
        .find(|text| wanted(text))
}

// Enter in the bar steps to the next match and the field keeps focus; Shift-Enter steps back; Esc closes.
#[test]
fn enter_steps_and_keeps_the_keyboard() {
    let mut harness = bar("cat", matches_on(&[0, 1, 2]), None);
    harness.run();
    let field = harness.get_by_role_and_label(Role::SearchInput, "Find text");
    field.focus();
    harness.run();
    assert_eq!(harness.state().matches.current(), Some(0));

    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().matches.current(), Some(1));
    assert_eq!(harness.state().steps, 1);
    assert!(harness.get_by_role_and_label(Role::SearchInput, "Find text").is_focused(), "the field keeps the keyboard");

    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    assert_eq!(harness.state().matches.current(), Some(0));

    // The first Esc clears the field, the second closes the bar.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().query.is_empty());
    assert!(!harness.state().closed);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
}

// With progress 3/10 the count reads "Searching… 3/10"; with no matches "No matches"; with 12 and the third current "3 of 12".
#[test]
fn counts_read_searching_none_or_c_of_n() {
    let mut harness = bar("cat", FindMatches::new(0), Some((3, 10)));
    harness.run();
    assert_eq!(label_text(&harness, |t| t.starts_with("Searching")).as_deref(), Some("Searching… 3/10"));

    harness.state_mut().progress = Some((10, 10));
    harness.run();
    assert!(label_text(&harness, |t| t == "No matches").is_some());

    let mut matches = FindMatches::new(0);
    matches.set_page(0, (0..12).map(|i| i * 4..i * 4 + 3).collect::<Vec<_>>());
    matches.pick(2);
    harness.state_mut().matches = matches;
    harness.run();
    assert!(label_text(&harness, |t| t == "3 of 12").is_some());
}

// A page that hit the match limit makes the count a lower bound: "3 of 12+".
#[test]
fn a_page_at_its_limit_marks_the_count() {
    let mut harness = bar("a", FindMatches::new(0), None);
    let pattern = FindPattern::compile("a", &FindOptions::default());
    let mut matches = FindMatches::new(0);
    if let Ok(pattern) = pattern {
        matches.set_page(0, pattern.find_page("aaaa", 3));
    }
    harness.state_mut().matches = matches;
    harness.run();
    assert!(label_text(&harness, |t| t == "1 of 3+").is_some());
}

// The field takes the keyboard on the pass the app opens the bar, and again when the app asks for
// it with the bar open; the bar never claims it unasked.
#[test]
fn the_app_decides_when_the_field_takes_the_keyboard() {
    let focused = |harness: &Harness<'_, BarApp>| harness.get_by_role_and_label(Role::SearchInput, "Find text").is_focused();
    let mut harness = bar("cat", matches_on(&[0]), None);
    harness.run();
    assert!(!focused(&harness));

    harness.state_mut().state.open();
    harness.run();
    assert!(focused(&harness));
    assert!(!harness.state().state.is_opening());

    harness.get_by_label("Find options").focus();
    harness.run();
    assert!(!focused(&harness));
    harness.state_mut().state.open();
    harness.run();
    assert!(focused(&harness));
}

// The refusal of a pattern is worded by FindLabels, in the labels' own text and numbers.
#[test]
fn refusals_read_in_the_labels_words() {
    let labels = FindLabels::english();
    let numbers = craft_ui::localized::LocaleNumbers::from_tag("en");
    let wild = |kind| labels.error(&FindError::Wildcards(kind), &numbers);
    assert_eq!(wild(WildcardError::UnclosedBracket), "A [ needs a closing ]");
    assert_eq!(wild(WildcardError::BadCount("x".into())), "{x} is not a valid count");
    assert_eq!(wild(WildcardError::CountTooLarge), "Counts go up to 1,000");
    assert_eq!(wild(WildcardError::ReversedRange { from: 'z', to: 'a' }), "[z-a] is not in ascending order");
    assert_eq!(labels.error(&FindError::Regex(RegexError::UnclosedGroup), &numbers), "A ( needs a closing )");
    assert_eq!(labels.error(&FindError::TooLong, &numbers), "The Find what text is longer than 1,024 characters");

    let own = FindLabels { regex_unclosed_group: "offen".into(), ..FindLabels::english() };
    assert_eq!(own.error(&FindError::Regex(RegexError::UnclosedGroup), &numbers), "offen");
}

// The dialog's Replace… button and the Special menu's rows are the labels' own words, not joins.
#[test]
fn replace_more_and_special_rows_come_from_the_labels() {
    let labels = FindLabels { replace_more: "Ersetzen mit".into(), special_item: "{code} = {name}".into(), ..FindLabels::english() };
    let mut harness = dialog(FindReplaceDialog::new("a", FindTab::Find).labels(labels.clone()));
    harness.run();
    harness.get_by_label("Ersetzen mit");
    drop(harness);

    let mut harness = dialog(FindReplaceDialog::new("a", FindTab::Find).word(true).labels(labels));
    harness.run();
    harness.get_by_label("Search Options").click();
    harness.run();
    harness.get_by_label("Special").click();
    harness.run();
    harness.get_by_label_contains("^p = Paragraph Mark");
}

struct DialogApp {
    host: DialogHost,
    requests: Vec<FindRequest>,
}

fn dialog(dialog: FindReplaceDialog) -> Harness<'static, DialogApp> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(720.0, 640.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut DialogApp| {
                app.host.show(ui.ctx());
                if let Some(dialog) = app.host.get_mut::<FindReplaceDialog>()
                    && let Some(request) = dialog.take_request()
                {
                    app.requests.push(request);
                }
            },
            DialogApp { host, requests: Vec::new() },
        )
}

// In Word's dialog Use wildcards disables Match case and Whole words; Special inserts its code into the field that last had focus.
#[test]
fn wildcards_grey_out_case_and_words() {
    let mut harness = dialog(FindReplaceDialog::new("a", FindTab::Find).word(true));
    harness.run();
    harness.get_by_label("Search Options").click();
    harness.run();
    assert!(!harness.get_by_label("Match case").accesskit_node().is_disabled());
    harness.get_by_label("Use wildcards").click();
    harness.run();
    assert!(harness.get_by_label("Match case").accesskit_node().is_disabled());
    assert!(harness.get_by_label("Whole words").accesskit_node().is_disabled());
    assert!(!harness.get_by_label("Use wildcards").accesskit_node().is_disabled());

    harness.get_by_label("Special").click();
    harness.run();
    harness.get_by_label_contains("Paragraph Mark").click();
    harness.run();
    harness.get_by_label("Find Next").click();
    harness.run();
    let wildcards = FindOptions { wildcards: true, ..Default::default() };
    assert_eq!(harness.state().requests, [FindRequest::FindNext { query: "a^p".into(), options: wildcards }]);
}

// Replace All returns one ReplaceAll request with the query, the replacement and the options.
#[test]
fn replace_all_requests_once() {
    let options = FindOptions { match_case: true, ..Default::default() };
    let mut harness = dialog(FindReplaceDialog::new("cat", FindTab::Replace).options(options));
    harness.run();
    let with = harness.get_by_role_and_label(Role::TextInput, "Replace with:");
    with.focus();
    harness.run();
    harness.get_by_role_and_label(Role::TextInput, "Replace with:").type_text("dog");
    harness.run();
    harness.get_by_label("Replace All").click();
    harness.run();
    assert_eq!(harness.state().requests, [FindRequest::ReplaceAll { query: "cat".into(), with: "dog".into(), options }]);
}
