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

/// A strip whose tabs fit shows no overflow button; one too narrow for them shows it.
#[trial]
fn the_overflow_button_shows_only_when_the_tabs_do_not_fit() {
    let (wide, _) = rig(WIDE);
    assert!(wide.by_label("More tabs").is_none());
    let (narrow, _) = rig(NARROW);
    assert!(narrow.by_label("More tabs").is_some());
}

/// A click on a tab reports it as the new selection.
#[trial]
fn a_click_on_a_tab_reports_it_selected() {
    let (mut fixture, picked) = rig(WIDE);
    let tab = fixture.by_label("Photo.psd").expect("the tab is named");
    fixture.click(&tab);
    assert_eq!(picked.borrow().as_slice(), &[1]);
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

/// A tab drag carries the mapped payload, and a tab the map gives `None` for does not drag.
#[trial]
fn a_tab_drag_carries_the_mapped_payload_and_a_tab_mapped_to_none_does_not_drag() {
    let carried = Rc::new(RefCell::new(Vec::new()));
    let seen = carried.clone();
    let mut fixture = Fixture::new(move |ui| {
        TabStrip::new("Documents").shown(0).drag_payload(|drag: &ListDrag<usize>| (drag.grabbed != 2).then_some(Grabbed(drag.grabbed))).show_items(
            ui,
            &Titles,
            &mut |key, _| Tab::new(TITLES[*key]),
        );
        if let Some(payload) = drag::active::<Grabbed>(ui.ctx()) {
            seen.borrow_mut().push(payload.0);
        }
    });
    fixture.room(WIDE);
    fixture.settle();
    let tab = fixture.by_label("Photo.psd").expect("the tab is named");
    fixture.drag(&tab, Vec2::new(30.0, 40.0));
    assert!(carried.borrow().iter().all(|&held| held == 1) && !carried.borrow().is_empty(), "the drag carries tab 1: {:?}", carried.borrow());
    carried.borrow_mut().clear();
    let other = fixture.by_label("Poster final.psd").expect("the tab is named");
    fixture.drag(&other, Vec2::new(30.0, 40.0));
    assert!(carried.borrow().is_empty(), "a tab mapped to None carries nothing");
}
