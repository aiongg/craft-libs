//! The start screen.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::theme::{Base, install};

type Actions = Rc<RefCell<Vec<StartAction>>>;

fn file(name: &str, opened: Option<i64>, created: Option<i64>) -> RecentFile {
    RecentFile { path: PathBuf::from("/work").join(name), name: name.to_owned(), opened, created, thumbnail: None }
}

fn harness(recent: Vec<RecentFile>, view: RecentView, actions: &Actions) -> Harness<'static, ()> {
    let actions = Rc::clone(actions);
    let builder: HarnessBuilder<()> = Harness::builder().with_size([900.0, 700.0]).with_step_dt(0.05).with_context_setup(|ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, Direction::Ltr);
    });
    let mut harness = builder.build_ui(move |ui| {
        let shown = StartScreen::new("home").recent(&recent).view(view, RecentSort::Opened).now(1_000_000).show(ui);
        actions.borrow_mut().extend(shown);
    });
    harness.run();
    harness
}

fn card<'h>(harness: &'h Harness<'_, ()>, name: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(name))
}

fn two_files() -> Vec<RecentFile> {
    vec![file("a.psd", Some(900_000), None), file("b.psd", Some(100_000), None)]
}

#[test]
fn start_screen_reports_new_open_and_recent() {
    let actions = Actions::default();
    let mut harness = harness(two_files(), RecentView::Grid, &actions);
    harness.get_by_label("New…").click();
    harness.run();
    harness.get_by_label("Open…").click();
    harness.run();
    card(&harness, "b.psd").click();
    harness.run();
    assert_eq!(*actions.borrow(), [StartAction::New, StartAction::Open, StartAction::OpenRecent(PathBuf::from("/work/b.psd"))]);
    // The card is named by the file and described by its folder and age.
    let node = card(&harness, "a.psd");
    assert_eq!(node.accesskit_node().description().as_deref(), Some("/work, 1 day ago"));

    // Arrow keys move between the cards and Enter opens the focused one.
    let focused = Actions::default();
    let mut harness = self::harness(two_files(), RecentView::Grid, &focused);
    card(&harness, "a.psd").focus();
    harness.run();
    key(&mut harness, Key::ArrowRight, Modifiers::NONE);
    key(&mut harness, Key::Enter, Modifiers::NONE);
    assert_eq!(*focused.borrow(), [StartAction::OpenRecent(PathBuf::from("/work/b.psd"))]);

    // The list view opens a row on Enter and names it by the file.
    let listed = Actions::default();
    let mut harness = self::harness(two_files(), RecentView::List, &listed);
    harness.get_by(|n| n.role() == Role::ListBox).focus();
    harness.run();
    key(&mut harness, Key::Enter, Modifiers::NONE);
    assert_eq!(listed.borrow().len(), 1, "Enter opens the lead row: {:?}", listed.borrow());
    assert!(harness.query_by_label_contains("a.psd").is_some());

    let empty = Actions::default();
    let harness = self::harness(Vec::new(), RecentView::Grid, &empty);
    assert!(harness.query_by_label("Files you open show here.").is_some() || harness.query_by_value("Files you open show here.").is_some());
    assert!(harness.query_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("a.psd")).is_none());
}

fn key(harness: &mut Harness<'_, ()>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        harness.event(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
    }
    harness.run_steps(3);
}

fn menu_item<'h>(harness: &'h Harness<'_, ()>, label: &'h str) -> Option<egui_kittest::Node<'h>> {
    harness.query_by(|n| n.role() == Role::MenuItem && n.label().as_deref() == Some(label))
}

#[test]
fn recent_card_menu_removes_and_clears() {
    let actions = Actions::default();
    let mut harness = harness(two_files(), RecentView::Grid, &actions);
    // Shift+F10 on the focused card.
    card(&harness, "a.psd").focus();
    harness.run();
    key(&mut harness, Key::F10, Modifiers::SHIFT);
    for label in ["Remove from Recent", "Clear Recent Files", "Show in Folder"] {
        assert!(menu_item(&harness, label).is_some(), "{label} is in the menu");
    }
    menu_item(&harness, "Show in Folder").expect("item").click();
    harness.run_steps(3);
    assert_eq!(*actions.borrow(), [StartAction::RevealRecent(PathBuf::from("/work/a.psd"))]);

    // A secondary click on the other card.
    let center = card(&harness, "b.psd").rect().center();
    harness.event(Event::PointerMoved(center));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: center, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        harness.run();
    }
    harness.run_steps(3);
    menu_item(&harness, "Clear Recent Files").expect("the menu opens at the pointer").click();
    harness.run_steps(3);
    assert_eq!(actions.borrow().last(), Some(&StartAction::ClearRecent));

    // Delete on a focused card.
    card(&harness, "b.psd").focus();
    harness.run();
    key(&mut harness, Key::Delete, Modifiers::NONE);
    assert_eq!(actions.borrow().last(), Some(&StartAction::RemoveRecent(PathBuf::from("/work/b.psd"))));
}

#[test]
fn recent_files_sort_by_opened_name_and_created() {
    let mut files =
        vec![file("banana", Some(10), Some(3)), file("Cherry", None, None), file("apple", Some(30), Some(1)), file("Date", Some(20), Some(2))];
    let names = |files: &[RecentFile]| files.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
    sort_recent(&mut files, RecentSort::Opened);
    assert_eq!(names(&files), ["apple", "Date", "banana", "Cherry"]);
    sort_recent(&mut files, RecentSort::Name);
    assert_eq!(names(&files), ["apple", "banana", "Cherry", "Date"]);
    sort_recent(&mut files, RecentSort::Created);
    assert_eq!(names(&files), ["banana", "Date", "apple", "Cherry"]);
}
