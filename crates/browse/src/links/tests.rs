//! `LinkMonitor` and `SearchRun` over `FakeLinkFiles` and `ManualJobs`.

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use craft_docpath::links::{Candidate, FakeLinkFiles, FileInfo, FolderEntry, Link, LinkFiles, LinkIdentity, LinkSearch, LinkStatus, SearchEnd};
use craft_host::{ManualJobs, Wake};

use super::{LinkChange, LinkMonitor, SearchRun};

/// A `FakeLinkFiles` the test can replace while a monitor holds it ("the files changed").
struct Disk(Mutex<FakeLinkFiles>);

impl Disk {
    fn new(files: FakeLinkFiles) -> Arc<Self> {
        Arc::new(Self(Mutex::new(files)))
    }

    fn change(&self, files: FakeLinkFiles) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = files;
    }

    fn identity_of(&self, path: &str) -> LinkIdentity {
        self.identity(Path::new(path)).unwrap()
    }
}

impl LinkFiles for Disk {
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).stat(path)
    }

    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).identity(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).list(dir)
    }
}

fn link(key: u32, stored: &str, identity: LinkIdentity) -> Link<u32> {
    Link { key, name: stored.into(), stored: stored.into(), identity: Some(identity), page: None, embedded: false }
}

fn monitor(files: &Arc<Disk>, jobs: &Arc<ManualJobs>) -> LinkMonitor<u32> {
    LinkMonitor::new(files.clone(), jobs.clone(), Wake::none())
}

// A file changes once: one LinkChange Ok -> Modified, and nothing on the following passes; a focus
// check reports only the links that changed.
#[test]
fn the_monitor_reports_a_change_once() {
    let files = Disk::new(FakeLinkFiles::new().with("/p/a.png", b"abc", 1).with("/p/b.png", b"abc", 1));
    let (a, b) = (link(1, "/p/a.png", files.identity_of("/p/a.png")), link(2, "/p/b.png", files.identity_of("/p/b.png")));
    let jobs = Arc::new(ManualJobs::new());
    let mut m = monitor(&files, &jobs);
    m.set(&[a, b], None);
    jobs.run_all();
    assert_eq!(m.take_changes().len(), 2);

    files.change(FakeLinkFiles::new().with("/p/a.png", b"abcd", 2).with("/p/b.png", b"abc", 1));
    m.window_focused();
    jobs.run_all();
    assert_eq!(m.take_changes(), vec![LinkChange { key: 1, before: Some(LinkStatus::Ok), status: LinkStatus::Modified }]);
    assert_eq!(m.status(&1), Some(LinkStatus::Modified));

    m.window_focused();
    jobs.run_all();
    assert!(m.take_changes().is_empty());
    assert!(m.take_changes().is_empty());

    files.change(FakeLinkFiles::new().with("/p/a.png", b"abcd", 2));
    m.window_focused();
    jobs.run_all();
    assert_eq!(m.take_changes(), vec![LinkChange { key: 2, before: Some(LinkStatus::Ok), status: LinkStatus::Missing }]);
}

// A check that finished before the link was updated is stale when it is taken in: it reports nothing, and the newer check decides.
#[test]
fn a_check_older_than_known_now_is_dropped() {
    let files = Disk::new(FakeLinkFiles::new().with("/p/a.png", b"abc", 1));
    let old = link(1, "/p/a.png", files.identity_of("/p/a.png"));
    files.change(FakeLinkFiles::new().with("/p/a.png", b"abcd", 2));
    let updated = link(1, "/p/a.png", files.identity_of("/p/a.png"));
    let jobs = Arc::new(ManualJobs::new());
    let mut m = monitor(&files, &jobs);
    m.set(&[old], None);
    jobs.run_all(); // saw the changed file: Modified, not yet taken in
    m.set(&[updated], None); // the app updated the link to the new file
    m.known_now(&1, LinkStatus::Ok);
    jobs.run_all();
    assert!(m.take_changes().is_empty(), "the stale Modified is not reported");
    assert_eq!(m.status(&1), Some(LinkStatus::Ok));
}

// Cancel during the run: `take_updates` is empty afterwards and `end` is Cancelled.
#[test]
fn a_cancelled_search_run_reports_nothing_more() {
    let files: Arc<dyn LinkFiles> = Arc::new(FakeLinkFiles::new().with("/r/x.png", b"abc", 1));
    let old = link(1, "/old/x.png", LinkIdentity { size: 3, fingerprint: 0, modified: None });
    let jobs = Arc::new(ManualJobs::new());
    let keep: Arc<super::LinkChecker<u32>> = Arc::new(|_: &Link<u32>, candidate: Candidate| candidate);
    let mut run = SearchRun::start(LinkSearch::new("/r"), vec![old.clone()], files.clone(), keep.clone(), jobs.clone(), Wake::none());
    assert_eq!(run.end(), None);
    run.cancel();
    jobs.run_all();
    assert!(run.take_updates().is_empty());
    assert_eq!(run.end(), Some(SearchEnd::Cancelled));

    let mut done = SearchRun::start(LinkSearch::new("/r"), vec![old], files, keep, jobs.clone(), Wake::none());
    jobs.run_all();
    assert_eq!(done.take_updates().len(), 1);
    assert_eq!(done.end(), Some(SearchEnd::Done { files: 1 }));
}
