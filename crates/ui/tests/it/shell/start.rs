//! The start screen: the order of the recent files and the actions New and Open report.

use craft_i18n::Collator;
use craft_ui::contract::Bind;
use craft_ui::shell::{RecentFile, RecentSort, RecentView, StartAction, StartScreen, sort_recent};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn file(name: &str, opened: Option<i64>, created: Option<i64>) -> RecentFile {
    let mut file = RecentFile::new(format!("/work/{name}"), name);
    file.opened = opened;
    file.created = created;
    file
}

fn names(files: &[RecentFile], sort: RecentSort) -> Vec<String> {
    let mut sorted = files.to_vec();
    sort_recent(&mut sorted, sort, &Collator::new("en"));
    sorted.into_iter().map(|file| file.name).collect()
}

#[trial]
fn opened_puts_the_newest_first_and_undated_files_last() {
    let files = [file("old", Some(10), None), file("none", None, None), file("new", Some(30), None), file("mid", Some(20), None)];
    assert_eq!(names(&files, RecentSort::Opened), ["new", "mid", "old", "none"]);
}

#[trial]
fn created_puts_the_newest_first_and_undated_files_last() {
    let files = [file("none", None, None), file("old", None, Some(10)), file("new", None, Some(30))];
    assert_eq!(names(&files, RecentSort::Created), ["new", "old", "none"]);
}

#[trial]
fn name_reads_runs_of_digits_by_value() {
    let files = [file("page 10", None, None), file("page 2", None, None), file("page 1", None, None)];
    assert_eq!(names(&files, RecentSort::Name), ["page 1", "page 2", "page 10"]);
}

#[trial]
fn new_and_open_report_their_actions() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let collator = Collator::new("en");
        let (mut view, mut sort) = (RecentView::default(), RecentSort::default());
        let out = StartScreen::new(Bind::bound(&mut view), Bind::bound(&mut sort), &[], &collator).show(ui);
        sink.borrow_mut().extend(out.actions);
    });
    fixture.settle();
    for label in ["New…", "Open…"] {
        let button = fixture.by_label(label).expect("the button shows");
        fixture.click(&button);
        fixture.idle(2);
    }
    assert_eq!(*seen.borrow(), [StartAction::New, StartAction::Open]);
}
