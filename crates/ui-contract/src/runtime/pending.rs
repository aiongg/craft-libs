//! Background results (14): one result per key in runtime memory, produced by a job on a `host::queue::JobQueue` per context.

pub use crate::contract::rt::pending::{Delivery, JobBoard, JobHandle};

use crate::runtime::egui::repaint;

use craft_host::queue::{Cancel, Priority};

use egui::Context;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// A result that may not have arrived.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Pending<T, E> {
    /// Still running.
    Loading {
        /// The job's last report, 0 to 1.
        progress: Option<f32>,
    },
    /// The job succeeded.
    Ready(T),
    /// The job failed.
    Failed(E),
    /// The job ended without delivering, [`LOST_LIMIT`] times in a row (it panics, or the host never runs it): it is
    /// not submitted again.
    Lost,
}

/// How many times in a row a job may end without delivering before its key shows [`Pending::Lost`].
pub const LOST_LIMIT: u32 = 3;

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

/// Background work for `Remember::pending`: runs on the context's job queue, stops early when its owner is
/// evicted or stops asking for its key.
pub struct Job<T, E> {
    priority: Priority,
    run: Run<T, E>,
}

impl<T, E> Job<T, E> {
    /// A job at `Priority::VISIBLE`. `run` checks `Cancel::is_cancelled` where it can stop early.
    pub fn new(run: impl FnOnce(&Progress, &Cancel) -> Result<T, E> + Send + 'static) -> Self {
        Job { priority: Priority::VISIBLE, run: Box::new(run) }
    }

    /// Runs before waiting jobs of lower priority.
    #[must_use]
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}
