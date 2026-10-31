//! `document::merge_changes` and two stores on one file (gap 31).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A key only the other instance changed keeps its value; a key only we changed is written; when
// both changed one key, ours wins.
#[test]
#[ignore = "skeleton"]
fn only_our_changes_are_written() {
    todo!()
}

// A key removed on disk and unchanged here stays removed; a key we removed is removed on disk.
#[test]
#[ignore = "skeleton"]
fn removals_follow_the_changer() {
    todo!()
}

// A disk file at a newer schema migrates our base first, so our migration doesn't overwrite the
// other instance's choice.
#[test]
#[ignore = "skeleton"]
fn stale_migration_is_not_written_over() {
    todo!()
}

// Two SettingsStores on one folder each change a different key and both changes survive (fill
// task S4-F12, which wires the merge into `save`).
#[test]
#[ignore = "skeleton"]
fn two_stores_keep_each_others_changes() {
    todo!()
}

// A store opened with custom SettingsFiles reads and writes only through them.
#[test]
#[ignore = "skeleton"]
fn custom_files_see_every_read_and_write() {
    todo!()
}
