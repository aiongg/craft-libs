//! The About dialog's layout rules.

use craft_ui::contract::{Bind, Role};
use craft_ui::patterns::form_dialogs::{AboutDialog, AboutInfo, AboutTab, Contributor, Credits, ModelCredit};
use craft_ui::testing::{Fixture, relations, trial};
use egui::Vec2;

/// Whichever tab shows, the table that fills the dialog leaves the OK button inside the screen and below the table.
#[trial]
fn the_footer_stays_visible_under_a_table_that_fills_the_dialog() {
    let info = AboutInfo::new("PhotoCraft", "An image editor.", "0.1");
    let people: Vec<Contributor> = (0..60).map(|at| Contributor::new(format!("person{at}")).counts(1, 10 + at)).collect();
    let models: Vec<ModelCredit> = (0..30).map(|at| ModelCredit::new("Acme", format!("Model {at}"), "1").work(5, 100, 10)).collect();
    let credits = Credits::new(people, models, 1_000);
    for tab in [AboutTab::Contributors, AboutTab::Models] {
        let (info, credits) = (info.clone(), credits.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut open = true;
            AboutDialog::new(&info, &credits).tab(tab).opened(Bind::bound(&mut open)).show(ui);
        });
        fixture.settle();
        let screen = fixture.ctx().content_rect();
        let table = fixture.by_role(Role::TABLE).first().copied().expect("the table showed");
        let ok = fixture.by_label("OK").expect("the footer button is shown");
        assert!(screen.contains_rect(ok.rect), "{tab:?}: OK is inside the screen: {:?} in {screen:?}", ok.rect);
        assert!(relations::above(table.rect, ok.rect), "{tab:?}: OK is below the table: {:?} under {:?}", ok.rect, table.rect);
    }
}

/// A fixture over the dialog on `tab`, in a room wide enough for every column.
fn open_on(credits: Credits, tab: AboutTab) -> Fixture {
    let info = AboutInfo::new("PhotoCraft", "An image editor.", "0.1");
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        AboutDialog::new(&info, &credits).tab(tab).opened(Bind::bound(&mut open)).show(ui);
    });
    fixture.room(Vec2::new(1600.0, 900.0));
    fixture.settle();
    fixture
}

/// The painted runs that share a line with the run `anchor`, left to right.
fn row_of(fixture: &Fixture, anchor: &str) -> Vec<String> {
    let texts = fixture.texts();
    let name = texts.iter().find(|run| run.text == anchor).unwrap_or_else(|| panic!("the row {anchor:?} is shown"));
    let mut cells: Vec<_> = texts.iter().filter(|run| run.rect.center().y > name.rect.min.y && run.rect.center().y < name.rect.max.y).collect();
    cells.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    cells.into_iter().map(|run| run.text.clone()).collect()
}

/// The names of `names` in the order the table lists them, top to bottom.
fn listed(fixture: &Fixture, names: &[&str]) -> Vec<String> {
    let texts = fixture.texts();
    let mut found: Vec<_> = names.iter().filter_map(|name| texts.iter().find(|run| run.text == *name)).collect();
    found.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    found.into_iter().map(|run| run.text.clone()).collect()
}

fn people() -> Credits {
    let person = |login: &'static str, first: &'static str| Contributor::new(login).counts(1, 10).active(first, first);
    Credits::new(
        vec![person("alice", ""), person("bob", "2023-01-05"), person("Carol", "2024-03-01"), person("dave", "2023-01-05")],
        Vec::<ModelCredit>::new(),
        100,
    )
}

/// The table starts by first commit, oldest first, with the undated last and ties in name order; the Name header sorts
/// by name without regard to case, and Reverse turns the order round.
#[trial]
fn the_contributors_table_orders_its_rows_by_the_chosen_sort() {
    const NAMES: [&str; 4] = ["@alice", "@bob", "@Carol", "@dave"];
    let mut fixture = open_on(people(), AboutTab::Contributors);
    assert_eq!(listed(&fixture, &NAMES), ["@bob", "@dave", "@Carol", "@alice"], "oldest first, the undated last, the tie in name order");

    let reverse = fixture.by_label("Reverse the order").expect("the reverse button");
    fixture.click(&reverse);
    fixture.settle();
    let turned = listed(&fixture, &NAMES);
    let at = |name: &str| turned.iter().position(|shown| shown == name).expect("listed");
    assert!(at("@Carol") < at("@bob") && at("@bob") < at("@dave"), "newest first, the tie still in name order: {turned:?}");

    let name = fixture.by_label("Name").expect("the Name header");
    fixture.click(&name);
    fixture.settle();
    assert_eq!(listed(&fixture, &NAMES), ["@alice", "@bob", "@Carol", "@dave"], "names compare without case");
}

/// A date shows in the region's date style whether the timestamp carries a time or not; none shows a dash and an
/// unreadable one its first ten characters.
#[trial]
fn the_commit_dates_show_in_the_table() {
    let dated = |login: &'static str, first: &'static str| Contributor::new(login).counts(1, 10).active(first, first);
    let credits = Credits::new(
        vec![dated("day", "2023-03-14"), dated("stamp", "2023-03-14T10:20:30Z"), dated("junk", "garbage-date-text"), dated("none", "")],
        Vec::<ModelCredit>::new(),
        100,
    );
    let fixture = open_on(credits, AboutTab::Contributors);
    let first = |login: &str| {
        let row = row_of(&fixture, &format!("@{login}"));
        row[row.len() - 2].clone()
    };
    assert_eq!(first("day"), first("stamp"), "the time of day is not shown");
    assert!(first("day").contains("2023") && first("day").contains("14") && !first("day").contains('T'), "a date: {:?}", first("day"));
    assert_eq!(first("junk"), "garbage-da");
    assert_eq!(first("none"), "—");
}

/// A model's share of all commits is a whole percent; a model with any commits never reads 0 %.
#[trial]
fn the_models_table_shows_each_models_share_of_all_commits() {
    let model = |company: &'static str, commits: u64| ModelCredit::new(company, "M", "1").work(commits, 0, 0);
    let credits = Credits::new(Vec::<Contributor>::new(), vec![model("Alpha", 500), model("Beta", 1), model("Gamma", 0), model("Delta", 250)], 1_000);
    let fixture = open_on(credits, AboutTab::Models);
    let share = |company: &str| {
        let row = row_of(&fixture, company);
        row[row.len() - 2].clone()
    };
    assert!(share("Alpha").contains("50"), "half: {:?}", share("Alpha"));
    assert!(share("Delta").contains("25"), "a quarter: {:?}", share("Delta"));
    assert!(share("Beta").starts_with('<'), "one commit in a thousand reads under one percent: {:?}", share("Beta"));
    assert!(!share("Gamma").starts_with('<') && share("Gamma").contains('0'), "no commits read 0 %: {:?}", share("Gamma"));
}
