//! The menu bar: which titles show in a room, how a title's menu opens, runs a row and hands over to its neighbour,
//! how F10 takes focus and returns it, what a title reports to assistive technology, and what the help search finds.

use craft_i18n::{Direction, Folder};
use craft_ui::buttons::Button;
use craft_ui::contract::{PopupKind, Show as _};
use craft_ui::menu::MenuItem;
use craft_ui::menu::bar::{MenuBar, MenuHelp, MenuTitle, TitleFit, fit_titles};
use craft_ui::menu::search::{HelpEntry, search_order};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const TITLES: [f32; 3] = [40.0, 50.0, 60.0];
const MORE: f32 = 20.0;

#[trial]
fn titles_that_do_not_fit_go_into_an_overflow_menu() {
    let fit = |room| {
        let TitleFit { shown, overflow, .. } = fit_titles(&TITLES, MORE, room);
        (shown, overflow)
    };
    assert_eq!(fit(150.0), (3, false));
    assert_eq!(fit(120.0), (2, true));
    assert_eq!(fit(50.0), (0, true), "none fit beside the overflow title");
}

/// A button, then a menu bar with File, Edit and Help; the rows' runs and the ids of the button and the bar are
/// recorded.
struct Rig {
    fixture: Fixture,
    ran: Rc<RefCell<Vec<&'static str>>>,
    button: Rc<Cell<Option<Id>>>,
    bar: Rc<Cell<Option<Id>>>,
}

fn entries() -> Vec<HelpEntry> {
    // The help rows show their command's label, so each entry has its own command.
    vec![
        HelpEntry::new(EditCommands::Paste, "Café").with_path(["Edit"]),
        HelpEntry::new(EditCommands::Copy, "Copy").with_english("Duplicate").with_path(["Edit"]),
        HelpEntry::new(EditCommands::Cut, "Cut").with_path(["Edit", "Clipboard"]),
    ]
}

fn rig(direction: Direction) -> Rig {
    let ran = Rc::new(RefCell::new(Vec::new()));
    let (button, bar) = (Rc::new(Cell::new(None)), Rc::new(Cell::new(None)));
    let shown = (ran.clone(), button.clone(), bar.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (ran, button, bar) = &shown;
        button.set(Some(Button::new("Apply").show(ui).id));
        let log = ran.clone();
        let titles = vec![
            MenuTitle::new("&File", move |ui| {
                MenuItem::new("&New").show(ui);
                if MenuItem::new("&Save").show(ui).ran {
                    log.borrow_mut().push("Save");
                }
            }),
            MenuTitle::new("&Edit", |ui| {
                MenuItem::new("Cu&t").show(ui);
            }),
            MenuTitle::new("&Help", |ui| {
                MenuItem::new("&About").show(ui);
            }),
        ];
        let help = entries();
        bar.set(Some(MenuBar::new("Main menu", titles).help(MenuHelp::new(2, &help)).show(ui).response.id));
    });
    fixture.direction(direction).settle();
    Rig { fixture, ran, button, bar }
}

impl Rig {
    fn open(&self, label: &str) -> bool {
        self.fixture.by_label(label).is_some()
    }

    fn key(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
        self.fixture.settle();
    }
}

#[trial]
fn a_press_on_a_title_opens_its_menu_and_a_release_on_a_row_runs_it() {
    let mut rig = rig(Direction::Ltr);
    let file = rig.fixture.by_label("File").expect("the File title");
    rig.fixture.press(&file);
    rig.fixture.idle(3);
    assert!(rig.open("Save"), "the menu is open while the press is still held");
    let save = rig.fixture.by_label("Save").expect("the Save row");
    rig.fixture.release(&save);
    rig.fixture.settle();
    assert_eq!(*rig.ran.borrow(), ["Save"]);
    assert!(!rig.open("Save"), "running a row closes the menu");
}

#[trial]
fn left_and_right_move_between_the_open_menus_in_reading_order() {
    for (direction, key) in [(Direction::Ltr, Key::ArrowRight), (Direction::Rtl, Key::ArrowLeft)] {
        let mut rig = rig(direction);
        let file = rig.fixture.by_label("File").expect("the File title");
        rig.fixture.click(&file);
        rig.fixture.settle();
        assert!(rig.open("Save"));
        rig.key(key);
        assert!(rig.open("Cut") && !rig.open("Save"), "{direction:?}: the key toward the end edge opens Edit");
    }
}

#[trial]
fn f10_focuses_the_first_title_and_escape_returns_focus_to_the_previous_owner() {
    let mut rig = rig(Direction::Ltr);
    rig.key(Key::Tab);
    assert_eq!(rig.fixture.focused(), rig.button.get(), "the button holds focus");
    rig.key(Key::F10);
    assert_eq!(rig.fixture.focused(), rig.bar.get(), "F10 takes focus for the bar");
    rig.key(Key::Escape);
    assert_eq!(rig.fixture.focused(), rig.button.get(), "Escape with no menu open gives focus back");
}

#[trial]
fn running_a_row_returns_focus_to_the_component_that_held_it() {
    let mut rig = rig(Direction::Ltr);
    rig.key(Key::Tab);
    rig.key(Key::F10);
    rig.key(Key::ArrowDown);
    assert!(rig.open("Save"), "Down on a title opens its menu");
    let save = rig.fixture.by_label("Save").expect("the Save row");
    rig.fixture.click(&save);
    rig.fixture.settle();
    assert_eq!(*rig.ran.borrow(), ["Save"]);
    assert_eq!(rig.fixture.focused(), rig.button.get());
}

#[trial]
fn a_title_reports_its_popup_its_state_and_its_access_key() {
    let mut rig = rig(Direction::Ltr);
    let file = rig.fixture.by_label("File").expect("the File title");
    let tree = rig.fixture.tree();
    assert_eq!(tree.has_popup_of(file.node), Some(PopupKind::Menu));
    assert_eq!(tree.access_key_of(file.node), Some('F'));
    assert_eq!(tree.expanded_of(file.node), Some(false));
    rig.fixture.click(&file);
    rig.fixture.settle();
    assert_eq!(rig.fixture.tree().expanded_of(file.node), Some(true));
}

#[trial]
fn help_search_ignores_accents_and_matches_the_english_label_and_the_path() {
    let mut rig = rig(Direction::Ltr);
    let help = rig.fixture.by_label("Help").expect("the Help title");
    rig.fixture.click(&help);
    rig.fixture.settle();
    let search = rig.fixture.by_label("Search menus").expect("the search field");
    rig.fixture.click(&search);
    for (typed, found, absent) in [("cafe", "Paste", "Copy"), ("duplic", "Copy", "Paste"), ("clipboard", "Cut", "Copy")] {
        rig.fixture.key(Key::A, Modifiers::COMMAND);
        rig.fixture.type_text(typed);
        rig.fixture.settle();
        assert!(rig.open(found), "{typed:?} finds the entry whose command row reads {found:?}");
        assert!(!rig.open(absent), "{typed:?} leaves {absent:?} out");
    }
}

/// An entry found by its label ranks before one found only through its menu path, whatever their order.
#[trial]
fn a_label_match_ranks_before_a_path_only_match() {
    let texts = [["Zoom", "", "Window"], ["Window", "", "View"], ["Tile", "", "Window"]];
    assert_eq!(search_order(&Folder::new("en"), "window", &texts), vec![1, 0, 2]);
    assert!(search_order(&Folder::new("en"), "  ", &texts).is_empty());
}
