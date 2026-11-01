//! `AboutDialog`.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::about::{ModelColumn, share_total};
use super::*;
use crate::dialog::DialogHost;
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

fn person(login: &'static str, prs: u64, commits: u64, first: &'static str) -> Contributor {
    Contributor {
        login: login.into(),
        display_name: None,
        real_name: None,
        prs,
        commits,
        lines_added: 1_200,
        lines_deleted: 200,
        binary_added: 0,
        binary_deleted: 0,
        first_commit: first.into(),
        last_commit: first.into(),
    }
}

fn credits() -> Credits {
    let mut bob = person("bob", 3, 40, "2025-03-01T00:00:00Z");
    bob.display_name = Some("Zed".into());
    Credits {
        contributors: vec![person("alice", 1, 12, "2025-01-02T00:00:00Z"), bob, person("Carol", 2, 0, "")].into(),
        models: vec![
            ModelCredit { company: "Acme".into(), model: "Big".into(), version: "1".into(), commits: 60, lines_added: 5_000, lines_deleted: 100 },
            ModelCredit { company: "Acme".into(), model: "Small".into(), version: "2".into(), commits: 1, lines_added: 3, lines_deleted: 0 },
        ]
        .into(),
        total_commits: 1_000,
    }
}

fn logins(list: Vec<&Contributor>) -> Vec<String> {
    list.into_iter().map(|c| c.login.to_string()).collect()
}

// Contributors sort by first commit oldest first by default, counts biggest first, names without
// case or @; PR-only contributors (no date) sort last; ties fall back to the name then the login.
#[test]
fn contributors_sort_as_photocraft() {
    let credits = credits();
    let list = &credits.contributors;
    let sort = |mode, key: ContributorSort, ascending| logins(sorted_contributors(list, mode, key, ascending));
    let key = ContributorSort::default();
    assert_eq!(sort(NameMode::Username, key, key.default_ascending()), ["alice", "bob", "Carol"], "oldest first, the undated last");
    assert_eq!(sort(NameMode::Username, key, false), ["Carol", "bob", "alice"], "reversed, the undated come first");
    let commits = ContributorSort::Commits;
    assert_eq!(sort(NameMode::Username, commits, commits.default_ascending()), ["bob", "alice", "Carol"], "biggest first");
    assert_eq!(sort(NameMode::Username, ContributorSort::Name, true), ["alice", "bob", "Carol"], "case and @ ignored");
    assert_eq!(sort(NameMode::DisplayName, ContributorSort::Name, true), ["alice", "Carol", "bob"], "Zed sorts after @alice and @Carol");

    let mut twins = vec![person("b", 1, 5, ""), person("a", 1, 5, "")];
    twins.push(person("c", 1, 5, ""));
    assert_eq!(
        logins(sorted_contributors(&twins, NameMode::Username, ContributorSort::Commits, false)),
        ["a", "b", "c"],
        "ties fall back to the name"
    );
    assert_eq!(credits.contributors[1].name(NameMode::RealName), "@bob", "no real name shows the login");
    assert_eq!(credits.contributors[1].name(NameMode::DisplayName), "Zed");
}

// A model with any commits never reads 0 %; the share is of total_commits or the busiest model's.
#[test]
fn model_share_is_never_zero_for_a_credited_model() {
    assert_eq!(commit_share(1, 1_000), CommitShare::UnderOne);
    assert_eq!(commit_share(0, 1_000), CommitShare::Ratio(0.0));
    assert_eq!(commit_share(5, 1_000), CommitShare::Ratio(0.005));
    assert_eq!(commit_share(10, 0), CommitShare::Ratio(1.0), "no total reads as the whole");
    let numbers = LocaleNumbers::from_tag("en-US");
    let models = credits().models;
    assert_eq!(ModelColumn::Share.cell(&models[1], 1_000, &numbers), "<1%");
    assert_eq!(ModelColumn::Share.cell(&models[0], 1_000, &numbers), "6%");
    let trailing = Credits { total_commits: 10, ..credits() };
    assert_eq!(share_total(&trailing), 60, "a total below the busiest model's is raised to it");
}

struct App {
    host: DialogHost,
}

fn harness(tab: AboutTab) -> Harness<'static, App> {
    let info = AboutInfo {
        app_name: "PhotoCraft".into(),
        blurb: "An open-source image editor.".into(),
        version: "0.42.1".into(),
        community: Some(AboutLink { label: "Join us on Discord".into(), url: "https://discord.example/join".into() }),
        links: vec![AboutLink { label: "Website".into(), url: "https://example.org".into() }],
        built_with: "egui · wgpu".into(),
    };
    let mut host = DialogHost::default();
    host.open(AboutDialog::new(info, credits()).tab(tab));
    HarnessBuilder::default().with_size(egui::vec2(1000.0, 800.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            app.host.show(ui.ctx());
        },
        App { host },
    )
}

fn taken(harness: &mut Harness<'static, App>) -> Option<String> {
    harness.state_mut().host.get_mut::<AboutDialog>().and_then(AboutDialog::take_link)
}

/// The close box's right edge: the window's right edge moves when its width does.
fn dialog_width(harness: &Harness<'static, App>) -> f32 {
    harness.get_by_label("Close").rect().right()
}

// The tabs switch with arrows; a contributor link click is handed out by take_link with
// github.com/<login>, not opened; the window keeps its width across tabs; a contributor's tooltip
// says "1 PR, 12 commits" with the counted words.
#[test]
fn about_tabs_and_links() {
    let mut harness = harness(AboutTab::About);
    harness.run();
    harness.get_by_role_and_label(Role::Dialog, "About PhotoCraft");
    harness.get_by_label_contains("Version 0.42.1");
    let width = dialog_width(&harness);

    harness.get_by_role_and_label(Role::Link, "Website").click();
    harness.run();
    assert_eq!(taken(&mut harness).as_deref(), Some("https://example.org"));
    harness.get_by_label("Join us on Discord").click();
    harness.run();
    assert_eq!(taken(&mut harness).as_deref(), Some("https://discord.example/join"));

    harness.get_by_role_and_label(Role::Tab, "About").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press_modifiers(Modifiers::NONE, Key::Enter);
    harness.run();
    assert_eq!(harness.state_mut().host.get_mut::<AboutDialog>().map(|d| d.current_tab()), Some(AboutTab::Contributors));
    assert_eq!(dialog_width(&harness), width, "the window keeps its width on every tab");
    harness.get_by_label_contains("Contributors: 3 · Commits: 1,000");

    harness.get_by_role_and_label(Role::Link, "@alice").click();
    harness.run();
    assert_eq!(taken(&mut harness).as_deref(), Some("https://github.com/alice"));
    let summary = credits().contributors[0].summary(&harness.ctx);
    assert!(summary.starts_with("@alice: 1 PR, 12 commits, +1,200 / −200 lines (Δ +1,000)"), "{summary}");

    harness.get_by_label("Table").click();
    harness.run();
    assert_eq!(dialog_width(&harness), width);
    harness.get_by_role_and_label(Role::ColumnHeader, "Commits").click();
    harness.run();
    let y = |harness: &Harness<'static, App>, name: &str| harness.get_by_role_and_label(Role::Link, name).rect().top();
    assert!(y(&harness, "@bob") < y(&harness, "@alice"), "a header click sorts counts biggest first");
    harness.get_by_role_and_label(Role::ColumnHeader, "Commits, sorted descending").click();
    harness.run();
    assert!(y(&harness, "@alice") < y(&harness, "@bob"), "a second click reverses the order");

    harness.get_by_role_and_label(Role::Tab, "Models").click();
    harness.run();
    assert_eq!(dialog_width(&harness), width);
    harness.get_by_label("<1%");
}
