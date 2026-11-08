//! `EditResponse::lifted`: a composite's response carries the lifted event and clear flags.

use craft_behavior::{EditEvent, GestureId, Mixed};
use craft_ui::contract::{EditResponse, Show as _, Validity};
use craft_ui::display::Text;
use craft_ui::testing::{Frames, trial};

use std::sync::Arc;

const LABEL: &str = "Gain";
const BEFORE: f64 = 1.0;
const AFTER: f64 = 2.0;

#[trial]
fn a_lifted_response_carries_the_event_and_the_shown_value_with_clear_flags() {
    let mut frames = Frames::new();
    let mut made = None;
    frames.frame_shapes(|ui| {
        let response = Text::new(LABEL).show(ui);
        let event = EditEvent::commit(GestureId::default(), Mixed::Same(BEFORE), AFTER);
        made = Some(EditResponse::lifted(response, Some(event), Some(Mixed::Same(Arc::new(AFTER)))));
    });
    let Some(edit) = made else { panic!("the frame ran") };
    assert_eq!(edit.value(), Some(&AFTER), "the event's value is the response's value");
    assert!(matches!(edit.shown, Some(Mixed::Same(ref value)) if **value == AFTER), "the shown value is carried");
    assert!(!edit.submitted && !edit.escaped && !edit.unparsed, "the flags are clear");
    assert!(matches!(edit.validity, Validity::Ok), "the validity is Ok");
}
