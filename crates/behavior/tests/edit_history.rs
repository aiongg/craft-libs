//! `EditHistory`: taking back a refused step returns the value before it and keeps the redo steps.

use craft_behavior::edit_history::Step;
use craft_behavior::{EditEvent, EditHistory, Mixed};

fn set(before: i32, after: i32) -> EditEvent<i32> {
    EditEvent::set(Mixed::Same(before), after, "Fill".into(), None)
}

