//! `craft_platform::temp`.
#![allow(clippy::unwrap_used, reason = "tests fail by panicking")]

use crate::temp::*;

// TempDir is mode 0700 on Unix, its files 0600, and the folder and its files are gone after drop
// unless kept.
#[test]
fn private_dir_is_owner_only_and_removed_on_drop() {
    let dir = TempDir::new("craft-temp-test").unwrap();
    let path = dir.path().to_path_buf();
    assert!(is_private_dir(&std::fs::symlink_metadata(&path).unwrap()));
    let (file_path, _file) = dir.create("a.txt").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&file_path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    assert!(dir.create("a.txt").is_err(), "an existing name is refused");
    assert!(dir.create("../b.txt").is_err(), "a name with a separator is refused");
    drop(dir);
    assert!(!path.exists());

    let kept = TempDir::new("craft-temp-test").unwrap().keep();
    assert!(kept.is_dir());
    std::fs::remove_dir_all(kept).unwrap();
}

// create() refuses a name a symlink already holds and leaves the link's target unchanged.
#[cfg(unix)]
#[test]
fn create_refuses_a_planted_symlink() {
    let dir = TempDir::new("craft-temp-test").unwrap();
    let other = TempDir::new("craft-temp-test").unwrap();
    let (target, _file) = other.create("target.txt").unwrap();
    std::fs::write(&target, "original").unwrap();
    std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();
    assert!(dir.create("link.txt").is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
}

// TempFiles::discard removes only the folder holding that path; cleanup and drop remove the rest.
#[test]
fn temp_files_remove_what_they_own() {
    let mut files = TempFiles::default();
    let a = files.create("craft-temp-test", "a.pdf").unwrap();
    let b = files.create("craft-temp-test", "b.pdf").unwrap();
    let c = files.create("craft-temp-test", "c.pdf").unwrap();
    for path in [&a, &b, &c] {
        assert!(path.parent().unwrap().is_dir());
        assert!(!path.exists(), "the file itself is not created");
    }
    let foreign = TempDir::new("craft-temp-test").unwrap();
    files.discard(&foreign.path().join("x"));
    assert!(foreign.path().is_dir(), "a folder it doesn't own stays");
    files.discard(&a);
    assert!(!a.parent().unwrap().exists());
    assert!(b.parent().unwrap().is_dir());
    files.cleanup();
    assert!(!b.parent().unwrap().exists() && !c.parent().unwrap().exists());

    let d = files.create("craft-temp-test", "d.pdf").unwrap();
    drop(files);
    assert!(!d.parent().unwrap().exists());
}
