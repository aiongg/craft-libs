//! `Button`: its output reports the press count of the click that activated it.

use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
use craft_ui::testing::pointer;
use craft_ui::testing::{Fixture, trial};
use egui::Pos2;
use std::cell::RefCell;
use std::rc::Rc;

/// A fixture showing one button, and the click counts it reported in the passes that activated it.
fn rig() -> (Fixture, Rc<RefCell<Vec<u32>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = Button::new("Apply").show(ui);
        if out.clicks > 0 {
            shown.borrow_mut().push(out.clicks);
        }
    });
    fixture.settle();
    (fixture, seen)
}

fn press_release(fixture: &mut Fixture, at: Pos2, times: usize) {
    fixture.frame(vec![pointer::moved(at)]);
    for _ in 0..times {
        fixture.frame(vec![pointer::press(at)]);
        fixture.frame(vec![pointer::release(at)]);
    }
    fixture.idle(2);
}

/// A single click reports one press and a double click reports the second click as two.
#[trial]
fn a_double_click_reports_two_clicks() {
    let (mut fixture, seen) = rig();
    let at = fixture.by_label("Apply").expect("the button has a node").center();
    press_release(&mut fixture, at, 1);
    assert_eq!(*seen.borrow(), vec![1], "a single click counts one");
    seen.borrow_mut().clear();
    fixture.advance(2.0);
    press_release(&mut fixture, at, 2);
    assert_eq!(*seen.borrow(), vec![1, 2], "the second click of a double click counts two");
}
