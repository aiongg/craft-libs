//! Pending work and jobs (docs/contract-rt.md section 10, "Pending and wakes"). A job runs off the UI
//! thread; its result is delivered into a ui-rt cell (or the image store) on the UI thread when the
//! platform wakes the loop, so the node that waits is the node that reads the cell: it is marked, with
//! the scopes on its path, and nothing else runs.
//!
//! The host owns one [`JobBoard`] per runtime (built from its job pool and the driver's waker, so a finished job wakes the loop once) and calls [`JobBoard::deliver`] when it gets
//! `PlatformEvent::Wake` (before the next step). A window root provides the board to its nodes
//! ([`super::WindowEnv::jobs`]).

use std::hash::Hash;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, PoisonError};

use craft_host::{InlineJobs, Jobs};
use craft_ui_rt::{ImageStore, Store, Waker};

pub use crate::runtime::pending::Pending;

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
    jobs: Arc<dyn Jobs>,
    waker: Option<Waker>,
    done: Mutex<Vec<Done>>,
}

/// The queue between background jobs and the UI thread.
#[derive(Clone)]
pub struct JobBoard {
    inner: Arc<Inner>,
}

impl JobBoard {
    /// Jobs run at once on the calling thread and nothing wakes a loop: the Harness, whose steps call
    /// [`JobBoard::deliver`] on their own schedule. A host builds the runtime's one board with
    /// [`JobBoard::new`] from its job pool and the driver's waker, and gives it to every window
    /// root ([`super::WindowEnv::jobs`]).
    pub fn inline() -> JobBoard {
        JobBoard { inner: Arc::new(Inner { jobs: Arc::new(InlineJobs), waker: None, done: Mutex::new(Vec::new()) }) }
    }
}

impl JobBoard {
    /// A board that runs jobs on `jobs` and calls `waker` when one finishes.
    pub fn new(jobs: Arc<dyn Jobs>, waker: Waker) -> JobBoard {
        JobBoard { inner: Arc::new(Inner { jobs, waker: Some(waker), done: Mutex::new(Vec::new()) }) }
    }

    /// Runs `work` in the background; `done` receives its result on the UI thread at the next
    /// [`JobBoard::deliver`]. A panic in `work` is delivered as `Err`.
    pub fn spawn<R: Send + 'static>(
        &self,
        name: &str,
        work: impl FnOnce() -> R + Send + 'static,
        done: impl FnOnce(Result<R, ()>, &mut Delivery<'_>) + Send + 'static,
    ) {
        let inner = self.inner.clone();
        self.inner.jobs.spawn(
            name,
            Box::new(move || {
                let result = std::panic::catch_unwind(AssertUnwindSafe(work)).map_err(|_| ());
                inner.done.lock().unwrap_or_else(PoisonError::into_inner).push(Box::new(move |d| done(result, d)));
                if let Some(waker) = &inner.waker {
                    waker.wake();
                }
            }),
        );
    }

    /// Hands every finished job's result to its `done`, in the order they finished. Returns how many.
    /// Writes to cells mark the nodes that read them; an image inserted damages the nodes drawing it.
    pub fn deliver(&self, store: &mut Store, images: &mut ImageStore) -> usize {
        let finished = std::mem::take(&mut *self.inner.done.lock().unwrap_or_else(PoisonError::into_inner));
        let count = finished.len();
        let mut delivery = Delivery { store, images };
        for done in finished {
            done(&mut delivery);
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
            board.spawn("craft-job", work, move |result, d| {
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
