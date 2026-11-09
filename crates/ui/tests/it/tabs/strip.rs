//! `TabStrip`: which tabs fit, and that a click on a tab reports it selected.

use craft_behavior::{DisplayOrder, ItemSource, SelectionChange};
use craft_ui::contract::{CollectionEvent, ListDrag, ShowCollection as _};
use craft_ui::runtime::drag;
use craft_ui::tabs::{Tab, TabStrip};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;
use std::cell::RefCell;
use std::rc::Rc;

const TITLES: [&str; 6] = ["Untitled-1", "Photo.psd", "Poster final.psd", "Logo.ai", "Sketch", "Notes"];
const WIDE: Vec2 = Vec2::new(900.0, 80.0);
const NARROW: Vec2 = Vec2::new(220.0, 80.0);

struct Titles;

impl DisplayOrder<usize> for Titles {
    fn len(&self) -> usize {
        TITLES.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < TITLES.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        TITLES.get(at).map(|title| std::borrow::Cow::Borrowed(*title))
    }
}

impl ItemSource<usize> for Titles {}

/// A strip of six tabs in `room` with the first one shown; the keys of every selection change it reports.
fn rig(room: Vec2) -> (Fixture, Rc<RefCell<Vec<usize>>>) {
    let picked = Rc::new(RefCell::new(Vec::new()));
    let seen = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = TabStrip::new("Documents").shown(0).show_items(ui, &Titles, &mut |key, _| Tab::new(TITLES[*key]));
        for event in out.response.events {
            if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event {
                seen.borrow_mut().extend(keys);
            }
        }
    });
    fixture.room(room);
    fixture.settle();
    (fixture, picked)
}

/// A tab the strip is too narrow to fit is not in the tree; the overflow menu still lists it.
#[trial]
fn the_tabs_that_do_not_fit_have_no_node_and_the_overflow_menu_lists_them() {
    let (mut fixture, _) = rig(NARROW);
    let hidden: Vec<&str> = TITLES.iter().copied().filter(|title| fixture.by_label(title).is_none()).collect();
    assert!(!hidden.is_empty() && hidden.len() < TITLES.len(), "some tabs fit and some do not: {hidden:?}");
    let more = fixture.by_label("More tabs").expect("the overflow button is named");
    fixture.click(&more);
    fixture.settle();
    for title in hidden {
        assert!(fixture.by_label(title).is_some(), "the overflow menu lists {title}");
    }
}

/// What the strip's drag carries: the grabbed tab.
struct Grabbed(usize);

/// Ctrl+Tab on the focused strip selects the next tab, wrapping at the end; Ctrl+Shift+Tab goes back.
#[trial]
fn ctrl_tab_switches_the_tabs_of_a_focused_strip() {
    use egui::{Key, Modifiers};

    let shown = Rc::new(std::cell::Cell::new(TITLES.len() - 2));
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = TabStrip::new("Documents").shown(seen.get()).show_items(ui, &Titles, &mut |key, _| Tab::new(TITLES[*key]));
        for event in out.response.events {
            if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event {
                seen.set(keys[0]);
            }
        }
    });
    fixture.room(WIDE);
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::CTRL);
    fixture.settle();
    assert_eq!(shown.get(), TITLES.len() - 1, "Ctrl+Tab selects the next tab");
    fixture.key(Key::Tab, Modifiers::CTRL);
    fixture.settle();
    assert_eq!(shown.get(), 0, "it wraps to the first tab");
    fixture.key(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
    fixture.settle();
    assert_eq!(shown.get(), TITLES.len() - 1, "Ctrl+Shift+Tab goes back");
}

/// A strip scrolls along its tabs only, so it sits in a dialog body that scrolls vertically and Ctrl+Tab still
/// switches its tabs.
#[trial]
fn a_strip_in_a_dialog_body_switches_pages() {
    use craft_ui::contract::{Bind, ShowContainer as _};
    use craft_ui::dialog::Dialog;
    use craft_ui::runtime::focus::Initial;
    use egui::{Key, Modifiers};

    let shown = Rc::new(std::cell::Cell::new(0));
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        Dialog::new("Pages").opened(Bind::bound(&mut open)).initial_focus(Initial::First).show(ui, |ui, _| {
            let out = TabStrip::new("Documents").shown(seen.get()).show_items(ui, &Titles, &mut |key, _| Tab::new(TITLES[*key]));
            for event in out.response.events {
                if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event {
                    seen.set(keys[0]);
                }
            }
        });
    });
    fixture.room(WIDE);
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::CTRL);
    fixture.settle();
    assert_eq!(shown.get(), 1, "Ctrl+Tab selects the next tab");
}
