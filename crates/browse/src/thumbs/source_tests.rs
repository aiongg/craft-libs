//! `FileThumbSource` through `Thumbnails` with `ManualJobs` and a fake decoder (task XT).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Two keys asked VISIBLE and NEAR: ManualJobs runs the VISIBLE decode first.
#[test]
#[ignore = "skeleton"]
fn file_source_jobs_run_visible_before_near() {
    todo!()
}

// After `invalidate(path)` the state is Loading with the old picture as stand-in, then Ready with the new.
#[test]
#[ignore = "skeleton"]
fn file_source_reloads_after_invalidate() {
    todo!()
}

// A failing decode: Failed, and no second job on later passes until `invalidate`.
#[test]
#[ignore = "skeleton"]
fn file_source_fails_once_per_stamp() {
    todo!()
}

// A decoder with `quick`: the quick picture shows, then stands in while `decode` runs, then the full one.
#[test]
#[ignore = "skeleton"]
fn file_source_shows_the_quick_preview_first() {
    todo!()
}

// A store hit is final: `decode` is never called and no second revision is asked.
#[test]
#[ignore = "skeleton"]
fn a_stored_thumbnail_needs_no_decode() {
    todo!()
}
