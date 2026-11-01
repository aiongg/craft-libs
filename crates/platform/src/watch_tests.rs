//! `craft_platform::watch`, inline (no worker thread).
#![allow(clippy::unwrap_used, reason = "tests fail by panicking")]

use std::path::PathBuf;
use std::sync::Arc;

use craft_host::Wake;

use crate::temp::TempDir;
use crate::watch::*;

fn watcher() -> FileWatch<u32> {
    FileWatch::new(Wake::none()).inline()
}

/// Two looks: the first sees the change, the second sees it unchanged and reports it.
fn settle(watch: &mut FileWatch<u32>) {
    watch.tick();
    watch.tick();
}

fn file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

// A file rewritten by another writer is reported once it has stayed the same for SETTLE; the
// version recorded with known_now after the app's own save is never reported.
#[test]
fn another_writer_is_a_change_and_an_own_save_is_not() {
    let dir = TempDir::new("craft-watch-test").unwrap();
    let path = file(&dir, "doc.txt", "one");
    let mut watch = watcher();
    watch.known_now(1, &path);
    watch.set([(1, path.clone())]);
    settle(&mut watch);
    assert!(watch.take_changes().is_empty());

    std::fs::write(&path, "two!").unwrap();
    watch.tick();
    assert!(watch.take_changes().is_empty(), "reported only after it stays the same");
    watch.tick();
    let changes = watch.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!((changes[0].key, changes[0].bytes.clone()), (1, None));
    assert_eq!(changes[0].stamp.len(), 4);

    // The app's own save.
    std::fs::write(&path, "three").unwrap();
    watch.known_now(1, &path);
    watch.set([(1, path)]);
    settle(&mut watch);
    assert!(watch.take_changes().is_empty());
}

// A change found before known_now bumped the epoch, or for a path the key no longer has, is
// dropped by take_changes.
#[test]
fn stale_changes_are_dropped() {
    let dir = TempDir::new("craft-temp-test").unwrap();
    let path = file(&dir, "a.txt", "one");
    let other = file(&dir, "b.txt", "one");

    let mut watch = watcher();
    watch.known_now(1, &path);
    watch.set([(1, path.clone())]);
    std::fs::write(&path, "two!").unwrap();
    settle(&mut watch);
    std::fs::write(&path, "three").unwrap();
    watch.known_now(1, &path);
    assert!(watch.take_changes().is_empty(), "found for an earlier reading of the file");

    let mut watch = watcher();
    watch.known_now(1, &path);
    watch.set([(1, path.clone())]);
    std::fs::write(&path, "four!!").unwrap();
    settle(&mut watch);
    watch.set([(1, other)]);
    assert!(watch.take_changes().is_empty(), "the key has another file now");
    watch.forget(&1);
}

// ReadPolicy::Whole reads only once `complete` accepts the tail; a file still being written
// costs no whole read.
#[test]
fn whole_policy_waits_for_a_complete_file() {
    let dir = TempDir::new("craft-watch-test").unwrap();
    let path = file(&dir, "doc.pdf", "old\n%%EOF");
    let mut watch =
        FileWatch::new(Wake::none()).read(ReadPolicy::Whole { tail: 16, complete: Arc::new(|tail| tail.windows(5).any(|w| w == b"%%EOF")) }).inline();
    watch.known_now(1, &path);
    watch.set([(1, path.clone())]);

    std::fs::write(&path, "new, half writ").unwrap();
    settle(&mut watch);
    assert!(watch.take_changes().is_empty());

    std::fs::write(&path, "new, whole\n%%EOF").unwrap();
    settle(&mut watch);
    let changes = watch.take_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].bytes.as_deref().map(Vec::as_slice), Some(b"new, whole\n%%EOF".as_slice()));
}

// `modified` is the modification time the metadata reports, and moves with a rewrite.
#[test]
fn a_stamp_reports_the_modification_time() {
    let dir = TempDir::new("craft-watch-test").unwrap();
    let path = file(&dir, "doc.txt", "content");
    let stamp = FileStamp::at(&path).unwrap();
    assert_eq!(stamp.modified(), std::fs::metadata(&path).unwrap().modified().ok());
    let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(past).unwrap();
    assert_eq!(FileStamp::at(&path).unwrap().modified(), Some(past));
}

// disk_differs: false for a missing file, an unknown key, or a change-time-only difference with
// equal saved bytes; true after another write.
#[test]
fn disk_differs_ignores_metadata_only_changes() {
    let dir = TempDir::new("craft-watch-test").unwrap();
    let path = file(&dir, "doc.txt", "content");
    let mut watch = FileWatch::<u32>::new(Wake::none());
    assert!(!watch.disk_differs(&1, &path, None), "unknown key");
    watch.known_now(1, &path);
    assert!(!watch.disk_differs(&1, &path, None));
    assert!(!watch.disk_differs(&1, &dir.path().join("missing"), None));

    // A permission change moves the change time only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(watch.disk_differs(&1, &path, None));
        assert!(watch.disk_differs(&1, &path, Some(b"other")));
        assert!(!watch.disk_differs(&1, &path, Some(b"content")));
    }

    std::fs::write(&path, "rewritten").unwrap();
    assert!(watch.disk_differs(&1, &path, Some(b"content")));
}

// Watching nothing keeps the recorded versions: Save still notices another writer. Only forget
// drops a record.
#[test]
fn unwatched_documents_keep_their_records() {
    let dir = TempDir::new("craft-watch-test").unwrap();
    let path = file(&dir, "doc.txt", "one");
    let mut watch = watcher();
    watch.known_now(1, &path);
    watch.set([(1, path.clone())]);
    watch.set([]);
    std::fs::write(&path, "two!").unwrap();
    assert!(watch.disk_differs(&1, &path, None));
    watch.forget(&1);
    assert!(!watch.disk_differs(&1, &path, None));
}
