//! `Menu` and `MenuItem`: the mnemonic of each row of a level, and what running a row does to the menu.

mod bar;

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::testing::{Fixture, trial};
use std::cell::Cell;
use std::rc::Rc;

const ROWS: [&str; 3] = ["&Open", "Save", "Save As"];

struct Rig {
    fixture: Fixture,
    open: Rc<Cell<bool>>,
    ran: Rc<Cell<Option<&'static str>>>,
}

fn rig() -> Rig {
    let (open, ran) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(None)));
    let (shown_open, shown_ran) = (open.clone(), ran.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("File").show(ui);
        let mut is_open = shown_open.get();
        Menu::new("File", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            for label in ROWS {
                if MenuItem::new(label).show(ui).ran {
                    shown_ran.set(Some(label));
                }
            }
        });
        shown_open.set(is_open);
    });
    fixture.settle();
    Rig { fixture, open, ran }
}

/// A row with a `&` marker keeps its letter; the others take the first letter no earlier row uses, so the third row
/// is `A` ("Save" holds `S`). The typed letter runs that row even though another row has the focus, and running a row
/// closes the menu.
#[trial]
fn a_typed_letter_runs_the_row_it_names_and_closes_the_menu() {
    let mut rig = rig();
    assert!(rig.open.get(), "the menu starts open");
    rig.fixture.type_text("a");
    rig.fixture.idle(4);
    assert_eq!(rig.ran.get(), Some("Save As"));
    assert!(!rig.open.get(), "running a row closes the menu");
}
