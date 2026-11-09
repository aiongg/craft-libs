//! A menu with several openers, and a menu a held press opened.

use craft_ui::buttons::Button;
use craft_ui::contract::overlay::{Anchor, HOLD};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _, Trigger};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::popover::Popover;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

fn menu_rect(fixture: &Fixture) -> Option<egui::Rect> {
    fixture.by_role(Role::MENU).first().map(|menu| menu.rect)
}

struct Ids {
    button: Cell<Option<Id>>,
    area: Cell<Option<Id>>,
}

/// A button that opens the menu with a press and a second control that opens it as a context menu.
fn two_openers() -> (Fixture, Rc<Ids>) {
    let ids = Rc::new(Ids { button: Cell::new(None), area: Cell::new(None) });
    let shown = ids.clone();
    let mut fixture = Fixture::new(move |ui| {
        let (button, area) = ui.horizontal(|ui| (Button::new("File").show(ui), Button::new("Area").show(ui))).inner;
        shown.button.set(Some(button.id));
        shown.area.set(Some(area.id));
        Menu::new("Actions").opened_by(&button, Trigger::Press).opened_by(&area, Trigger::Context).show(ui, |ui, _| {
            MenuItem::new("Open").show(ui);
            MenuItem::new("Save").show(ui);
        });
    });
    fixture.settle();
    (fixture, ids)
}

#[trial]
fn a_menu_opens_from_each_of_its_openers_at_the_anchor_of_the_one_that_opened_it() {
    let (mut fixture, ids) = two_openers();
    let button = fixture.node_of(ids.button.get().unwrap()).expect("the button has a node");
    let area = fixture.node_of(ids.area.get().unwrap()).expect("the second control has a node");

    fixture.click(&button);
    fixture.settle();
    let under_button = menu_rect(&fixture).expect("a press on the button opens the menu");
    assert!(under_button.top() >= button.rect.bottom() - 1.0, "the menu hangs under the button");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.settle();
    assert!(menu_rect(&fixture).is_none(), "Escape closes the menu");

    let at = area.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    let at_pointer = menu_rect(&fixture).expect("a secondary click on the second control opens the menu");
    assert!(at_pointer.top() < area.rect.bottom(), "the menu hangs at the pointer, inside the control");
    assert!(at_pointer.left() <= at.x + 1.0 && at_pointer.right() >= at.x, "the menu starts at the pointer");
}

#[trial]
fn the_menu_key_on_a_context_opener_opens_the_menu_and_escape_returns_the_focus_to_it() {
    let (mut fixture, ids) = two_openers();
    let area = fixture.node_of(ids.area.get().unwrap()).expect("the second control has a node");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Tab, Modifiers::NONE);
    assert_eq!(fixture.focused(), ids.area.get(), "the second control has the focus");
    fixture.key(Key::F10, Modifiers::SHIFT);
    fixture.settle();
    let menu = menu_rect(&fixture).expect("the Menu key opens the menu");
    assert!(menu.top() >= area.rect.top(), "a menu a key opened hangs from the opener");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.settle();
    assert!(menu_rect(&fixture).is_none(), "Escape closes the menu");
    assert_eq!(fixture.focused(), ids.area.get(), "the focus is back on the opener that opened the menu");
}

/// Press on the opener, hold past `HOLD`, move onto the second row and release there.
#[trial]
fn a_menu_a_held_press_opened_runs_the_row_the_press_ends_on() {
    let ran = Rc::new(Cell::new(None));
    let shown = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        let tool = Button::new("Tool").show(ui);
        Menu::new("Tool menu").opened_by(&tool, Trigger::Hold).show(ui, |ui, _| {
            for label in ["First", "Second"] {
                if MenuItem::new(label).show(ui).ran {
                    shown.set(Some(label));
                }
            }
        });
    });
    fixture.settle();
    let tool = fixture.by_label("Tool").expect("the opener has a node");
    fixture.press(&tool);
    fixture.advance(HOLD + 0.1);
    fixture.idle(4);
    assert!(menu_rect(&fixture).is_some(), "holding the press opens the menu");
    let row = fixture.by_role(Role::MENU_ITEM).into_iter().find(|row| fixture.tree().name_of(row.node) == Some("Second")).expect("the row shows");
    fixture.frame(vec![pointer::moved(row.center())]);
    fixture.frame(vec![pointer::release(row.center())]);
    fixture.settle();
    assert_eq!(ran.get(), Some("Second"), "the release ran the row under it");
    assert!(menu_rect(&fixture).is_none(), "running the row closed the menu");
}

/// One response that is both a Manual opener hung from a separate control and a Context opener.
fn manual_and_context(open_bound: bool) -> (Fixture, Rc<Ids>) {
    let ids = Rc::new(Ids { button: Cell::new(None), area: Cell::new(None) });
    let shown = ids.clone();
    let mut fixture = Fixture::new(move |ui| {
        let (menu_button, target) = ui.horizontal(|ui| (Button::new("Menu").show(ui), Button::new("Target").show(ui))).inner;
        shown.button.set(Some(menu_button.id));
        shown.area.set(Some(target.id));
        let mut menu = Menu::new("Actions").opened_by(&target, Trigger::Manual).anchor(Anchor::Rect(menu_button.rect));
        let mut open = true;
        if open_bound {
            menu = menu.open(Bind::bound(&mut open));
        }
        menu.opened_by(&target, Trigger::Context).show(ui, |ui, _| {
            MenuItem::new("Open").show(ui);
        });
    });
    fixture.settle();
    (fixture, ids)
}

#[trial]
fn a_context_opening_shares_the_response_of_a_manual_opener_and_hangs_at_the_pointer() {
    let (mut fixture, ids) = manual_and_context(false);
    let target = fixture.node_of(ids.area.get().unwrap()).expect("the target has a node");
    let button = fixture.node_of(ids.button.get().unwrap()).expect("the menu button has a node");
    let at = target.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    let menu = menu_rect(&fixture).expect("a secondary click on the target opens the menu");
    assert!(menu.left() >= button.rect.right() - 1.0, "the menu does not hang under the menu button: {menu:?} vs {:?}", button.rect);
    assert!(menu.left() <= at.x + 1.0 && menu.right() >= at.x, "the menu starts at the pointer");
}

#[trial]
fn a_context_key_on_a_shared_response_returns_the_focus_to_it_when_the_menu_closes() {
    let (mut fixture, ids) = manual_and_context(false);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Tab, Modifiers::NONE);
    assert_eq!(fixture.focused(), ids.area.get(), "the target has the focus");
    fixture.key(Key::F10, Modifiers::SHIFT);
    fixture.settle();
    assert!(menu_rect(&fixture).is_some(), "the Menu key opens the menu");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.settle();
    assert!(menu_rect(&fixture).is_none(), "Escape closes the menu");
    assert_eq!(fixture.focused(), ids.area.get(), "the focus returns to the response");
}

#[trial]
fn a_menu_a_manual_opener_opens_hangs_from_the_opener_rect() {
    let (fixture, ids) = manual_and_context(true);
    let button = fixture.node_of(ids.button.get().unwrap()).expect("the menu button has a node");
    let menu = menu_rect(&fixture).expect("the bound menu is open");
    assert!(menu.top() >= button.rect.bottom() - 1.0, "the menu hangs under the menu button: {menu:?} vs {:?}", button.rect);
}

/// A context menu whose opener sits inside an open popover opens on the first secondary click, and the popover
/// keeps its content.
#[trial]
fn a_context_menu_opened_from_inside_a_popover_leaves_the_popover_open() {
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut open = true;
        Popover::new("Brush options", Bind::bound(&mut open)).opened_by(&opener).show(ui, |ui, _| {
            let target = Button::new("Target").show(ui);
            Button::new("Reset").show(ui);
            Menu::context("Target menu", &target).show(ui, |ui, _| {
                MenuItem::new("Delete").show(ui);
            });
        });
    });
    fixture.settle();
    let target = fixture.by_label("Target").expect("the target has a node");
    fixture.click(&target);
    fixture.settle();
    let at = target.center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    assert!(menu_rect(&fixture).is_some(), "a secondary click on the target opens the menu");
    assert!(fixture.by_label("Reset").is_some(), "the popover keeps its content");
}

/// A context menu opened over the popover it was opened from takes the click on its row: the row runs and the menu
/// closes, and the popover stays open below it.
#[trial]
fn a_menu_opened_from_a_popover_takes_a_click_on_its_row() {
    let ran = Rc::new(Cell::new(false));
    let seen = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut open = true;
        Popover::new("Brush options", Bind::bound(&mut open)).opened_by(&opener).show(ui, |ui, _| {
            let target = Button::new("Target").show(ui);
            Button::new("Reset").show(ui);
            Button::new("Swap").show(ui);
            Menu::context("Target menu", &target).show(ui, |ui, _| {
                if MenuItem::new("Delete").show(ui).ran {
                    seen.set(true);
                }
            });
        });
    });
    fixture.settle();
    let at = fixture.by_label("Target").expect("the target has a node").center();
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.settle();
    let delete = fixture.by_label("Delete").expect("a secondary click on the target opens the menu");
    let popover = fixture.by_role(Role::DIALOG).first().map(|popover| popover.rect).expect("the popover is open");
    assert!(popover.contains(delete.center()), "the row lies over the popover");
    fixture.click(&delete);
    fixture.settle();
    assert!(ran.get(), "the click ran the row");
    assert!(menu_rect(&fixture).is_none(), "the menu closed");
    assert!(fixture.by_label("Reset").is_some(), "the popover stays open");
}

/// A menu taller than both sides of its opener takes the side with more room, is capped to it, and scrolls its rows:
/// it never covers its opener, and the arrow keys reach its last row.
#[trial]
fn a_menu_taller_than_either_side_of_its_opener_stays_on_one_side_and_scrolls_to_its_last_row() {
    let mut fixture = Fixture::new(|ui| {
        ui.add_space(280.0);
        let button = Button::new("Open").show(ui);
        Menu::new("Long").opened_by(&button, Trigger::Press).show(ui, |ui, _| {
            for at in 0..17 {
                MenuItem::new(format!("Row {at}")).show(ui);
            }
        });
    });
    fixture.settle();
    let button = fixture.by_label("Open").expect("the opener has a node");
    fixture.click(&button);
    fixture.settle();
    let menu = menu_rect(&fixture).expect("a press on the button opens the menu");
    let room = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    assert!(room.contains_rect(menu), "the menu {menu:?} stays inside the room");
    assert!(!menu.intersect(button.rect).is_positive(), "the menu {menu:?} leaves the opener {:?} uncovered", button.rect);
    for _ in 0..16 {
        fixture.key(Key::ArrowDown, Modifiers::NONE);
        fixture.settle();
    }
    let last = fixture.by_label("Row 16").expect("the last row has a node");
    let menu = menu_rect(&fixture).expect("the menu is still open");
    assert!(menu.contains_rect(last.rect), "the last row {:?} is revealed inside the menu {menu:?}", last.rect);
}

/// A context menu its owner opens through the open binding, a pass after the press, keeps the place it opened at
/// when the pointer leaves the opener for the menu.
#[trial]
fn a_context_menu_opened_by_its_binding_stays_where_it_opened_when_the_pointer_leaves_the_opener() {
    let wanted = Rc::new(Cell::new(false));
    let shown = wanted.clone();
    let mut fixture = Fixture::new(move |ui| {
        let target = Button::new("Target").show(ui);
        if shown.get() {
            let mut open = true;
            Menu::context("Actions", &target).open(Bind::bound(&mut open)).show(ui, |ui, _| {
                MenuItem::new("Open").show(ui);
            });
        }
    });
    fixture.settle();
    let target = fixture.by_label("Target").expect("the target shows").rect;
    let at = target.right_bottom() - egui::vec2(2.0, 2.0);
    fixture.frame(vec![pointer::moved(at)]);
    wanted.set(true);
    fixture.settle();
    let opened = menu_rect(&fixture).expect("the bound menu is open");
    let row = fixture.by_role(Role::MENU_ITEM).pop().expect("the menu has a row");
    assert!(!target.contains(row.center()), "the row lies off the opener");
    fixture.frame(vec![pointer::moved(row.center())]);
    fixture.settle();
    assert_eq!(menu_rect(&fixture), Some(opened), "the menu does not move when the pointer leaves the opener");
}
