//! [`SuiteWatch`]: the suite folder watched from a worker thread and published as snapshots that
//! a UI checks with one atomic load per frame.

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use craft_theme::{SuiteTheme, ThemeId, builtin};

use crate::{Accepted, FileStamp, LibraryFile, Problem, SelectionChange, Store, StoreError, StoreOptions, SuiteDir, ThemeEntry, ThemeRecord};

/// How often the worker checks `selection.json`'s stamp (length, modification time, file
/// identity) and the stamps of the themes in use.
pub const STAT_INTERVAL: Duration = Duration::from_millis(500);
/// How often the worker re-reads `selection.json` and compares its bytes, which catches a
/// replacement that kept length and modification time.
pub const VERIFY_INTERVAL: Duration = Duration::from_secs(10);
/// How many operation results a snapshot keeps.
pub const RESULTS_KEPT: usize = 16;
/// How long dropping a [`SuiteWatch`] waits for its worker.
#[cfg(not(target_arch = "wasm32"))]
const DROP_WAIT: Duration = Duration::from_millis(100);

/// Work for the worker; [`SuiteWatch::request`] returns its id and the result appears in
/// [`Snapshot::results`].
#[derive(Clone, Debug)]
pub enum Op {
    /// Writes the fields the user changed ([`Store::update`]). `base` is the accepted state the
    /// UI showed when the user made the change.
    Update { base: Accepted, change: SelectionChange },
    /// Adds a theme file to the library ([`Store::import`]); the result is
    /// [`OpOutput::Imported`].
    Import { bytes: Arc<[u8]>, as_copy: bool },
    /// Removes a library theme ([`Store::delete`]).
    Delete(ThemeId),
    /// A theme file for sharing ([`Store::export`]); the result is [`OpOutput::Exported`].
    Export(ThemeId),
    /// Re-lists the library now, for a theme picker that opens. Unchanged files are not read.
    Rescan,
    /// The library theme the app uses for its own choice ("Use a different theme in this app"),
    /// kept parsed in [`Snapshot::app_theme`]; `None` when it uses the suite's or a built-in.
    AppTheme(Option<ThemeId>),
}

/// What a successful [`Op`] returns.
#[derive(Clone, Debug)]
pub enum OpOutput {
    Done,
    Exported(Arc<[u8]>),
    /// The stored id; for an import as copy, the new one.
    Imported(ThemeId),
}

/// The state of the suite folder at one generation. Snapshots never change; a new one replaces
/// the old.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The selection as last read or written. A damaged, too new or unreadable `selection.json`
    /// leaves it in place and shows up in `problems`.
    pub accepted: Accepted,
    /// The library themes, by file name.
    pub index: Arc<[ThemeEntry]>,
    /// The suite selection's theme when it is a library theme: the last version that parsed.
    pub active: Option<ThemeRecord>,
    /// The theme named by [`Op::AppTheme`]: the last version that parsed.
    pub app_theme: Option<ThemeRecord>,
    pub problems: Vec<Problem>,
    /// The latest operation results by request id, oldest first, at most [`RESULTS_KEPT`].
    pub results: Arc<[(u64, Result<OpOutput, StoreError>)]>,
    pub generation: u64,
}

impl Snapshot {
    /// The result of request `id`, once the worker has run it.
    pub fn result(&self, id: u64) -> Option<&Result<OpOutput, StoreError>> {
        self.results.iter().rev().find(|(r, _)| *r == id).map(|(_, result)| result)
    }

    pub fn last_result(&self) -> Option<&(u64, Result<OpOutput, StoreError>)> {
        self.results.last()
    }

    /// A built-in theme, the active theme or the app's own library theme, for
    /// [`craft_theme::resolve`]'s `lookup`. Other library themes give `None` until the app
    /// names them with [`Op::AppTheme`].
    pub fn theme(&self, id: &ThemeId) -> Option<SuiteTheme> {
        if let Some(theme) = builtin(id) {
            return Some(theme.clone());
        }
        [&self.active, &self.app_theme].into_iter().flatten().find(|r| r.theme.id == *id).map(|r| r.theme.clone())
    }
}

/// Watches the suite folder and runs store operations off the UI thread.
///
/// The worker thread ("craft-theme-watch") checks `selection.json`'s stamp every
/// [`STAT_INTERVAL`] and parses it only when it changed, compares its bytes every
/// [`VERIFY_INTERVAL`], re-lists the library only when `library_revision` changes or on
/// [`Op::Rescan`] (parsing only changed files), and re-reads the themes in use only when their
/// stamps change. Each change publishes a new [`Snapshot`], bumps [`generation`](Self::generation)
/// and calls `wake` once; an unchanged poll does neither.
///
/// Without a folder (and on `wasm32`) there is no thread: operations run inside
/// [`request`](Self::request), with the same snapshots, generations and wake calls.
///
/// Dropping stops the worker and waits at most 100 ms for it: operations already requested
/// finish first. A worker still blocked on a file or the lock then is detached, and ends when
/// its operation returns. `wake` runs on the worker (inside `request` without one) and must
/// not block or call back into the watch.
pub struct SuiteWatch {
    shared: Arc<Shared>,
    next_request: AtomicU64,
    #[cfg(not(target_arch = "wasm32"))]
    worker: Option<Worker>,
}

impl SuiteWatch {
    /// Watches `dir` with default store options. Returns at once; the first snapshot with the
    /// folder's contents follows from the worker.
    pub fn start(dir: SuiteDir, wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self::open(&dir, StoreOptions::new(concat!("craft-theme-store ", env!("CARGO_PKG_VERSION"))), wake)
    }

    /// Watches `dir` with the app's store options: `written_by` and the last accepted state from
    /// its preferences, which the first snapshot shows until the folder has been read.
    pub fn open(dir: &SuiteDir, options: StoreOptions, wake: impl Fn() + Send + Sync + 'static) -> Self {
        let store = Store::open(dir, options);
        #[cfg(not(target_arch = "wasm32"))]
        if !dir.is_memory() {
            return Self::spawn(store, Box::new(wake), Intervals::default());
        }
        Self::inline(store, Box::new(wake))
    }

    /// An in-memory suite, for app constructors, tests, web builds and portable installs.
    pub fn memory() -> Self {
        Self::open(&SuiteDir::memory(), StoreOptions::new(concat!("craft-theme-store ", env!("CARGO_PKG_VERSION"))), || {})
    }

    /// Goes up by one with every new snapshot: the only call an app makes each frame.
    pub fn generation(&self) -> u64 {
        self.shared.generation.load(Ordering::Acquire)
    }

    /// The current snapshot; take it when [`generation`](Self::generation) changed.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        Arc::clone(&lock(&self.shared.snapshot))
    }

    /// Queues `op` and returns its id; never blocks on the disk while a worker runs. The result
    /// appears in a later snapshot's [`results`](Snapshot::results).
    pub fn request(&self, op: Op) -> u64 {
        let id = self.next_request.fetch_add(1, Ordering::Relaxed) + 1;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(worker) = &self.worker {
            // A worker that is gone has nothing left to report; the request is dropped with it.
            let _ = worker.ops.send((id, op));
            return id;
        }
        let mut engine = lock(&self.shared.engine);
        engine.apply(id, op);
        self.shared.publish(&mut engine);
        id
    }

    fn build(store: Store, wake: Wake) -> (Arc<Shared>, Self) {
        let first = Snapshot {
            accepted: store.accepted().clone(),
            index: Arc::from([]),
            active: None,
            app_theme: None,
            problems: Vec::new(),
            results: Arc::from([]),
            generation: 0,
        };
        let shared = Arc::new(Shared {
            engine: Mutex::new(Engine::new(store)),
            snapshot: Mutex::new(Arc::new(first)),
            generation: AtomicU64::new(0),
            wake,
            stopped: AtomicBool::new(false),
        });
        let watch = SuiteWatch {
            shared: Arc::clone(&shared),
            next_request: AtomicU64::new(0),
            #[cfg(not(target_arch = "wasm32"))]
            worker: None,
        };
        (shared, watch)
    }

    /// No thread: the first load and every operation run on the caller's thread.
    pub(crate) fn inline(store: Store, wake: Wake) -> Self {
        let (shared, watch) = Self::build(store, wake);
        let mut engine = lock(&shared.engine);
        engine.poll(true);
        shared.publish(&mut engine);
        drop(engine);
        watch
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn spawn(store: Store, wake: Wake, intervals: Intervals) -> Self {
        let (shared, mut watch) = Self::build(store, wake);
        let (ops, queue) = std::sync::mpsc::channel();
        let (done, finished) = std::sync::mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let spawned = std::thread::Builder::new().name("craft-theme-watch".into()).spawn(move || {
            run(&worker_shared, &queue, intervals);
            let _ = done.send(());
        });
        match spawned {
            Ok(thread) => watch.worker = Some(Worker { ops, thread, finished: Mutex::new(finished) }),
            // No thread to be had: work on the caller's thread rather than not at all.
            Err(_) => {
                let mut engine = lock(&shared.engine);
                engine.poll(true);
                shared.publish(&mut engine);
            }
        }
        watch
    }

    /// Runs one poll on the caller's thread, as the worker does on its timer.
    #[cfg(test)]
    pub(crate) fn poll_once(&self, verify: bool) {
        let mut engine = lock(&self.shared.engine);
        engine.poll(verify);
        self.shared.publish(&mut engine);
    }

    #[cfg(test)]
    pub(crate) fn parse_counts(&self) -> crate::ParseCounts {
        lock(&self.shared.engine).store.parse_counts()
    }
}

impl Drop for SuiteWatch {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(worker) = self.worker.take() {
            let Worker { ops, thread, finished } = worker;
            // Closing the queue ends the worker once the requests already in it are done.
            drop(ops);
            let _ = finished.into_inner().unwrap_or_else(PoisonError::into_inner).recv_timeout(DROP_WAIT);
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
    }
}

pub(crate) type Wake = Box<dyn Fn() + Send + Sync>;

#[cfg(not(target_arch = "wasm32"))]
struct Worker {
    ops: std::sync::mpsc::Sender<(u64, Op)>,
    thread: std::thread::JoinHandle<()>,
    // In a mutex only so that `SuiteWatch` is `Sync`.
    finished: Mutex<std::sync::mpsc::Receiver<()>>,
}

/// The worker's timers; tests shorten them.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Intervals {
    pub stat: Duration,
    pub verify: Duration,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Intervals {
    fn default() -> Self {
        Intervals { stat: STAT_INTERVAL, verify: VERIFY_INTERVAL }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run(shared: &Shared, queue: &std::sync::mpsc::Receiver<(u64, Op)>, intervals: Intervals) {
    use std::{sync::mpsc::RecvTimeoutError, time::Instant};

    {
        let mut engine = lock(&shared.engine);
        engine.poll(true);
        shared.publish(&mut engine);
    }
    let start = Instant::now();
    let mut next_stat = start + intervals.stat;
    let mut next_verify = start + intervals.verify;
    loop {
        let wait = next_stat.saturating_duration_since(Instant::now());
        match queue.recv_timeout(wait) {
            Ok((id, op)) => {
                let mut engine = lock(&shared.engine);
                engine.apply(id, op);
                shared.publish(&mut engine);
            }
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let verify = now >= next_verify;
                if verify {
                    next_verify = now + intervals.verify;
                }
                next_stat = now + intervals.stat;
                let mut engine = lock(&shared.engine);
                engine.poll(verify);
                shared.publish(&mut engine);
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

struct Shared {
    engine: Mutex<Engine>,
    snapshot: Mutex<Arc<Snapshot>>,
    generation: AtomicU64,
    wake: Wake,
    stopped: AtomicBool,
}

impl Shared {
    /// Publishes a new snapshot when something changed. Called with the engine locked, so
    /// generations never race.
    fn publish(&self, engine: &mut Engine) {
        if !std::mem::take(&mut engine.changed) {
            return;
        }
        let generation = self.generation.load(Ordering::Acquire).saturating_add(1);
        *lock(&self.snapshot) = Arc::new(engine.snapshot(generation));
        // After the snapshot, so a reader that sees the new generation gets at least its snapshot.
        self.generation.store(generation, Ordering::Release);
        if !self.stopped.load(Ordering::Acquire) {
            (self.wake)();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A library file as last read: its stamp and what it gave.
struct Cached {
    stamp: FileStamp,
    entry: Result<ThemeEntry, Problem>,
}

/// A library theme in use, re-read only when its stamp changes. A failed read keeps the last
/// record that parsed.
#[derive(Default)]
struct Tracked {
    id: Option<ThemeId>,
    /// The stamp of the last read; `None` with `read` set means the file was missing.
    stamp: Option<FileStamp>,
    read: bool,
    record: Option<ThemeRecord>,
    problem: Option<Problem>,
}

impl Tracked {
    /// Built-ins need no file. Returns whether the id changed.
    fn follow(&mut self, id: Option<&ThemeId>) -> bool {
        let id = id.filter(|id| !id.is_builtin());
        if self.id.as_ref() == id {
            return false;
        }
        *self = Tracked { id: id.cloned(), ..Tracked::default() };
        true
    }

    /// A record read by the library listing, so the same file isn't parsed twice.
    fn offer(&mut self, record: &ThemeRecord, stamp: FileStamp) -> bool {
        if self.id.as_ref() != Some(&record.theme.id) || (self.read && self.stamp == Some(stamp)) {
            return false;
        }
        *self = Tracked { id: self.id.take(), stamp: Some(stamp), read: true, record: Some(record.clone()), problem: None };
        true
    }

    /// Returns whether anything changed.
    fn refresh(&mut self, store: &mut Store) -> bool {
        let Some(id) = self.id.clone() else {
            return false;
        };
        let problem = |error: StoreError| Problem::Theme { stem: id.as_str().to_owned(), error };
        let stamp = match store.theme_stamp(&id) {
            Ok(stamp) => stamp,
            Err(e) => return set_problem(&mut self.problem, Some(problem(e.into()))),
        };
        if self.read && self.stamp == stamp {
            return false;
        }
        self.stamp = stamp;
        self.read = true;
        if stamp.is_none() {
            return set_problem(&mut self.problem, Some(problem(StoreError::NotFound(id.clone()))));
        }
        match store.load_theme(&id) {
            Ok(record) => {
                self.record = Some(record);
                self.problem = None;
                true
            }
            Err(e) => {
                set_problem(&mut self.problem, Some(problem(e)));
                // The file changed even if the problem reads the same.
                true
            }
        }
    }
}

/// Replaces `slot` and returns whether it changed. Problems carry I/O errors, which can't be
/// compared, so they compare by their text.
fn set_problem(slot: &mut Option<Problem>, next: Option<Problem>) -> bool {
    let same = slot.as_ref().map(ToString::to_string) == next.as_ref().map(ToString::to_string);
    *slot = next;
    !same
}

/// The worker's state. Every method reports a change by setting `changed`.
struct Engine {
    store: Store,
    accepted: Accepted,
    selection_stamp: Option<Option<FileStamp>>,
    selection_problem: Option<Problem>,
    listed: Option<u64>,
    files: HashMap<String, Cached>,
    index: Arc<[ThemeEntry]>,
    file_problems: Vec<Problem>,
    library_problem: Option<Problem>,
    suite_theme: Tracked,
    app_theme: Tracked,
    results: VecDeque<(u64, Result<OpOutput, StoreError>)>,
    changed: bool,
}

impl Engine {
    fn new(store: Store) -> Self {
        Engine {
            accepted: store.accepted().clone(),
            store,
            selection_stamp: None,
            selection_problem: None,
            listed: None,
            files: HashMap::new(),
            index: Arc::from([]),
            file_problems: Vec::new(),
            library_problem: None,
            suite_theme: Tracked::default(),
            app_theme: Tracked::default(),
            results: VecDeque::new(),
            changed: false,
        }
    }

    /// One timer tick: `selection.json` by stamp (by bytes with `verify`), then the library and
    /// the themes in use.
    fn poll(&mut self, verify: bool) {
        self.check_selection(verify);
        self.settle(false);
    }

    fn apply(&mut self, id: u64, op: Op) {
        let mut rescan = false;
        let result = match op {
            Op::Update { base, change } => self.store.update(&base, change).map(|_| OpOutput::Done),
            Op::Import { bytes, as_copy } => self.store.import(&bytes, as_copy).map(|i| OpOutput::Imported(i.record.theme.id)),
            Op::Delete(theme) => self.store.delete(&theme).map(|_| OpOutput::Done),
            Op::Export(theme) => self.store.export(&theme).map(OpOutput::Exported),
            Op::Rescan => {
                rescan = true;
                Ok(OpOutput::Done)
            }
            Op::AppTheme(theme) => {
                self.app_theme.follow(theme.as_ref());
                Ok(OpOutput::Done)
            }
        };
        if self.results.len() >= RESULTS_KEPT {
            self.results.pop_front();
        }
        self.results.push_back((id, result));
        self.changed = true;
        // A write leaves the store with its new state; the stamp check picks up the file
        // without parsing it again, and clears a problem the write replaced.
        self.check_selection(false);
        self.settle(rescan);
    }

    fn check_selection(&mut self, verify: bool) {
        let stamp = match self.store.selection_stamp() {
            Ok(stamp) => stamp,
            Err(e) => {
                self.changed |= set_problem(&mut self.selection_problem, Some(Problem::SelectionUnreadable(Arc::new(e))));
                return;
            }
        };
        if !verify && self.selection_stamp == Some(stamp) {
            return;
        }
        self.selection_stamp = Some(stamp);
        let problem = self.store.load_selection().err();
        self.changed |= set_problem(&mut self.selection_problem, problem);
    }

    /// Follows the accepted state: the library when its revision moved (or `rescan`), and the
    /// themes in use.
    fn settle(&mut self, rescan: bool) {
        if *self.store.accepted() != self.accepted {
            self.accepted = self.store.accepted().clone();
            self.changed = true;
        }
        let suite = self.accepted.suite.as_ref().map(|s| s.theme.clone());
        self.changed |= self.suite_theme.follow(suite.as_ref());
        if rescan || self.listed != Some(self.accepted.library_revision) {
            self.list_library();
        }
        self.changed |= self.suite_theme.refresh(&mut self.store);
        self.changed |= self.app_theme.refresh(&mut self.store);
    }

    fn list_library(&mut self) {
        self.listed = Some(self.accepted.library_revision);
        let files = match self.store.library_files() {
            Ok(files) => files,
            Err(e) => {
                self.changed |= set_problem(&mut self.library_problem, Some(Problem::LibraryUnreadable(Arc::new(e))));
                return;
            }
        };
        self.changed |= set_problem(&mut self.library_problem, None);
        let mut previous = std::mem::take(&mut self.files);
        let mut changed = files.len() != previous.len();
        for file in files {
            let cached = match previous.remove(&file.stem) {
                Some(cached) if cached.stamp == file.stamp => cached,
                _ => {
                    changed = true;
                    match self.read_library_file(&file) {
                        Some(cached) => cached,
                        // Removed since the listing.
                        None => continue,
                    }
                }
            };
            self.files.insert(file.stem, cached);
        }
        if changed || !previous.is_empty() {
            self.rebuild_index();
        }
    }

    fn read_library_file(&mut self, file: &LibraryFile) -> Option<Cached> {
        let entry = match self.store.load_stem(&file.stem) {
            Ok(None) => return None,
            Ok(Some(record)) => {
                self.changed |= self.suite_theme.offer(&record, file.stamp);
                self.changed |= self.app_theme.offer(&record, file.stamp);
                Ok(ThemeEntry::new(&record, file.stamp))
            }
            Err(error) => Err(Problem::Theme { stem: file.stem.clone(), error }),
        };
        Some(Cached { stamp: file.stamp, entry })
    }

    fn rebuild_index(&mut self) {
        let mut stems: Vec<&String> = self.files.keys().collect();
        stems.sort();
        let mut entries = Vec::with_capacity(stems.len());
        let mut problems = Vec::new();
        for stem in stems {
            match self.files.get(stem).map(|c| &c.entry) {
                Some(Ok(entry)) => entries.push(entry.clone()),
                Some(Err(problem)) => problems.push(problem.clone()),
                None => {}
            }
        }
        self.index = Arc::from(entries);
        self.file_problems = problems;
        self.changed = true;
    }

    fn snapshot(&self, generation: u64) -> Snapshot {
        let mut problems: Vec<Problem> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        // A broken theme in use is also a broken library file; list it once.
        let all = [&self.selection_problem, &self.library_problem, &self.suite_theme.problem, &self.app_theme.problem]
            .into_iter()
            .flatten()
            .chain(&self.file_problems);
        for problem in all {
            if seen.insert(problem.to_string()) {
                problems.push(problem.clone());
            }
        }
        Snapshot {
            accepted: self.accepted.clone(),
            index: Arc::clone(&self.index),
            active: self.suite_theme.record.clone(),
            app_theme: self.app_theme.record.clone(),
            problems,
            results: self.results.iter().cloned().collect(),
            generation,
        }
    }
}

#[cfg(test)]
mod tests;
