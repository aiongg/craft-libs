//! `JobQueue` and `ManualJobs`: order, replacement, one job per key, cancellation, wakes and
//! inline runs. Every test drives lanes with `ManualJobs` (or `lanes: 0`), so nothing depends on
//! thread timing.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// With one lane over ManualJobs: Visible before Near before Background; newest first among equals.
#[test]
#[ignore = "skeleton"]
fn queue_runs_highest_priority_first() {
    todo!()
}

// A second submit for a waiting key runs only the second job; the first never runs.
#[test]
#[ignore = "skeleton"]
fn queue_replaces_queued_job_of_same_key() {
    todo!()
}

// With two lanes: a key's second job waits until its first ends, while another key's job runs.
#[test]
#[ignore = "skeleton"]
fn queue_runs_one_job_per_key() {
    todo!()
}

// `reprioritize` returning None drops waiting jobs and sets Cancel on running ones; neither result arrives.
#[test]
#[ignore = "skeleton"]
fn queue_dropped_keys_are_cancelled() {
    todo!()
}

// The wake is called once per finished job, never for a dropped or cancelled one.
#[test]
#[ignore = "skeleton"]
fn queue_wakes_after_each_finish() {
    todo!()
}

// With `lanes == 0` nothing runs on submit; `run_inline(1)` runs the best job and returns 1.
#[test]
#[ignore = "skeleton"]
fn queue_runs_inline_on_demand() {
    todo!()
}

// `shutdown` drops waiting jobs, cancels running ones, and returns true once they have ended.
#[test]
#[ignore = "skeleton"]
fn queue_shutdown_waits_for_running_jobs() {
    todo!()
}

// `ManualJobs::run_next` runs the oldest job only; a job spawned while it runs waits behind the others.
#[test]
#[ignore = "skeleton"]
fn manual_jobs_run_one_job_per_call() {
    todo!()
}
