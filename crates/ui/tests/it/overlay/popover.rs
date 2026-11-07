//! `Popover`: the close reason and the open binding written when a close rule fires.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::popover::Popover;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    open: Rc<Cell<bool>>,
    closed: Rc<Cell<Option<CloseReason>>>,
}

fn rig() -> Rig {
    let (open, closed) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(None)));
    let (shown_open, shown_closed) = (open.clone(), closed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut is_open = shown_open.get();
        let shown = Popover::new("Brush options", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            Button::new("Reset").show(ui);
        });
        shown_open.set(is_open);
        if shown.output.closed.is_some() {
            shown_closed.set(shown.output.closed);
        }
    });
    fixture.settle();
    Rig { fixture, open, closed }
}

/// Escape with the focus inside closes the popover, clears the caller's binding and reports `Escape` once.
#[trial]
fn escape_closes_a_popover_and_reports_why() {
    let mut rig = rig();
    assert!(rig.open.get(), "the popover starts open");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    rig.fixture.idle(30);
    assert!(!rig.open.get(), "Escape clears the open binding");
    assert_eq!(rig.closed.get(), Some(CloseReason::Escape));
}
