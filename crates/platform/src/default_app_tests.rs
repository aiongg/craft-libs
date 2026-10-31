//! `craft_platform::default_app`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// file:///a%20b.pdf and file://localhost/x decode to paths; http://, file://host/x, plain paths
// and non-UTF-8 escapes pass through; file:///C:/x.pdf is C:/x.pdf on Windows.
#[test]
#[ignore = "skeleton"]
fn file_uris_become_paths() {
    todo!()
}

// ["P:\Docs\01", "Contract.PDF", "other.pdf"] with "P:\Docs\01 Contract.PDF" existing becomes
// two paths; arguments that name files stay as given.
#[test]
#[ignore = "skeleton"]
fn unquoted_paths_with_spaces_rejoin() {
    todo!()
}

// XDG_DATA_HOME and XDG_DATA_DIRS each give an applications folder; defaults apply when unset
// and relative entries are ignored.
#[test]
#[ignore = "skeleton"]
fn application_dirs_follow_the_xdg_spec() {
    todo!()
}
