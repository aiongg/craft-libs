//! Behaviour tests of `control` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// An expired queued request is replied `expired_error` and never dispatched; a dispatched one that
// times out gets `timeout_error`.
#[test]
#[ignore = "skeleton"]
fn deadlines_distinguish_expired_from_timed_out() {
    todo!()
}

// AfterInput waiters are answered after input_processed; AfterJob waiters with the job's result.
#[test]
#[ignore = "skeleton"]
fn waiters_reply_when_input_or_job_settles() {
    todo!()
}
