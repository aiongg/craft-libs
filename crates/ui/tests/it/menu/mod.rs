//! `Menu` and `MenuItem`: the mnemonic of each row of a level, what running a row does to the menu, the one open
//! submenu, value menus and the panel rows.

mod bar;

mod item_access;

mod native;

mod openers;

mod pick_focus;

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Entry, Role, Show as _, ShowContainer as _, Trigger};
use craft_ui::menu::{GroupItem, Menu, MenuItem, PanelMenu, PanelRow};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Pos2};
use std::cell::Cell;
use std::rc::Rc;

const ROWS: [&str; 3] = ["&Open", "Save", "Save As"];

fn rows(ui: &mut egui::Ui, ran: &Cell<Option<&'static str>>) {
    for label in ROWS {
        if MenuItem::new(label).show(ui).ran {
            ran.set(Some(label));
        }
    }
}

fn menu_shown(fixture: &Fixture) -> bool {
    !fixture.by_role(Role::MENU).is_empty()
}

/// A menu opened by a press of its opener, with no binding and no row focused: the container reads the typed letter.
#[trial]
fn a_typed_letter_runs_a_row_of_a_menu_opened_by_the_pointer() {
    let ran = Rc::new(Cell::new(None));
    let shown_ran = ran.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("File").show(ui);
        Menu::new("File").opened_by(&opener, Trigger::Press).show(ui, |ui, _| rows(ui, &shown_ran));
    });
    fixture.settle();
    let opener = fixture.by_label("File").expect("the opener has a node");
    fixture.click(&opener);
    fixture.idle(4);
    assert!(menu_shown(&fixture), "a press of the opener opens the menu");
    fixture.type_text("a");
    fixture.idle(4);
    assert_eq!(ran.get(), Some("Save As"));
    assert!(!menu_shown(&fixture), "running a row closes the menu");
}

/// The names of the menus on screen, sorted.
fn menus(fixture: &Fixture) -> Vec<String> {
    let tree = fixture.tree();
    let mut names: Vec<String> = fixture.by_role(Role::MENU).iter().filter_map(|menu| tree.name_of(menu.node)).map(str::to_owned).collect();
    names.sort();
    names
}

/// The pointer rests on a row for longer than the submenu delay.
fn rest_on(fixture: &mut Fixture, row: &str) {
    let at: Pos2 =
        fixture.by_role(Role::MENU_ITEM).iter().find(|target| fixture.tree().name_of(target.node) == Some(row)).expect("the row shows").center();
    fixture.frame(vec![Event::PointerMoved(at)]);
    fixture.advance(0.5);
    fixture.idle(6);
}

#[trial]
fn resting_on_a_second_submenu_row_closes_the_first_submenu() {
    let submenu = |label: &str, inner: &str| {
        PanelRow::Submenu(label.to_owned().into(), vec![PanelRow::Item(Entry::new(GroupItem::MoveTo(0), inner.to_owned()))])
    };
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Panel").show(ui);
        Menu::new("Panel").opened_by(&opener, Trigger::Press).show(ui, |ui, _| {
            PanelMenu::new(vec![submenu("Alpha", "Alpha One"), submenu("Beta", "Beta One")], Vec::new()).show(ui);
        });
    });
    fixture.settle();
    let opener = fixture.by_label("Panel").expect("the opener has a node");
    fixture.click(&opener);
    fixture.idle(4);
    rest_on(&mut fixture, "Alpha");
    assert_eq!(menus(&fixture), ["Alpha", "Panel"], "the submenu of the row the pointer rests on is open");
    rest_on(&mut fixture, "Beta");
    assert_eq!(menus(&fixture), ["Beta", "Panel"], "only the second submenu is open");
}

fn zoom(checked: Option<u8>) -> Vec<Entry<u8>> {
    let entry = |value: u8, label: &str| Entry::new(value, label.to_owned()).checked(checked == Some(value));
    vec![entry(0, "Fit in Window"), entry(1, "Fill Screen"), Entry::separator(), entry(2, "100%")]
}

fn value_menu(checked: Option<u8>) -> Fixture {
    let entries = zoom(checked);
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Zoom").show(ui);
        let mut open = true;
        Menu::new("Zoom").values().open(Bind::bound(&mut open)).opened_by(&opener, Trigger::Click).pick(ui, &entries);
    });
    fixture.settle();
    fixture
}

#[trial]
fn a_value_menu_opens_with_the_lead_on_the_checked_entry() {
    let fixture = value_menu(Some(2));
    let focused = fixture.focused().and_then(|id| fixture.node_of(id)).expect("a row holds the focus");
    let checked = fixture.by_label("100%").expect("the checked entry shows");
    assert_eq!(focused.rect, checked.rect);
}

