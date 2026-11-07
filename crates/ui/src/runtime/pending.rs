//! Background results (14): one result per key in runtime memory, produced by a job on `host::Jobs`.

use crate::runtime::egui::repaint;
use crate::runtime::env::Env;
use crate::runtime::memory::MemoryStage;
use craft_host::queue::Cancel;
use egui::{Context, Id};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// A result that may not have arrived.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Pending<T, E> {
    /// Still running.
    #[non_exhaustive]
    Loading {
        /// The job's last report, 0 to 1.
        progress: Option<f32>,
    },
    /// The job succeeded.
    Ready(T),
    /// The job failed.
    Failed(E),
}

impl<T, E> Default for Pending<T, E> {
    fn default() -> Self {
        Pending::Loading { progress: None }
    }
}

/// A job's report of how far it has come. A report asks for a pass, so the progress shows.
#[derive(Clone)]
pub struct Progress {
    bits: Arc<AtomicU32>,
    ctx: Context,
}

/// The stored bits of "no report yet": a NaN, which a report never writes.
const NO_REPORT: u32 = u32::MAX;

impl Progress {
    /// Reports `fraction` (0 to 1) done.
    pub fn report(&self, fraction: f32) {
        self.bits.store(fraction.clamp(0.0, 1.0).to_bits(), Ordering::Release);
        repaint::request(&self.ctx);
    }

    fn get(&self) -> Option<f32> {
        match self.bits.load(Ordering::Acquire) {
            NO_REPORT => None,
            bits => Some(f32::from_bits(bits)),
        }
    }
}

type Run<T, E> = Box<dyn FnOnce(&Progress, &Cancel) -> Result<T, E> + Send>;

/// Background work for `Remember::pending`: runs on `host::Jobs`, stops early when its owner is evicted.
pub struct Job<T, E> {
    name: &'static str,
    run: Run<T, E>,
}

impl<T, E> Job<T, E> {
    /// A job; `name` labels its thread. `run` checks `Cancel::is_cancelled` where it can stop early.
    pub fn new(name: &'static str, run: impl FnOnce(&Progress, &Cancel) -> Result<T, E> + Send + 'static) -> Self {
        Job { name, run: Box::new(run) }
    }
}

/// What the job leaves for the next pass.
struct Shared<T, E> {
    result: Mutex<Option<Result<T, E>>>,
    progress: Progress,
}

/// Cancels the job when the last slot holding it is dropped, which is when the owner is evicted.
struct CancelOnDrop(Cancel);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

struct Started<T, E> {
    shared: Arc<Shared<T, E>>,
    _owner: Arc<CancelOnDrop>,
}

impl<T, E> Clone for Started<T, E> {
    fn clone(&self) -> Self {
        Started { shared: self.shared.clone(), _owner: self._owner.clone() }
    }
}

/// The memory slot of one key.
pub(crate) struct PendingSlot<T, E> {
    shown: Pending<T, E>,
    started: Option<Started<T, E>>,
}

impl<T: Clone, E: Clone> Clone for PendingSlot<T, E> {
    fn clone(&self) -> Self {
        PendingSlot { shown: self.shown.clone(), started: self.started.clone() }
    }
}

impl<T, E> Default for PendingSlot<T, E> {
    fn default() -> Self {
        PendingSlot { shown: Pending::default(), started: None }
    }
}

/// The result under `id`: submits `start()` once, takes a finished result into the slot and returns it. Without
/// a host there is no spawner and the result stays loading; a sizing pass keeps nothing, so it starts nothing.
pub(crate) fn poll<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
    stage: &mut MemoryStage,
    id: Id,
    start: impl FnOnce() -> Job<T, E>,
) -> &Pending<T, E> {
    let ctx = stage.ctx().clone();
    let sizing = stage.is_peek();
    let slot = stage.shown::<PendingSlot<T, E>>(id);
    if slot.started.is_none()
        && !sizing
        && let Some(host) = Env::host_of(&ctx)
    {
        let Job { name, run } = start();
        let progress = Progress { bits: Arc::new(AtomicU32::new(NO_REPORT)), ctx: ctx.clone() };
        let shared = Arc::new(Shared { result: Mutex::new(None), progress: progress.clone() });
        let cancel = Cancel::new();
        let (landing, token) = (shared.clone(), cancel.clone());
        host.jobs().spawn(
            name,
            Box::new(move || {
                let result = run(&progress, &token);
                if !token.is_cancelled() {
                    *landing.result.lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
                    repaint::request(&ctx);
                }
            }),
        );
        slot.started = Some(Started { shared, _owner: Arc::new(CancelOnDrop(cancel)) });
    }
    if let Some(started) = &slot.started
        && matches!(slot.shown, Pending::Loading { .. })
    {
        let landed = started.shared.result.lock().unwrap_or_else(PoisonError::into_inner).take();
        slot.shown = match landed {
            Some(Ok(value)) => Pending::Ready(value),
            Some(Err(error)) => Pending::Failed(error),
            None => Pending::Loading { progress: started.shared.progress.get() },
        };
    }
    &slot.shown
}
