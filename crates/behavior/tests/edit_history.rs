//! `EditHistory`: dropping the newest step leaves the ones before it to undo.

use craft_behavior::edit_history::Step;
use craft_behavior::{EditEvent, EditHistory, GestureId, Mixed};

fn set(n: u64, before: i32, after: i32) -> EditEvent<i32> {
    EditEvent::set(GestureId::new(n), Mixed::Same(before), after, "Fill".into(), None)
}

#[test]
fn dropping_the_last_step_leaves_the_first_to_undo_and_keeps_the_redo_steps() {
    let mut history = EditHistory::new(&0);
    history.record(&set(1, 0, 1));
    history.record(&set(2, 1, 2));
    assert!(history.drop_last());
    let mut now = 2;
    assert!(history.step(&mut now, Step::Undo));
    assert_eq!(now, 0, "the undo applies the first step");
    assert!(!history.can_undo());
    assert!(history.can_redo());
    assert!(!history.drop_last(), "nothing is left to drop");
    assert!(history.can_redo(), "dropping nothing leaves the redo steps");
}
