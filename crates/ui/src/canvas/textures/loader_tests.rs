//! `TextureLoader` over a `JobQueue` driven by `ManualJobs` (task XT, after Q1, X1 and X3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A queued job whose key is not asked for in two passes never runs.
#[test]
#[ignore = "skeleton"]
fn loader_drops_jobs_nobody_asked_for() {
    todo!()
}

// With 20 results finished, 16 are uploaded this pass, the rest in the next, and a repaint is requested.
#[test]
#[ignore = "skeleton"]
fn loader_uploads_at_most_budget_per_pass() {
    todo!()
}

// A result for a revision replaced before it finished is not uploaded.
#[test]
#[ignore = "skeleton"]
fn loader_discards_results_of_older_revisions() {
    todo!()
}

// With `lanes: 0` the first call of a pass runs one job inline.
#[test]
#[ignore = "skeleton"]
fn loader_runs_one_job_per_pass_without_lanes() {
    todo!()
}
