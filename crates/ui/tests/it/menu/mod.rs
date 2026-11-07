//! `Menu` and `MenuItem`: the mnemonic of each row of a level, what running a row does to the menu, the one open
//! submenu, value menus and the panel rows.

mod bar;

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

/// A row with a `&` marker keeps its letter; the others take the first letter no earlier row uses, so the third row
/// is `A` ("Save" holds `S`). The typed letter runs that row even though another row has the focus, and running a row
/// closes the menu.
#[trial]
fn a_typed_letter_runs_the_row_it_names_and_closes_the_menu() {
    let (open, ran) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(None)));
    let (shown_open, shown_ran) = (open.clone(), ran.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("File").show(ui);
        let mut is_open = shown_open.get();
        Menu::new("File").open(Bind::bound(&mut is_open)).opened_by(&opener, Trigger::Click).show(ui, |ui, _| rows(ui, &shown_ran));
        shown_open.set(is_open);
    });
    fixture.settle();
    assert!(open.get(), "the menu starts open");
    fixture.type_text("a");
    fixture.idle(4);
    assert_eq!(ran.get(), Some("Save As"));
    assert!(!open.get(), "running a row closes the menu");
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
    // The closed menu fades out before its nodes leave the tree.
    fixture.idle(30);
    assert!(menu_shown(&fixture), "a press of the opener opens the menu");
    fixture.type_text("a");
    fixture.idle(30);
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
    // The layer of a submenu that closed fades out before it leaves the tree.
    fixture.settle();
}

#[trial]
fn resting_on_a_second_submenu_row_closes_the_first_submenu() {
    let submenu =
        |label: &str, inner: &str| PanelRow::Submenu(label.to_owned().into(), vec![PanelRow::Item(Entry::new(GroupItem::Float, inner.to_owned()))]);
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

/// A check row that keeps the menu open toggles its state and the menu stays.
#[trial]
fn a_keep_open_row_runs_and_the_menu_stays_open() {
    let (checked, runs, open) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(0)), Rc::new(Cell::new(true)));
    let (shown_checked, shown_runs, shown_open) = (checked.clone(), runs.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("View").show(ui);
        let mut is_open = shown_open.get();
        Menu::new("View").open(Bind::bound(&mut is_open)).opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            if MenuItem::new("Show Rulers").check(shown_checked.get()).keep_open(true).show(ui).ran {
                shown_checked.set(!shown_checked.get());
                shown_runs.set(shown_runs.get() + 1);
            }
        });
        shown_open.set(is_open);
    });
    fixture.settle();
    let row = fixture.by_role(Role::MENU_ITEM_CHECK_BOX).pop().expect("the row is a check item");
    fixture.click(&row);
    fixture.idle(4);
    assert_eq!((runs.get(), checked.get()), (1, true), "the row ran and toggled");
    assert!(open.get(), "a keep-open row leaves the menu open");
}

/// The mnemonic of "Save As" in a level where "Save" holds `S` is `A`, and the index points at the "A" of "As", not
/// the "a" of "Save".
#[trial]
fn the_mnemonic_index_is_the_character_of_the_letter() {
    let marks = craft_keymap::mnemonic::assign(&["&Open", "Save", "Save As"]);
    assert_eq!(marks, [Some(('O', 0)), Some(('S', 0)), Some(('A', 5))]);
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
fn a_value_menu_has_radio_rows_whether_or_not_an_entry_is_checked() {
    assert_eq!(value_menu(None).by_role(Role::MENU_ITEM_RADIO).len(), 3, "nothing checked");
    assert_eq!(value_menu(Some(1)).by_role(Role::MENU_ITEM_RADIO).len(), 3, "one checked");
}

#[trial]
fn a_value_menu_opens_with_the_lead_on_the_checked_entry() {
    let fixture = value_menu(Some(2));
    let focused = fixture.focused().and_then(|id| fixture.node_of(id)).expect("a row holds the focus");
    let checked = fixture.by_label("100%").expect("the checked entry shows");
    assert_eq!(focused.rect, checked.rect);
}

#[trial]
fn a_disabled_panel_row_does_not_run() {
    let picked = Rc::new(Cell::new(None));
    let shown_picked = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Panel").show(ui);
        let mut open = true;
        Menu::new("Panel").open(Bind::bound(&mut open)).opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            let rows = vec![
                PanelRow::Item(Entry::new(GroupItem::StepGroup(true), "Move Group Up".to_owned()).enabled(false)),
                PanelRow::Item(Entry::new(GroupItem::Float, "Float".to_owned())),
            ];
            if let Some(item) = PanelMenu::new(Vec::new(), rows).show(ui).picked {
                shown_picked.set(Some(item));
            }
        });
    });
    fixture.settle();
    let disabled = fixture.by_label("Move Group Up").expect("the disabled row shows");
    fixture.click(&disabled);
    fixture.idle(2);
    assert_eq!(picked.get(), None, "a disabled row reports nothing");
    let enabled = fixture.by_label("Float").expect("the enabled row shows");
    fixture.click(&enabled);
    fixture.idle(2);
    assert_eq!(picked.get(), Some(GroupItem::Float), "an enabled row reports its item");
}
