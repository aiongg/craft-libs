//! A command picked from a menu acts where focus was before the menu opened: the pick closes the menu, focus returns
//! to the stop that held it, and only then does the command run. Undo picked from the Edit menu, or found through the
//! Help search, undoes the typing in the text field that held focus. Any other activation that closes a menu returns
//! focus the same way; Escape is no activation and leaves focus on the bar title.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::menu::MenuItem;
use craft_ui::menu::bar::{MenuBar, MenuHelp, MenuTitle};
use craft_ui::menu::search::HelpEntry;
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Id, Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A menu bar with Edit (Undo, Redo) and Help (the search over Undo), then a text field holding "hello" that has
/// focus and the typed run "abc".
struct Rig {
    fixture: Fixture,
    text: Rc<RefCell<String>>,
    field: Rc<Cell<Option<Id>>>,
}

fn rig() -> Rig {
    let text = Rc::new(RefCell::new("hello".to_owned()));
    let field = Rc::new(Cell::new(None));
    let shown = (text.clone(), field.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (text, field) = &shown;
        let titles = vec![
            MenuTitle::new("&Edit", |ui| {
                MenuItem::command(EditCommands::Undo).show(ui);
                MenuItem::command(EditCommands::Redo).show(ui);
            }),
            MenuTitle::new("&Help", |ui| {
                MenuItem::new("&About").show(ui);
            }),
        ];
        let help = vec![HelpEntry::new(EditCommands::Undo, "Undo").with_path(["Edit"])];
        MenuBar::new("Main menu", titles).help(MenuHelp::new(1, &help)).show(ui);
        let mut held = text.borrow_mut();
        field.set(Some(TextField::new("Name", Bind::bound(&mut *held)).show(ui).response.id));
    });
    fixture.settle();
    let name = fixture.by_label("Name").expect("the text field");
    fixture.click(&name);
    fixture.type_text("abc");
    // The typing run ends, so one undo takes back all of it.
    fixture.advance(2.0);
    fixture.settle();
    assert_eq!(*text.borrow(), "helloabc", "the typing reaches the field");
    Rig { fixture, text, field }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("{label:?} shows"));
        self.fixture.click(&target);
        self.fixture.settle();
    }

    fn assert_undone_in_the_field(&self, path: &str) {
        assert_eq!(*self.text.borrow(), "hello", "{path}: Undo takes back the typing in the field");
        assert_eq!(self.fixture.focused_stop().map(|(id, _)| id), self.field.get(), "{path}: focus is back in the field");
        assert!(self.fixture.by_label("Redo").is_none() && self.fixture.by_label("About").is_none(), "{path}: the pick closes the menu");
    }
}

#[trial]
fn undo_picked_from_the_edit_menu_or_found_through_the_help_search_undoes_in_the_field_that_held_focus() {
    let mut rig = rig();
    rig.click("Edit");
    rig.click("Undo");
    rig.assert_undone_in_the_field("Edit › Undo");

    let mut rig = self::rig();
    rig.click("Help");
    rig.fixture.type_text("undo");
    rig.fixture.settle();
    // From the search field, Tab moves to the found row, and Enter runs it.
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.settle();
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    rig.fixture.settle();
    rig.assert_undone_in_the_field("Help search › Undo");
}

/// A button holding focus, then a menu bar whose View menu holds a button and a row; the ids of the button outside
/// the bar and of the bar are recorded.
fn bar_rig() -> (Fixture, Rc<Cell<Option<Id>>>, Rc<Cell<Option<Id>>>) {
    let (apply, bar) = (Rc::new(Cell::new(None)), Rc::new(Cell::new(None)));
    let shown = (apply.clone(), bar.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (apply, bar) = &shown;
        apply.set(Some(Button::new("Apply").show(ui).id));
        let titles = vec![MenuTitle::new("&View", |ui| {
            Button::new("Reset").show(ui);
            MenuItem::new("&Zoom").show(ui);
        })];
        bar.set(Some(MenuBar::new("Main menu", titles).show(ui).response.id));
    });
    fixture.settle();
    for key in [Key::Tab, Key::F10, Key::ArrowDown] {
        fixture.key(key, Modifiers::NONE);
        fixture.settle();
    }
    assert!(fixture.by_label("Zoom").is_some(), "Down on the focused title opens its menu");
    (fixture, apply, bar)
}

#[trial]
fn running_or_activating_in_a_bar_menu_returns_focus_to_where_the_bar_took_it_and_escape_leaves_it_on_the_bar() {
    let (mut fixture, apply, _) = bar_rig();
    let reset = fixture.by_label("Reset").expect("the button in the menu");
    fixture.click(&reset);
    fixture.settle();
    assert!(fixture.by_label("Zoom").is_none(), "the activation closes the menu");
    assert_eq!(fixture.focused_stop().map(|(id, _)| id), apply.get(), "focus is back on the button F10 took it from");

    let (mut fixture, apply, _) = bar_rig();
    let zoom = fixture.by_label("Zoom").expect("the row in the menu");
    fixture.click(&zoom);
    fixture.settle();
    assert!(fixture.by_label("Zoom").is_none(), "running a row closes the menu");
    assert_eq!(fixture.focused_stop().map(|(id, _)| id), apply.get(), "running a row returns focus to the component that held it");

    let (mut fixture, _, bar) = bar_rig();
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.settle();
    assert!(fixture.by_label("Zoom").is_none(), "Escape closes the menu");
    assert_eq!(fixture.focused_stop().map(|(id, _)| id), bar.get(), "focus returns to the bar title");
}

#[trial]
fn help_menu_reopened_after_escape_focuses_its_search_field() {
    let mut rig = rig();
    for open in 1..=3 {
        rig.click("Help");
        let stop = rig.fixture.focused_stop();
        assert!(stop.is_some(), "open {open}: the search field has focus");
        assert_ne!(stop.map(|(id, _)| id), rig.field.get(), "open {open}: focus is in the popup, not the Name field");
        rig.fixture.key(Key::Escape, Modifiers::NONE);
        rig.fixture.settle();
        assert_eq!(rig.fixture.focused_stop().map(|(id, _)| id), rig.field.get(), "open {open}: Escape returns focus to the Name field");
    }
    rig.click("Help");
    rig.fixture.type_text("undo");
    rig.fixture.settle();
    assert!(rig.fixture.by_label("Undo").is_some(), "the text reaches the search field");
}
