//! `LinkMonitor` and `SearchRun` over a fake `LinkFiles` and `ManualJobs`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use craft_docpath::links::{FileInfo, FolderEntry, Link, LinkFiles, LinkIdentity, LinkSearch, LinkStatus, SearchEnd};
use craft_host::{ManualJobs, Wake};

use super::{LinkChange, LinkMonitor, SearchRun};

/// Files by path: size, fingerprint and modification second.
#[derive(Default)]
struct Fake(Mutex<BTreeMap<PathBuf, (u64, u64, u64)>>);

impl Fake {
    fn put(&self, path: &str, size: u64, fingerprint: u64, secs: u64) {
        self.0.lock().unwrap().insert(path.into(), (size, fingerprint, secs));
    }
}

fn stamp(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

impl LinkFiles for Fake {
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        self.0.lock().unwrap().get(path).map(|&(size, _, secs)| FileInfo { size, modified: Some(stamp(secs)) })
    }

    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        let &(size, fingerprint, secs) = self.0.lock().unwrap().get(path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(LinkIdentity { size, fingerprint, modified: Some(stamp(secs)) })
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        let files = self.0.lock().unwrap();
        Ok(files
            .keys()
            .filter(|p| p.parent() == Some(dir))
            .map(|p| FolderEntry { path: p.clone(), name: p.file_name().unwrap().to_string_lossy().into_owned(), is_dir: false, is_link: false })
            .collect())
    }
}

fn link(key: u32, stored: &str) -> Link<u32> {
    Link {
        key,
        name: stored.into(),
        stored: stored.into(),
        identity: Some(LinkIdentity { size: 3, fingerprint: 7, modified: Some(stamp(1)) }),
        page: None,
        embedded: false,
    }
}

fn monitor(files: &Arc<Fake>, jobs: &Arc<ManualJobs>) -> LinkMonitor<u32> {
    LinkMonitor::new(files.clone(), jobs.clone(), Wake::none())
}

// A file changes once: one LinkChange Ok → Modified, and nothing on the following passes.
#[test]
fn the_monitor_reports_a_change_once() {
    let files = Arc::new(Fake::default());
    files.put("/p/a.png", 3, 7, 1);
    let jobs = Arc::new(ManualJobs::new());
    let mut m = monitor(&files, &jobs);
    m.set(&[link(1, "/p/a.png")], None);
    jobs.run_all();
    assert_eq!(m.take_changes(), vec![LinkChange { key: 1, before: None, status: LinkStatus::Ok }]);

    files.put("/p/a.png", 4, 9, 2);
    m.window_focused();
    jobs.run_all();
    assert_eq!(m.take_changes(), vec![LinkChange { key: 1, before: Some(LinkStatus::Ok), status: LinkStatus::Modified }]);
    assert_eq!(m.status(&1), Some(LinkStatus::Modified));

    m.window_focused();
    jobs.run_all();
    assert!(m.take_changes().is_empty());
    assert!(m.take_changes().is_empty());
}

// `window_focused` checks every link again; unchanged links report nothing.
#[test]
fn a_focus_check_reports_only_changed_links() {
    let files = Arc::new(Fake::default());
    files.put("/p/a.png", 3, 7, 1);
    files.put("/p/b.png", 3, 7, 1);
    let jobs = Arc::new(ManualJobs::new());
    let mut m = monitor(&files, &jobs);
    m.set(&[link(1, "/p/a.png"), link(2, "/p/b.png")], None);
    jobs.run_all();
    assert_eq!(m.take_changes().len(), 2);

    files.0.lock().unwrap().remove(Path::new("/p/b.png"));
    m.window_focused();
    jobs.run_all();
    assert_eq!(m.take_changes(), vec![LinkChange { key: 2, before: Some(LinkStatus::Ok), status: LinkStatus::Missing }]);
}

// Cancel during the run: `take_updates` is empty afterwards and `end` is Cancelled.
#[test]
fn a_cancelled_search_run_reports_nothing_more() {
    let files = Arc::new(Fake::default());
    files.put("/r/x.png", 3, 7, 1);
    let jobs = Arc::new(ManualJobs::new());
    let mut run = SearchRun::start(LinkSearch::new("/r"), vec![link(1, "/old/x.png")], files.clone(), jobs.clone(), Wake::none());
    assert_eq!(run.end(), None);
    run.cancel();
    jobs.run_all();
    assert!(run.take_updates().is_empty());
    assert_eq!(run.end(), Some(SearchEnd::Cancelled));

    let mut done = SearchRun::start(LinkSearch::new("/r"), vec![link(1, "/old/x.png")], files, jobs.clone(), Wake::none());
    jobs.run_all();
    assert_eq!(done.take_updates().len(), 1);
    assert_eq!(done.end(), Some(SearchEnd::Done { files: 1 }));
}
