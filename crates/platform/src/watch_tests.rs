//! `craft_platform::watch`, inline (no worker thread).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A file rewritten by another writer is reported once it has stayed the same for SETTLE; the
// version recorded with known_now after the app's own save is never reported.
#[test]
#[ignore = "skeleton"]
fn another_writer_is_a_change_and_an_own_save_is_not() {
    todo!()
}

// A change found before known_now bumped the epoch, or for a path the key no longer has, is
// dropped by take_changes.
#[test]
#[ignore = "skeleton"]
fn stale_changes_are_dropped() {
    todo!()
}

// ReadPolicy::Whole reads only once `complete` accepts the tail; a file still being written
// costs no whole read.
#[test]
#[ignore = "skeleton"]
fn whole_policy_waits_for_a_complete_file() {
    todo!()
}

// disk_differs: false for a missing file, an unknown key, or a change-time-only difference with
// equal saved bytes; true after another write.
#[test]
#[ignore = "skeleton"]
fn disk_differs_ignores_metadata_only_changes() {
    todo!()
}
