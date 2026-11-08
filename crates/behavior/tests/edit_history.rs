//! `EditHistory`: taking back a refused step returns the value before it and keeps the redo steps.

use craft_behavior::edit_history::Step;
use craft_behavior::{EditEvent, EditHistory, Mixed};

fn set(before: i32, after: i32) -> EditEvent<i32> {
    EditEvent::set(Mixed::Same(before), after, "Fill".into(), None)
}

#[test]
fn taking_back_the_last_step_returns_the_value_before_it_and_keeps_the_redo_steps() {
    let mut history = EditHistory::new(&0);
    history.record(&set(0, 1));
    history.record(&set(1, 2));
    let mut now = 2;
    assert!(history.step(&mut now, Step::Undo));
    assert_eq!(now, 1);
    assert_eq!(history.take_back(), Some(0), "the refused first step gives back the value before it");
    assert!(!history.can_undo());
    assert!(history.can_redo(), "taking back leaves the redo steps");
    assert_eq!(history.take_back(), None, "nothing is left to take back");
}
