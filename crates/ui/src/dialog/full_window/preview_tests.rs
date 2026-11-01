//! `PreviewStatus` tickets.

use super::PreviewStatus;

// A result for an older ticket is dropped and leaves the status pending; the latest clears it.
#[test]
fn preview_status_drops_a_result_for_an_older_ticket() {
    let mut status = PreviewStatus::new();
    assert!(!status.pending());
    let first = status.request();
    let second = status.request();
    assert!(first < second);
    assert!(!status.ready(first));
    assert!(status.pending());
    assert!(!status.failed(first, "late"));
    assert_eq!(status.error(), None);
    assert!(status.ready(second));
    assert!(!status.pending());
}

// Pending from a request to its result; a failure of the latest ticket keeps one error until
// the next request; progress shows only for the latest ticket.
#[test]
fn preview_status_reports_pending_and_error_once() {
    let mut status = PreviewStatus::new();
    let old = status.request();
    let ticket = status.request();
    // Pending with no progress is indeterminate; a progress of an older ticket is ignored.
    assert_eq!(status.progress(), Some(None));
    status.report_progress(old, 0.9);
    assert_eq!(status.progress(), Some(None));
    status.report_progress(ticket, 0.25);
    assert_eq!(status.progress(), Some(Some(0.25)));

    assert!(status.failed(ticket, "out of memory"));
    assert_eq!(status.error(), Some("out of memory"));
    assert!(!status.pending());
    assert_eq!(status.progress(), None);

    // The next request clears the error.
    status.request();
    assert_eq!(status.error(), None);
    assert!(status.pending());
}
