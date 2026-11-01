//! `Places::resolve`, `PlacesSettings`, `SystemDirectories` (temp dir) and `FolderCache` over a
//! fake `Directories` and `ManualJobs`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Keep, remove (a favourite drops, a standard root hides), restore and restore all; recents keep 10, newest first.
#[test]
#[ignore = "skeleton"]
fn places_keep_hide_and_restore_roots() {
    todo!()
}

// LightCraft's `browsing_below_an_unlisted_folder_has_a_session_root` and `session_root_stays_while_browsing_below_it`.
#[test]
#[ignore = "skeleton"]
fn a_browsed_folder_outside_every_root_gets_a_session_root() {
    todo!()
}

// Home and a kept folder inside it both hold the browsed folder: the kept one owns the reveal.
#[test]
#[ignore = "skeleton"]
fn the_innermost_root_owns_the_reveal() {
    todo!()
}

// LightCraft's `hidden_folder_is_not_revealed_in_an_outer_tree`.
#[test]
#[ignore = "skeleton"]
fn a_hidden_folder_never_opens_a_tree_through_itself() {
    todo!()
}

// LightCraft's `equivalent_paths_are_one_local_location`: `/a/b/`, `/a/./b` are one root.
#[test]
#[ignore = "skeleton"]
fn equal_folders_are_one_root_however_spelled() {
    todo!()
}

// A temp dir with `.git`, `b10`, `b9`, a file: `subfolders` gives `b9`, `b10`.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn folder_listing_skips_hidden_entries_and_sorts_naturally() {
    todo!()
}

// After `refresh`, a folder asked again keeps its old listing until the new job lands, then shows the new one.
#[test]
#[ignore = "skeleton"]
fn a_folder_is_listed_again_after_a_refresh() {
    todo!()
}

// A Directories that blocks: `subfolders` answers Loading at once; the listing arrives after ManualJobs runs.
#[test]
#[ignore = "skeleton"]
fn listing_never_blocks_the_pass() {
    todo!()
}
