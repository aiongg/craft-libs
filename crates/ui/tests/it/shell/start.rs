//! The start screen: the order of the recent files and the actions New and Open report.

use craft_i18n::Collator;
use craft_ui::contract::Bind;
use craft_ui::runtime::access::Role;
use craft_ui::runtime::commands::{CommandState, EditCommands};
use craft_ui::runtime::frame::take_output;
use craft_ui::shell::{RecentFile, RecentSort, RecentView, StartAction, StartScreen, Template, sort_recent};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::path::PathBuf;
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
fn new_and_open_report_their_actions() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let (mut view, mut sort) = (RecentView::default(), RecentSort::default());
        let out = StartScreen::new(Bind::bound(&mut view), Bind::bound(&mut sort), &[]).show(ui);
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

/// Delete on a focused recent file runs the start screen's own Remove, so the application's Edit > Delete on the same
/// key does not fire.
#[trial]
fn delete_on_a_recent_file_removes_it_and_leaves_the_edit_command() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let files = [file("first.fig", Some(20), None), file("second.fig", Some(10), None)];
    let mut fixture = Fixture::new(move |ui| {
        let (mut view, mut sort) = (RecentView::List, RecentSort::default());
        let out = StartScreen::new(Bind::bound(&mut view), Bind::bound(&mut sort), &files).show(ui);
        sink.borrow_mut().extend(out.actions);
    });
    fixture.command_state(EditCommands::Delete, CommandState::new());
    fixture.settle();
    // A list row with a detail is named by its label and the detail, here the folder.
    let row = fixture.by_label("first.fig, /work").unwrap_or_else(|| {
        let tree = fixture.tree();
        let names: Vec<_> = tree.keys().into_iter().map(|key| (tree.role_of(key), tree.name_of(key).map(str::to_owned))).collect();
        panic!("the recent file is listed among {names:?}")
    });
    fixture.click(&row);
    fixture.idle(2);
    seen.borrow_mut().clear();
    let _ = take_output(fixture.ctx());
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    let fired = take_output(fixture.ctx()).fired;
    assert_eq!(
        *seen.borrow(),
        [StartAction::RemoveRecent("/work/first.fig".into())],
        "focus {:?}, app commands fired {}",
        fixture.focused(),
        fired.len()
    );
    assert!(fired.is_empty(), "the application's Delete command did not fire");
}

/// A recent file's card shows a remove button while the pointer is over it, and its press reports the same remove
/// the menu fires.
#[trial]
fn the_remove_button_of_a_recent_card_shows_on_hover_and_removes_the_file() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let files = [file("first.fig", Some(20), None), file("second.fig", Some(10), None)];
    let mut fixture = Fixture::new(move |ui| {
        let (mut view, mut sort) = (RecentView::Grid, RecentSort::default());
        let out = StartScreen::new(Bind::bound(&mut view), Bind::bound(&mut sort), &files).show(ui);
        sink.borrow_mut().extend(out.actions);
    });
    fixture.settle();
    let remove_button = |fixture: &Fixture| {
        let tree = fixture.tree();
        fixture.by_role(Role::BUTTON).into_iter().find(|target| tree.name_of(target.node).is_some_and(|name| name.starts_with("Remove from Recent")))
    };
    assert!(remove_button(&fixture).is_none(), "no remove button before the pointer is over a card");
    let card = fixture.by_label("first.fig").expect("the card shows");
    fixture.frame(vec![pointer::moved(card.center())]);
    fixture.idle(2);
    let Some(remove) = remove_button(&fixture) else {
        let tree = fixture.tree();
        let names: Vec<_> = tree.keys().into_iter().map(|key| (tree.role_of(key), tree.name_of(key).map(str::to_owned))).collect();
        panic!("the pointer over the card shows its remove button among {names:?}");
    };
    fixture.click(&remove);
    fixture.idle(2);
    assert_eq!(*seen.borrow(), [StartAction::RemoveRecent("/work/first.fig".into())]);
}

const SCREEN: Vec2 = Vec2::new(600.0, 400.0);

fn heading_top(fixture: &Fixture, text: &str) -> f32 {
    fixture.texts().iter().find(|painted| painted.text == text).map_or_else(|| panic!("the page shows {text:?}"), |painted| painted.rect.min.y)
}

/// Many recent files and templates: the three sections follow each other down one page, and the wheel scrolls that page
/// in both views of the recent files.
#[trial]
fn many_files_and_templates_share_one_scrolling_page() {
    for show in [RecentView::Grid, RecentView::List] {
        let files: Vec<RecentFile> = (0..200).map(|at| file(&format!("file {at:02}"), Some(at), None)).collect();
        let pinned = vec![PathBuf::from("/work/a"), PathBuf::from("/work/b")];
        let templates: Vec<Template> = (0..100).map(|at| Template::new(format!("t{at}"), format!("Template {at}"), "")).collect();
        let mut fixture = Fixture::new(move |ui| {
            let (mut view, mut sort) = (show, RecentSort::default());
            let _ = StartScreen::new(Bind::bound(&mut view), Bind::bound(&mut sort), &files)
                .pinned(&pinned)
                .templates(&templates)
                .max_recent(200)
                .show(ui);
        });
        fixture.room(SCREEN);
        fixture.settle();
        let (recent, pinned, templates) =
            (heading_top(&fixture, "Recent"), heading_top(&fixture, "Pinned Folders"), heading_top(&fixture, "Templates"));
        assert!(recent < pinned && pinned < templates, "{show:?}: the sections follow each other: {recent} {pinned} {templates}");
        assert!(templates > SCREEN.y, "{show:?}: the templates start below the screen: {templates}");
        fixture.frame(vec![pointer::moved(Pos2::new(300.0, 200.0))]);
        fixture.frame(vec![pointer::wheel(-1000.0)]);
        fixture.idle(2);
        let after = heading_top(&fixture, "Templates");
        assert!(after < templates, "{show:?}: the wheel scrolled the page: {templates} -> {after}");
    }
}

#[trial]
fn recent_files_sort_by_date_and_name() {
    {
        let files = [file("old", Some(10), None), file("none", None, None), file("new", Some(30), None), file("mid", Some(20), None)];
        assert_eq!(names(&files, RecentSort::Opened), ["new", "mid", "old", "none"]);
    }
    {
        let files = [file("none", None, None), file("old", None, Some(10)), file("new", None, Some(30))];
        assert_eq!(names(&files, RecentSort::Created), ["new", "old", "none"]);
    }
    {
        let files = [file("page 10", None, None), file("page 2", None, None), file("page 1", None, None)];
        assert_eq!(names(&files, RecentSort::Name), ["page 1", "page 2", "page 10"]);
    }
}
