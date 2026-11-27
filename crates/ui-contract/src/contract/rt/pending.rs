//! Pending work and jobs (docs/contract-rt.md section 10, "Pending and wakes"). A job runs off the UI
//! thread; its result is delivered into a ui-rt cell (or the image store) on the UI thread when the
//! platform wakes the loop, so the node that waits is the node that reads the cell: it is marked, with
//! the scopes on its path, and nothing else runs.
//!
//! The host owns one [`JobBoard`] per runtime (built from its job pool and the driver's waker, so a finished job wakes the loop once) and calls [`JobBoard::deliver`] when it gets
//! `PlatformEvent::Wake` (before the next step). A window root provides the board to its nodes
//! ([`super::WindowEnv::jobs`]).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use craft_host::queue::{JobQueue, QueueOptions};
use craft_host::{InlineJobs, Jobs, Wake};
use craft_ui_rt::{ImageStore, Store, Waker};

pub use crate::runtime::pending::Pending;
pub use craft_host::queue::{Cancel, Priority};

use super::ui::Ui;

/// What a finished job writes: the cells and the images, on the UI thread.
pub struct Delivery<'a> {
    /// The runtime's cells.
    pub store: &'a mut Store,
    /// The images renderers upload.
    pub images: &'a mut ImageStore,
}

type Done = Box<dyn FnOnce(&mut Delivery<'_>) + Send>;

struct Inner {
    /// Every job, keyed by its serial: the host queue's lanes run the best one first.
    queue: JobQueue<u64, Done>,
    /// Results handed in on the UI thread ([`JobBoard::post`]).
    posted: Mutex<Vec<Done>>,
    wake: Wake,
    /// The driver's waker behind `wake`; `None` on an inline board.
    waker: Option<Waker>,
    serial: AtomicU64,
    /// The runtime's shared services built on the board (texture caches), by type and name.
    shared: Mutex<HashMap<(TypeId, u64), Box<dyn Any + Send + Sync>>>,
}

/// The queue between background jobs and the UI thread: jobs run on the host's job pool through
/// [`craft_host::queue`] lanes, best priority first, and each finished job wakes the loop once.
#[derive(Clone)]
pub struct JobBoard {
    inner: Arc<Inner>,
}

/// A queued or running job: lower its priority as it scrolls away, or cancel it. A cancelled
/// job's result is never delivered.
#[derive(Clone)]
pub struct JobHandle {
    serial: u64,
    board: Weak<Inner>,
}

impl JobHandle {
    /// Drops the job: a waiting one never runs, a running one sees [`Cancel::is_cancelled`].
    pub fn cancel(&self) {
        if let Some(inner) = self.board.upgrade() {
            inner.queue.reprioritize(|serial, priority| (*serial != self.serial).then_some(priority));
        }
    }

    /// Ranks the job at `priority` among the waiting ones.
    pub fn set_priority(&self, priority: Priority) {
        if let Some(inner) = self.board.upgrade() {
            inner.queue.reprioritize(|serial, old| Some(if *serial == self.serial { priority } else { old }));
        }
    }
}

impl JobBoard {
    /// Jobs run at once on the calling thread and nothing wakes a loop: the Harness, whose steps call
    /// [`JobBoard::deliver`] on their own schedule. A host builds the runtime's one board with
    /// [`JobBoard::new`] from its job pool and the driver's waker, and gives it to every window
    /// root ([`super::WindowEnv::jobs`]).
    pub fn inline() -> JobBoard {
        JobBoard::with(Arc::new(InlineJobs), Wake::none(), None)
    }

    /// The waker the board calls when a job finishes, if it has one.
    pub(crate) fn waker(&self) -> Option<Waker> {
        self.inner.waker.clone()
    }

    /// A board that runs jobs on `jobs` and calls `waker` when one finishes.
    pub fn new(jobs: Arc<dyn Jobs>, waker: Waker) -> JobBoard {
        let wake = {
            let waker = waker.clone();
            Wake::new(move || waker.wake())
        };
        JobBoard::with(jobs, wake, Some(waker))
    }

    fn with(jobs: Arc<dyn Jobs>, wake: Wake, waker: Option<Waker>) -> JobBoard {
        // At least one lane: on wasm32 the host's jobs run inline, so a lane runs its jobs at once.
        let options = QueueOptions { lanes: QueueOptions::default_lanes().max(1), name: "craft-jobs", ..QueueOptions::default() };
        let queue = JobQueue::new(jobs, wake.clone(), options);
        JobBoard {
            inner: Arc::new(Inner {
                queue,
                posted: Mutex::new(Vec::new()),
                wake,
                waker,
                serial: AtomicU64::new(0),
                shared: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Runs `work` in the background at [`Priority::VISIBLE`]; `done` receives its result on the UI
    /// thread at the next [`JobBoard::deliver`]. A panic in `work` is delivered as `Err`.
    pub fn spawn<R: Send + 'static>(
        &self,
        work: impl FnOnce() -> R + Send + 'static,
        done: impl FnOnce(Result<R, ()>, &mut Delivery<'_>) + Send + 'static,
    ) {
        self.spawn_with(Priority::VISIBLE, move |_| work(), done);
    }

    /// Runs `work` in the background at `priority`; `done` receives its result on the UI thread at
    /// the next [`JobBoard::deliver`], unless the returned handle cancelled it first. `work` reads
    /// the cancel flag between steps. A panic in `work` is delivered as `Err`.
    pub fn spawn_with<R: Send + 'static>(
        &self,
        priority: Priority,
        work: impl FnOnce(&Cancel) -> R + Send + 'static,
        done: impl FnOnce(Result<R, ()>, &mut Delivery<'_>) + Send + 'static,
    ) -> JobHandle {
        let serial = self.inner.serial.fetch_add(1, Ordering::Relaxed);
        self.inner.queue.submit(serial, priority, move |cancel| -> Done {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| work(cancel))).map_err(|_| ());
            Box::new(move |d| done(result, d))
        });
        JobHandle { serial, board: Arc::downgrade(&self.inner) }
    }

    /// The value of type `T` named `name` on this board, made by `make` the first time: one texture cache per
    /// name for the whole runtime, found by every node that names it.
    pub fn shared<T: Clone + Send + Sync + 'static>(&self, name: u64, make: impl FnOnce() -> T) -> T {
        let key = (TypeId::of::<T>(), name);
        let lock = || self.inner.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(value) = lock().get(&key) {
            return value.downcast_ref::<T>().expect("keyed by its type").clone();
        }
        // Made outside the lock: a value may be made of other shared values (thumbnails of a store).
        let made = make();
        lock().entry(key).or_insert_with(|| Box::new(made)).downcast_ref::<T>().expect("keyed by its type").clone()
    }

    /// Hands `done` to the next [`JobBoard::deliver`] and wakes the loop: UI-thread work that writes
    /// cells or images outside a build (a retry in the next frame).
    pub fn post(&self, done: impl FnOnce(&mut Delivery<'_>) + Send + 'static) {
        self.inner.posted.lock().unwrap_or_else(PoisonError::into_inner).push(Box::new(done));
        self.inner.wake.wake();
    }

    /// Hands every finished job's result to its `done`, in the order they finished, after the
    /// posted ones. Returns how many. Writes to cells mark the nodes that read them; an image
    /// inserted damages the nodes drawing it.
    pub fn deliver(&self, store: &mut Store, images: &mut ImageStore) -> usize {
        let posted = std::mem::take(&mut *self.inner.posted.lock().unwrap_or_else(PoisonError::into_inner));
        let mut count = posted.len();
        let mut delivery = Delivery { store, images };
        for done in posted {
            done(&mut delivery);
        }
        while let Some(finished) = self.inner.queue.try_recv() {
            (finished.result)(&mut delivery);
            count += 1;
        }
        count
    }
}

impl Ui<'_, '_> {
    /// The result of the job `key` names, started by the first call: `work` runs in the background and
    /// this node reads the result cell, so only this node (and the scopes on its path) runs when the
    /// result arrives. The cell belongs to the node and goes with it.
    pub fn job<T, E>(&mut self, key: impl Hash, work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Pending<T, E>
    where
        T: Clone + Send + 'static,
        E: Clone + Send + 'static,
    {
        let cell = self.cx.slot(("job", &key_hash(&key)), || Pending::<T, E>::Loading { progress: None });
        let started = self.cx.slot(("job-started", &key_hash(&key)), || false);
        if !*self.cx.peek_state(started) {
            started.set(self.cx, true);
            let board = self.cx.provided::<JobBoard>().unwrap_or_else(JobBoard::inline);
            board.spawn(work, move |result, d| {
                let value = match result {
                    Ok(Ok(t)) => Pending::Ready(t),
                    Ok(Err(e)) => Pending::Failed(e),
                    Err(()) => Pending::Lost,
                };
                cell.set(&mut *d.store, value);
            });
        }
        cell.get(self.cx)
    }
}

fn key_hash(key: &impl Hash) -> u64 {
    craft_ui_rt::hash_key(key)
}
