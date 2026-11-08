//! The context-menu key on a collection (9.2 rule 3): Shift+F10 and the Menu key at the lead record the context
//! request a `Menu::context` on the collection opens from, at the lead row, and the menu acts on the lead item.

use super::kit::{List, Row, Words};
use craft_host::target::Os;
use craft_ui::contract::{CollectionEvent, Props as _, Role, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn region() -> Rect {
    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(300.0, 200.0))
}

/// What the list and its menu reported.
#[derive(Default)]
struct Seen {
    /// The list's height.
    list: Cell<f32>,
    /// The key of the last menu request the list made.
    asked: Cell<Option<usize>>,
    /// The context request recorded for the list in the last pass that made one.
    request: Cell<Option<Rect>>,
    /// The keys the menu's row ran on.
    acted: RefCell<Vec<usize>>,
}

fn rig() -> (Fixture, Rc<Seen>) {
    let words: &'static Words = Box::leak(Box::new(Words::flat(&["one", "two", "three"])));
    let seen = Rc::new(Seen::default());
    let log = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = in_region(ui, region(), |ui| List::new(SelectMode::Single).id_salt("list").show_items(ui, words, &mut |_, _| Row::new()));
        log.list.set(out.response.rect.height());
        for event in &out.events {
            if let CollectionEvent::ContextMenu(request) = event {
                log.asked.set(Some(request.key));
            }
        }
        if let Some(rect) = craft_ui::testing::focus::context_request(ui.ctx(), out.response.id) {
            log.request.set(Some(rect));
        }
        Menu::context("Row menu", &out.response).show(ui, |ui, _| {
            if MenuItem::new("Remove").show(ui).ran
                && let Some(key) = log.asked.get()
            {
                log.acted.borrow_mut().push(key);
            }
        });
    });
    fixture.os(Os::Windows).settle();
    (fixture, seen)
}

fn rows(fixture: &Fixture) -> Vec<Target> {
    let mut rows = fixture.by_role(Role::LIST_ITEM);
    rows.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    rows
}

/// Opens the menu with `key` on the lead `row` and runs its row; the request lies on that row.
fn open_and_run(fixture: &mut Fixture, seen: &Seen, row: &Target, key: Key, mods: Modifiers) {
    seen.request.set(None);
    fixture.key(key, mods).settle();
    let request = seen.request.get().expect("the key records a context request for the list");
    assert!(request.contains(row.rect.center()), "the request is at the lead row: {request:?} vs {:?}", row.rect);
    assert!(request.height() < seen.list.get(), "the request is the row, not the list: {request:?}");
    let remove = fixture.by_label("Remove").expect("the key opens the list's context menu");
    fixture.click(&remove).settle();
}

/// Shift+F10 and the Menu key on a list open its context menu at the lead, and the menu's row acts on the lead item.
#[trial]
fn the_context_menu_key_opens_the_lists_menu_at_the_lead() {
    let (mut fixture, seen) = rig();
    let shown = rows(&fixture);
    assert_eq!(shown.len(), 3, "the list shows its rows");
    fixture.click(&shown[1]).settle();
    open_and_run(&mut fixture, &seen, &shown[1], Key::F10, Modifiers::SHIFT);
    assert_eq!(*seen.acted.borrow(), [1], "the menu acts on the lead item");
    assert!(fixture.by_label("Remove").is_none(), "the row's click closes the menu");
    fixture.click(&shown[2]).settle();
    open_and_run(&mut fixture, &seen, &shown[2], Key::Menu, Modifiers::NONE);
    assert_eq!(*seen.acted.borrow(), [1, 2], "the Menu key opens it at the new lead");
}
