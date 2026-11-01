//! `RelinkDialog` through egui_kittest with a fake `LinkFiles` and `ManualJobs` (task B10).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Link on the first row's candidate selects the second row and clears the candidates.
#[test]
#[ignore = "skeleton"]
fn the_relink_dialog_links_one_row_then_the_next() {
    todo!()
}

// A candidate with a DifferentFile problem: the default button reads Link Anyway.
#[test]
#[ignore = "skeleton"]
fn link_anyway_replaces_link_for_a_failing_candidate() {
    todo!()
}

// Relinking `C:\shoot\a.mov` to `/T7/shoot/a.mov` links the other row under `C:\shoot` that passes; a failing one stays missing.
#[test]
#[ignore = "skeleton"]
fn relink_others_offers_the_rows_the_remap_fits() {
    todo!()
}

// Done returns linked and offline rows; Cancel and Esc return Cancel.
#[test]
#[ignore = "skeleton"]
fn done_returns_every_link_and_cancel_returns_nothing() {
    todo!()
}

// While the search job waits in ManualJobs, the dialog draws and Stop cancels it.
#[test]
#[ignore = "skeleton"]
fn the_search_never_blocks_and_stop_cancels_it() {
    todo!()
}
