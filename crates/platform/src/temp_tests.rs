//! `craft_platform::temp`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// TempDir is mode 0700 on Unix, its files 0600, and the folder and its files are gone after drop
// unless kept.
#[test]
#[ignore = "skeleton"]
fn private_dir_is_owner_only_and_removed_on_drop() {
    todo!()
}

// create() refuses a name a symlink already holds and leaves the link's target unchanged.
#[test]
#[ignore = "skeleton"]
fn create_refuses_a_planted_symlink() {
    todo!()
}

// TempFiles::discard removes only the folder holding that path; cleanup and drop remove the rest.
#[test]
#[ignore = "skeleton"]
fn temp_files_remove_what_they_own() {
    todo!()
}
