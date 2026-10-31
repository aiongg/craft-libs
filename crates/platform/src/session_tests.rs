//! Reopen last session.

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// from_json skips entries without a path, defaults a missing view to Null and keeps at most MAX_DOCUMENTS.
fn stored_session_reads_leniently_and_is_capped() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// to_reopen leaves out a deleted file, a path in skip (compared after absolute) and one is_open reports.
fn reopen_skips_missing_skipped_and_open_files() {
    todo!()
}

#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
// After quit_started, closing documents doesn't change to_save(true, ..); quit_cancelled makes to_save return the open session again.
fn quit_keeps_the_session_from_its_start() {
    todo!()
}
