//! `craft_platform::autosave`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Nothing is due before the interval; then only dirty documents with a new revision and no
// autosave in flight; a failure returns "Autosave failed: …".
#[test]
#[ignore = "skeleton"]
fn due_picks_dirty_changed_documents_each_interval() {
    todo!()
}

// Platform mode: LOCALAPPDATA on Windows, an absolute XDG_STATE_HOME or ~/.local/state on Linux,
// the config folder on macOS; self-contained mode always uses the config folder.
#[test]
#[ignore = "skeleton"]
fn recovery_dir_follows_the_platform_rules() {
    todo!()
}

// A copy written by a run that is gone is a leftover; one of a live instance (its lock held) is
// not; discard removes copy and sidecar.
#[test]
#[ignore = "skeleton"]
fn leftovers_skip_live_instances() {
    todo!()
}

// An interrupted legacy move (.moving entries) is finished by the next launch.
#[test]
#[ignore = "skeleton"]
fn legacy_recovery_move_resumes() {
    todo!()
}

// Keys keep letters, digits, '-' and '_'; others become '_'; empty is "untitled".
#[test]
#[ignore = "skeleton"]
fn keys_are_sanitized() {
    todo!()
}
