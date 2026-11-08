//! The panel's job queue and sample sentences.

use std::sync::{Arc, Mutex, PoisonError};

use craft_font_manager::craft_font_catalog::Os;
use craft_font_manager::index::{self, Family, Index};
use craft_font_manager::panel_parts::jobs::{Jobs, Request};
use craft_font_manager::panel_parts::samples::{PANGRAM, for_language, for_scripts};
use craft_font_manager::store::{Downloaded, Installer, Progress, RemoveRequest, StoreError};
use craft_font_manager::{Catalog, FontEnv};
use craft_host::{AppInfo, Host};
use craft_ui::testing::trial;

/// Logs when each call starts and ends.
struct Logging(Arc<Mutex<Vec<&'static str>>>);

impl Logging {
    fn call(&self, start: &'static str, end: &'static str) -> Result<(), StoreError> {
        let mut log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        log.push(start);
        log.push(end);
        Ok(())
    }
}

impl Installer for Logging {
    fn downloads_allowed(&self) -> bool {
        true
    }
    fn allow_downloads(&self, _: bool) -> Result<(), StoreError> {
        Ok(())
    }
    fn install(&self, _: &Index, _: &Family, _: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
        self.call("install start", "install end")
    }
    fn install_file(&self, _: &Index, _: &Family, _: &str, _: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
        Ok(())
    }
    fn remove(&self, _: &RemoveRequest) -> Result<(), StoreError> {
        self.call("remove start", "remove end")
    }
    fn download(&self, _: &Index, _: &Family, _: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
        Ok(Vec::new())
    }
}

type Task = Box<dyn FnOnce() + Send>;

/// Keeps the spawned jobs until the test runs them.
#[derive(Default)]
struct Deferred(Mutex<Vec<Task>>);

impl craft_host::Jobs for Deferred {
    fn spawn(&self, _: &str, job: Task) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push(job);
    }
}

#[trial]
fn jobs_run_one_at_a_time_in_request_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let installer: Arc<dyn Installer> = Arc::new(Logging(Arc::clone(&log)));
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    let deferred = Arc::new(Deferred::default());
    let host = Arc::new(Host::builder(AppInfo { id: "jobs-test", name: "Jobs test", version: "1.0" }).jobs(deferred.clone()).build());
    let index = Arc::new(index::bundled().expect("the bundled index"));
    let remove = RemoveRequest { family: "Hack".into(), files: Vec::new(), whole: true };
    let mut jobs = Jobs::new();
    for request in [Request::Install(Arc::clone(&index), 0), Request::Remove(remove)] {
        jobs.start(Some(&host), Arc::clone(&installer), Arc::clone(&catalog), request);
    }

    let mut spawned = std::mem::take(&mut *deferred.0.lock().unwrap_or_else(PoisonError::into_inner));
    assert_eq!(spawned.len(), 1, "one runner drains the queue");
    assert!(log.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
    spawned.remove(0)();
    assert_eq!(*log.lock().unwrap_or_else(PoisonError::into_inner), ["install start", "install end", "remove start", "remove end"]);

    // The queue is idle again: the next job gets a new runner.
    jobs.start(Some(&host), installer, catalog, Request::Install(index, 0));
    assert_eq!(deferred.0.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
    assert!(jobs.poll().len() >= 2);
}

#[trial]
fn samples_follow_language_then_script() {
    assert_eq!(for_language("ja_Jpan"), for_language("ja"));
    assert_eq!(for_language("zh_Hant"), for_language("zh_Hant_TW"));
    assert_ne!(for_language("zh_Hant"), for_language("zh_Hans"));
    assert_eq!(for_language("xx_Arab"), for_language("ar"));
    assert_eq!(for_language("xx_Zzzz"), None);
    assert_eq!(for_scripts(["Arab", "Latn"]), PANGRAM);
    assert_eq!(for_scripts(["Zzzz", "Hebr"]), for_language("he").unwrap_or_default());
}
