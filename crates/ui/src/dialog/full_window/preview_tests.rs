//! `PreviewStatus` tickets.

#![expect(clippy::todo, reason = "skeleton: K17 fills these tests")]

// A result for an older ticket is dropped and leaves the status pending; the latest clears it.
#[test]
#[ignore = "skeleton"]
fn preview_status_drops_a_result_for_an_older_ticket() {
    todo!()
}

// Pending from a request to its result; a failure of the latest ticket keeps one error until
// the next request; progress shows only for the latest ticket.
#[test]
#[ignore = "skeleton"]
fn preview_status_reports_pending_and_error_once() {
    todo!()
}
