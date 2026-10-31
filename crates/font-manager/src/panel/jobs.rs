//! Installs, removals and preview downloads, run through the Host's jobs, one at a time.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use craft_host::Host;

use crate::Catalog;
use crate::index::Index;
use crate::store::{Downloaded, Installer, Progress, RemoveRequest, StoreError};

type Task = Box<dyn FnOnce() + Send>;

/// Runs `task` as a job of `host`. Without a Host it runs on the Host builder's default: a thread
/// per job natively, right away on wasm32.
pub fn spawn(host: Option<&Host>, name: &str, task: Task) {
    match host {
        Some(host) => host.jobs().spawn(name, task),
        #[cfg(not(target_arch = "wasm32"))]
        None => craft_host::Jobs::spawn(&craft_host::ThreadJobs, name, task),
        #[cfg(target_arch = "wasm32")]
        None => craft_host::Jobs::spawn(&craft_host::InlineJobs, name, task),
    }
}

/// The jobs waiting, and whether a runner is draining them.
#[derive(Default)]
struct Queue {
    tasks: VecDeque<Task>,
    draining: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Install,
    Remove,
    Preview,
}

pub struct Running {
    /// The family's source; empty for a removal, which goes by the installed files.
    pub source: String,
    pub family: String,
    pub kind: Kind,
    /// One file's index path or store path, when the job is for one style.
    pub file: Option<String>,
    pub progress: Progress,
}

impl Running {
    /// How far along, when the size is known.
    pub fn fraction(&self) -> Option<f32> {
        (self.progress.total > 0).then(|| (self.progress.done as f32 / self.progress.total as f32).clamp(0.0, 1.0))
    }
}

pub enum Finished {
    Installed,
    Removed,
    Previewed(Vec<Downloaded>),
}

enum Msg {
    Progress(u64, Progress),
    Done(u64, Result<Finished, StoreError>),
}

pub enum Request {
    /// A family of the index, by position.
    Install(Arc<Index>, usize),
    /// One file of a family of the index, by its path in the index.
    InstallFile(Arc<Index>, usize, String),
    Remove(RemoveRequest),
    /// A family of the index, by position.
    Preview(Arc<Index>, usize),
}

pub struct Jobs {
    /// Jobs run one at a time in request order: an install and a removal of one family, and the
    /// catalog rescans after them, never overlap.
    queue: Arc<Mutex<Queue>>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    next: u64,
    running: BTreeMap<u64, Running>,
}

impl Jobs {
    pub fn new() -> Jobs {
        let (tx, rx) = channel();
        Jobs { queue: Arc::default(), tx, rx, next: 0, running: BTreeMap::new() }
    }

    /// The job running for `family` of `source` (any source when empty), if any.
    pub fn of(&self, source: &str, family: &str) -> Option<&Running> {
        self.running.values().find(|r| r.family.eq_ignore_ascii_case(family) && (source.is_empty() || r.source.is_empty() || r.source == source))
    }

    /// Runs `request` through `installer`, then rescans `catalog` after an install or removal.
    pub fn start(&mut self, ctx: &egui::Context, host: Option<&Host>, installer: Arc<dyn Installer>, catalog: Arc<Catalog>, request: Request) {
        let id = self.next;
        self.next += 1;
        let (source, family, kind, file, total) = match &request {
            Request::Install(index, i) | Request::Preview(index, i) => {
                let Some(f) = index.families().get(*i) else { return };
                let kind = if matches!(request, Request::Install(..)) { Kind::Install } else { Kind::Preview };
                (f.source.clone(), f.family.clone(), kind, None, f.size())
            }
            Request::InstallFile(index, i, path) => {
                let Some(f) = index.families().get(*i) else { return };
                let size = f.files.iter().find(|x| &x.path == path).map_or(0, |x| x.size);
                (f.source.clone(), f.family.clone(), Kind::Install, Some(path.clone()), size)
            }
            Request::Remove(r) => {
                let file = if r.whole { None } else { r.files.first().map(|p| p.display().to_string()) };
                (String::new(), r.family.clone(), Kind::Remove, file, 0)
            }
        };
        self.running.insert(id, Running { source, family, kind, file, progress: Progress { done: 0, total } });
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let task: Task = Box::new(move || {
            let mut progress = |p: Progress| {
                let _ = tx.send(Msg::Progress(id, p));
                ctx.request_repaint();
            };
            let family = |index: &Index, i: usize| index.families().get(i).cloned().ok_or_else(|| StoreError("no such family".into()));
            let result = match request {
                Request::Install(index, i) => {
                    family(&index, i).and_then(|f| installer.install(&index, &f, &mut progress)).map(|()| Finished::Installed)
                }
                Request::InstallFile(index, i, path) => {
                    family(&index, i).and_then(|f| installer.install_file(&index, &f, &path, &mut progress)).map(|()| Finished::Installed)
                }
                Request::Remove(r) => installer.remove(&r).map(|()| Finished::Removed),
                Request::Preview(index, i) => family(&index, i).and_then(|f| installer.download(&index, &f, &mut progress)).map(Finished::Previewed),
            };
            if matches!(result, Ok(Finished::Installed | Finished::Removed)) {
                catalog.rescan();
            }
            let _ = tx.send(Msg::Done(id, result));
            ctx.request_repaint();
        });
        let start_runner = {
            let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            queue.tasks.push_back(task);
            !std::mem::replace(&mut queue.draining, true)
        };
        if start_runner {
            let queue = Arc::clone(&self.queue);
            spawn(
                host,
                "font-jobs",
                Box::new(move || {
                    loop {
                        let next = {
                            let mut queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
                            let next = queue.tasks.pop_front();
                            queue.draining = next.is_some();
                            next
                        };
                        match next {
                            Some(task) => task(),
                            None => break,
                        }
                    }
                }),
            );
        }
    }

    /// Takes the progress reports, and returns the jobs that ended.
    pub fn poll(&mut self) -> Vec<(Running, Result<Finished, StoreError>)> {
        let mut done = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Progress(id, p) => {
                    if let Some(job) = self.running.get_mut(&id) {
                        job.progress = p;
                    }
                }
                Msg::Done(id, result) => {
                    if let Some(job) = self.running.remove(&id) {
                        done.push((job, result));
                    }
                }
            }
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::FontEnv;
    use crate::store::{Downloaded, RemoveRequest};

    /// Logs when each call starts and ends.
    struct Logging(Arc<Mutex<Vec<&'static str>>>);

    impl Logging {
        fn call(&self, start: &'static str, end: &'static str) -> Result<(), StoreError> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(start);
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(end);
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
        fn install(&self, _: &Index, _: &crate::index::Family, _: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
            self.call("install start", "install end")
        }
        fn install_file(&self, _: &Index, _: &crate::index::Family, _: &str, _: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
            Ok(())
        }
        fn remove(&self, _: &RemoveRequest) -> Result<(), StoreError> {
            self.call("remove start", "remove end")
        }
        fn download(&self, _: &Index, _: &crate::index::Family, _: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
            Ok(Vec::new())
        }
    }

    /// Keeps the spawned jobs until the test runs them.
    #[derive(Default)]
    struct Deferred(Mutex<Vec<Task>>);

    impl craft_host::Jobs for Deferred {
        fn spawn(&self, _: &str, job: Task) {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(job);
        }
    }

    #[test]
    fn jobs_run_one_at_a_time_in_request_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let installer: Arc<dyn Installer> = Arc::new(Logging(Arc::clone(&log)));
        let catalog = Arc::new(Catalog::new(FontEnv { os: crate::craft_font_catalog::Os::None, ..FontEnv::default() }).with_font_service(false));
        let deferred = Arc::new(Deferred::default());
        let host = Host::builder(craft_host::AppInfo { id: "jobs-test", name: "Jobs test", version: "1.0" }).jobs(deferred.clone()).build();
        let index = Arc::new(crate::index::bundled().unwrap());
        let remove = RemoveRequest { family: "Hack".into(), files: Vec::new(), whole: true };
        let ctx = egui::Context::default();
        let mut jobs = Jobs::new();
        for request in [Request::Install(Arc::clone(&index), 0), Request::Remove(remove)] {
            jobs.start(&ctx, Some(&host), Arc::clone(&installer), Arc::clone(&catalog), request);
        }

        let mut spawned = std::mem::take(&mut *deferred.0.lock().unwrap_or_else(PoisonError::into_inner));
        assert_eq!(spawned.len(), 1, "one runner drains the queue");
        assert!(log.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
        spawned.remove(0)();
        assert_eq!(*log.lock().unwrap_or_else(PoisonError::into_inner), ["install start", "install end", "remove start", "remove end"]);

        // The queue is idle again: the next job gets a new runner.
        jobs.start(&ctx, Some(&host), installer, catalog, Request::Install(index, 0));
        assert_eq!(deferred.0.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
        assert!(jobs.poll().len() >= 2);
    }
}
