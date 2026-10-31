//! `PathField` in a harness with a `Files` stub installed in the Host.

// Typing a path and pressing Enter commits it; the Browse button follows the field in Tab order.
#[test]
#[ignore = "skeleton"]
fn typed_path_commits_on_enter_and_browse_is_next_in_tab_order() {
    todo!()
}

// Browse queues the field's request; the answer's path fills the field on the next pass and
// commits once. An answer with another ticket is returned to the service.
#[test]
#[ignore = "skeleton"]
fn browse_answer_fills_the_field_and_other_tickets_are_returned() {
    todo!()
}

// With `must_exist`, a missing path shows the not-found message under the field.
#[test]
#[ignore = "skeleton"]
fn missing_path_shows_not_found_when_required() {
    todo!()
}
