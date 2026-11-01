//! `PlacesTree` through egui_kittest with a fake `Directories` and `ManualJobs` (task B7).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `current` deep in Home: the rows open once on the way down; collapsing one keeps it closed on the next passes.
#[test]
#[ignore = "skeleton"]
fn the_tree_reveals_the_browsed_folder_once_and_collapse_sticks() {
    todo!()
}

// A ListDrag over an accepted row outlines it and the release reports Dropped; a refused row shows nothing.
#[test]
#[ignore = "skeleton"]
fn a_drop_on_an_accepted_row_reports_it() {
    todo!()
}

// The hidden row with one hidden root restores it at once; with two it opens a menu with Restore All.
#[test]
#[ignore = "skeleton"]
fn hidden_places_come_back_through_their_row() {
    todo!()
}

// Window focus regained: the cache's listings go stale and open folders are listed again.
#[test]
#[ignore = "skeleton"]
fn regaining_focus_lists_open_folders_again() {
    todo!()
}
