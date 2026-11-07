//! The rules of the tour: with the focus on the default action, Right goes to the next step and Left back, because
//! the card's actions are separate Tab stops and leave the arrows to the tour's keys.

use craft_ui::contract::Bind;
use craft_ui::patterns::panels::{Tour, TourStep};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

fn tour(index: Rc<Cell<usize>>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let steps =
            [TourStep::new("First", "The first step."), TourStep::new("Second", "The second step."), TourStep::new("Third", "The third step.")];
        let mut at = index.get();
        Tour::new("Tour", &steps, Bind::bound(&mut at)).show(ui);
        index.set(at);
    });
    fixture.settle();
    fixture
}

/// Right and Left reach the tour's keys while the focus is on a card action.
#[trial]
fn the_arrows_step_the_tour_with_the_focus_on_an_action() {
    let index = Rc::new(Cell::new(0));
    let mut fixture = tour(index.clone());
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.settle();
    assert_eq!(index.get(), 1, "Right goes to the next step");
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.settle();
    assert_eq!(index.get(), 0, "Left goes back");
}
