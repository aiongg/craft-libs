//! Device health: the one handler of a wgpu device's faults, and the subscribers it tells.
//!
//! wgpu keeps one uncaptured-error handler and one device-lost callback per device; a second
//! registration replaces the first. The window's device is shared by the canvas, the compositor,
//! effects and 3D, so a library that registered its own handler silently disabled the others'
//! (nine places did, on one device). [`DeviceHealth::watch`] is therefore the only code that
//! registers either. Everything else that needs to hear of a fault [`subscribes`] to the device's
//! `DeviceHealth`, and every GPU entry point checks [`DeviceHealth::fault`] before issuing work.
//!
//! wgpu's default handler panics ("Buffer 'pc_compose_uniforms' is invalid" after the OS killed a
//! huge submission, PhotoCraft #243), and `Device::poll` panics on a lost device whatever the
//! handler. So the handler records the fault instead, the waits here poll only while the device is
//! healthy, and a poll's panic is caught as a last resort.
//!
//! Builds for the web too: the callbacks only touch atomics and a mutex.
//!
//! [`subscribes`]: DeviceHealth::subscribe

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use crate::scope::{ScopeError, ScopeKind};

/// Outcome of a bounded wait for submitted GPU work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitOutcome {
    /// The submitted work finished before the deadline.
    Finished,
    /// The deadline passed (or the device faulted) before the work finished.
    TimedOut,
    /// The caller cancelled the wait before the work finished.
    Cancelled,
}

/// Why the GPU can't be used any more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The device was lost (driver reset, the OS killed a submission, `Device::destroy`).
    Lost(String),
    /// An uncaptured wgpu error (out of memory, validation, internal): the GPU output can't be
    /// trusted, and on most drivers the device is about to be lost anyway.
    Error(String),
}

impl Fault {
    /// Whether the device itself was lost (rather than an error reported).
    pub fn is_lost(&self) -> bool {
        matches!(self, Fault::Lost(_))
    }

    /// An out-of-memory error on a device that still works: freeing memory and trying again can
    /// succeed, unlike after a loss or a validation error (PhotoCraft #6564). Only wgpu's own
    /// out-of-memory error is recorded with exactly [`OUT_OF_MEMORY`]; a validation error that
    /// merely mentions memory is not one.
    pub fn is_out_of_memory(&self) -> bool {
        matches!(self, Fault::Error(s) if s == OUT_OF_MEMORY)
    }

    /// The driver's or wgpu's message.
    pub fn detail(&self) -> &str {
        match self {
            Fault::Lost(s) | Fault::Error(s) => s,
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Lost(s) if s.is_empty() => f.write_str("GPU device lost"),
            Fault::Lost(s) => write!(f, "GPU device lost: {s}"),
            Fault::Error(s) => write!(f, "GPU error: {s}"),
        }
    }
}

/// The message an out-of-memory [`Fault::Error`] carries (wgpu's `Error::OutOfMemory`).
pub const OUT_OF_MEMORY: &str = "Out of Memory";

/// The longest fault detail kept: a driver's message can run to pages.
const MAX_DETAIL: usize = 2048;

/// What a subscriber hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HealthEvent {
    /// The device faulted. Sent once per subscriber: at the fault, or at once when it subscribes to
    /// a device that has already faulted.
    Faulted(Fault),
    /// A later fault replaced the recorded one ([`DeviceHealth::mark`]): a loss after an error, or
    /// an out-of-memory error after another error. Work already abandoned at [`Faulted`] must not
    /// be abandoned twice; a UI repaints to show the new state.
    ///
    /// [`Faulted`]: HealthEvent::Faulted
    Upgraded(Fault),
}

type FaultFn = Arc<dyn Fn(&HealthEvent) + Send + Sync>;

/// A subscriber to scoped errors, with the kinds it has heard of already.
struct ScopedSubscriber {
    tell: Arc<dyn Fn(ScopeKind) + Send + Sync>,
    seen: AtomicU8,
}

#[derive(Default)]
struct State {
    fault: Option<Fault>,
    next_id: u64,
    on_fault: Vec<(u64, FaultFn)>,
    on_scoped: Vec<(u64, Arc<ScopedSubscriber>)>,
}

#[derive(Default)]
struct Inner {
    /// [`State::fault`] is set; read without the lock on every GPU entry point.
    faulted: AtomicBool,
    /// The fault and the subscribers under one lock, so a fault and a subscription that race are
    /// delivered exactly once: either the subscription sees the fault, or the fault sees it.
    state: Mutex<State>,
    errors: AtomicU64,
    /// wgpu itself reported the device lost (see [`DeviceHealth::device_lost`]).
    device_lost: AtomicBool,
}

impl Inner {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Shared health of one wgpu device. Cheap to clone; every clone sees the same state.
#[derive(Clone, Default)]
pub struct DeviceHealth(Arc<Inner>);

impl fmt::Debug for DeviceHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceHealth").field("fault", &self.fault()).field("errors", &self.errors()).finish()
    }
}

/// Which list a [`Subscription`] is in.
#[derive(Clone, Copy, Debug)]
enum List {
    Fault,
    Scoped,
}

/// A subscription to a device's health; dropping it unsubscribes.
#[must_use = "dropping a subscription unsubscribes at once"]
pub struct Subscription {
    health: Weak<Inner>,
    id: u64,
    list: List,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription").field("id", &self.id).field("list", &self.list).finish()
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let Some(inner) = self.health.upgrade() else { return };
        let mut state = inner.state();
        match self.list {
            List::Fault => state.on_fault.retain(|(id, _)| *id != self.id),
            List::Scoped => state.on_scoped.retain(|(id, _)| *id != self.id),
        }
    }
}

impl DeviceHealth {
    /// A healthy record not attached to a device (tests, CPU-only callers).
    pub fn new() -> Self {
        Self::default()
    }

    /// Watch `device`: its device-lost callback and uncaptured errors mark the returned record
    /// instead of panicking. The only place in the Craft suite that registers either: wgpu keeps
    /// one of each per device, and this replaces any set before.
    pub fn watch(device: &wgpu::Device) -> Self {
        let health = Self::new();
        let lost = health.clone();
        // Called by wgpu with its own locks held: only record and tell the subscribers, which
        // never call back into wgpu either.
        device.set_device_lost_callback(move |reason, message| {
            let detail = match (reason, message.is_empty()) {
                (wgpu::DeviceLostReason::Destroyed, true) => "device destroyed".to_string(),
                (_, true) => "lost by the driver".to_string(),
                (_, false) => message,
            };
            log::error!("wgpu device lost ({reason:?}): {detail}");
            lost.0.device_lost.store(true, Ordering::Release);
            lost.mark(Fault::Lost(detail));
        });
        let errors = health.clone();
        device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| errors.uncaptured(&e)));
        health
    }

    /// Hear of this device's faults: `tell` gets [`HealthEvent::Faulted`] once (at once, outside
    /// any lock, when the device has faulted already) and [`HealthEvent::Upgraded`] for each later
    /// replacement. `tell` may run on any thread, inside wgpu's callbacks: it must not call into
    /// wgpu, block, or take a lock that GPU work holds. A panic in it is caught and logged (native).
    pub fn subscribe(&self, tell: impl Fn(&HealthEvent) + Send + Sync + 'static) -> Subscription {
        let tell: FaultFn = Arc::new(tell);
        let (id, replay) = {
            let mut state = self.0.state();
            let id = state.next_id;
            state.next_id = id.wrapping_add(1);
            state.on_fault.push((id, tell.clone()));
            (id, state.fault.clone())
        };
        if let Some(fault) = replay {
            deliver(&tell, &HealthEvent::Faulted(fault));
        }
        Subscription { health: Arc::downgrade(&self.0), id, list: List::Fault }
    }

    /// Hear of the errors scopes caught on this device and reported with [`Self::observe`]: each
    /// kind once per subscription, so a failure that repeats every frame can't flood a log or a
    /// notice. The same rules as [`Self::subscribe`] apply to `tell`.
    pub fn subscribe_scoped(&self, tell: impl Fn(ScopeKind) + Send + Sync + 'static) -> Subscription {
        let subscriber = Arc::new(ScopedSubscriber { tell: Arc::new(tell), seen: AtomicU8::new(0) });
        let mut state = self.0.state();
        let id = state.next_id;
        state.next_id = id.wrapping_add(1);
        state.on_scoped.push((id, subscriber));
        Subscription { health: Arc::downgrade(&self.0), id, list: List::Scoped }
    }

    /// Report an error a scope caught as an observation: scoped subscribers hear of its kind, and
    /// the device stays usable. For per-frame errors the caller recovers from (a kernel that
    /// failed to build renders on the CPU).
    pub fn observe(&self, error: &ScopeError) {
        let bit = 1u8 << (error.kind as u8);
        let subscribers: Vec<Arc<ScopedSubscriber>> = self.0.state().on_scoped.iter().map(|(_, s)| s.clone()).collect();
        for s in subscribers {
            if s.seen.fetch_or(bit, Ordering::Relaxed) & bit != 0 {
                continue;
            }
            let tell = &s.tell;
            run_isolated(|| tell(error.kind));
        }
    }

    /// Record an uncaptured wgpu error.
    fn uncaptured(&self, e: &wgpu::Error) {
        self.0.errors.fetch_add(1, Ordering::Relaxed);
        log::error!("uncaptured wgpu error: {e}");
        let oom = matches!(e, wgpu::Error::OutOfMemory { .. });
        self.mark(fault_of(oom, &e.to_string()));
    }

    /// Report an error a scope caught as a fault that retires the device, for callers that stop
    /// using the GPU after a failed frame (PhotoCraft's compositor falls back to the CPU).
    ///
    /// [`Self::watch`]'s handler never sees a scoped error, so without this a watched device would
    /// keep being handed out after it failed. Polls first: a lost device is a [`Fault::Lost`], and
    /// its callback may not have run yet (it fires on a poll). The poll is a single non-blocking
    /// check, never a fence wait: this runs on the UI thread right after a failed frame, and a wait
    /// would stall the app until every submission drained, or forever on a wedged device.
    ///
    /// A device that only recorded an error is still polled and marked, so a loss or an
    /// out-of-memory error can upgrade that fault as [`Self::mark`] allows (PhotoCraft #6564).
    pub fn caught(&self, device: &wgpu::Device, error: &ScopeError) {
        if self.lost() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.poll(device);
        #[cfg(target_arch = "wasm32")]
        let _ = device;
        if self.lost() {
            return;
        }
        self.mark(fault_of(error.kind == ScopeKind::OutOfMemory, &error.message));
    }

    fn lost(&self) -> bool {
        self.fault().is_some_and(|f| f.is_lost())
    }

    /// Mark the device unusable. The first fault is kept; a later device loss upgrades an error,
    /// and a later out-of-memory error upgrades another error, so the canvas can still recover by
    /// itself when memory ran out after a validation error in the same frame (PhotoCraft #6564).
    /// Subscribers hear of the first fault and of each upgrade.
    pub fn mark(&self, fault: Fault) {
        let fault = bounded(fault);
        let (event, subscribers) = {
            let mut state = self.0.state();
            let event = match state.fault.as_ref() {
                None => Some(HealthEvent::Faulted(fault.clone())),
                Some(old) if !old.is_lost() && (fault.is_lost() || (fault.is_out_of_memory() && !old.is_out_of_memory())) => {
                    Some(HealthEvent::Upgraded(fault.clone()))
                }
                Some(_) => None,
            };
            if event.is_some() {
                state.fault = Some(fault);
            }
            self.0.faulted.store(true, Ordering::Release);
            let subscribers: Vec<FaultFn> = match event {
                Some(_) => state.on_fault.iter().map(|(_, f)| f.clone()).collect(),
                None => Vec::new(),
            };
            (event, subscribers)
        };
        if let Some(event) = event {
            for tell in &subscribers {
                deliver(tell, &event);
            }
        }
    }

    /// Whether the device may still be used.
    pub fn is_ok(&self) -> bool {
        !self.0.faulted.load(Ordering::Acquire)
    }

    /// Why the device can't be used, if it can't.
    pub fn fault(&self) -> Option<Fault> {
        if self.is_ok() {
            return None;
        }
        self.0.state().fault.clone()
    }

    /// Uncaptured wgpu errors seen so far.
    pub fn errors(&self) -> u64 {
        self.0.errors.load(Ordering::Relaxed)
    }

    /// Whether wgpu itself reported the device lost: its device-lost callback ran, or polling it
    /// panicked. Unlike a fault recorded with [`mark`](Self::mark) (a simulated loss, or an
    /// uncaptured error that names one), nothing drawn with the device reaches the screen any
    /// more, the window's own frames included (PhotoCraft #3438): the window's presentation must
    /// be recreated too.
    pub fn device_lost(&self) -> bool {
        self.0.device_lost.load(Ordering::Acquire)
    }

    /// Non-blocking poll that runs even after a fault was recorded, so a device that is lost
    /// later (a driver reset while the app sat idle or the PC slept) is noticed before the next
    /// frame is drawn with it. Skipped once [`device_lost`](Self::device_lost).
    pub fn probe(&self, device: &wgpu::Device) {
        if self.device_lost() {
            return;
        }
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.poll(wgpu::PollType::Poll))).is_err() {
            self.lost_by_poll();
        }
    }

    /// `Device::poll` panicked: wgpu found the device lost.
    fn lost_by_poll(&self) {
        self.0.device_lost.store(true, Ordering::Release);
        self.mark(Fault::Lost("the device stopped responding".into()));
    }

    /// Block until `index` (or all submitted work) finished, unless the device is unusable.
    /// Returns whether the device is still healthy afterwards. `Device::poll` panics on a lost
    /// device, so a panic is caught and recorded as a loss (native only; the web has no blocking
    /// wait).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait(&self, device: &wgpu::Device, index: Option<wgpu::SubmissionIndex>) -> bool {
        self.wait_idle(device, index);
        self.is_ok()
    }

    /// Like [`Self::wait`], but returns whether the wait completed: `false` also when the poll
    /// itself failed, so the work may still be running.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait_idle(&self, device: &wgpu::Device, index: Option<wgpu::SubmissionIndex>) -> bool {
        if !self.is_ok() {
            return false;
        }
        let poll =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.poll(wgpu::PollType::Wait { submission_index: index, timeout: None })));
        match poll {
            Ok(Ok(_)) => self.is_ok(),
            Ok(Err(e)) => {
                log::warn!("wgpu poll: {e}");
                false
            }
            Err(_) => {
                self.lost_by_poll();
                false
            }
        }
    }

    /// Non-blocking poll (reclaims staging memory), skipped once the device is unusable.
    pub fn poll(&self, device: &wgpu::Device) {
        if !self.is_ok() {
            return;
        }
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.poll(wgpu::PollType::Poll))).is_err() {
            self.lost_by_poll();
        }
    }

    /// Like [`Self::wait`], but bounded: returns whether the submitted work finished within
    /// `timeout`. A timeout (or a poll that fails without reporting a loss) marks the device
    /// faulted, so later work stops going to a wedged GPU instead of each waiting the full timeout
    /// again. A device that was already faulted returns `false` untouched.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait_timeout(&self, device: &wgpu::Device, index: Option<wgpu::SubmissionIndex>, timeout: Duration) -> bool {
        self.wait_timeout_cancellable(device, index, timeout, &|| false) == WaitOutcome::Finished
    }

    /// [`Self::wait_timeout`] that polls in short slices and checks `cancelled` between them, so a
    /// cancelled headless export stops waiting promptly while keeping the same overall deadline.
    /// A slice timeout means "not finished yet"; only the overall deadline (or a failed poll)
    /// marks the device faulted.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait_timeout_cancellable(
        &self,
        device: &wgpu::Device,
        index: Option<wgpu::SubmissionIndex>,
        timeout: Duration,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> WaitOutcome {
        if !self.is_ok() {
            return WaitOutcome::TimedOut;
        }
        let mut failure = None;
        let outcome = wait_slices(timeout, cancelled, |slice| {
            let poll = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                device.poll(wgpu::PollType::Wait { submission_index: index.clone(), timeout: Some(slice) })
            }));
            match poll {
                Ok(Ok(_)) => {
                    if self.is_ok() {
                        SlicePoll::Done
                    } else {
                        SlicePoll::Failed
                    }
                }
                Ok(Err(wgpu::PollError::Timeout)) => SlicePoll::Pending,
                Ok(Err(e)) => {
                    failure = Some(format!("GPU poll failed: {e}"));
                    SlicePoll::Failed
                }
                Err(_) => {
                    self.lost_by_poll();
                    SlicePoll::Failed
                }
            }
        });
        if outcome == WaitOutcome::TimedOut && self.is_ok() {
            let fault = failure.map_or_else(|| Fault::Lost(format!("GPU wait timed out after {:.1} s", timeout.as_secs_f64())), Fault::Error);
            self.mark(fault);
        }
        outcome
    }
}

/// Tell one subscriber, keeping its panic out of wgpu's error handling.
fn deliver(tell: &FaultFn, event: &HealthEvent) {
    run_isolated(|| tell(event));
}

/// Run a subscriber; on native targets a panic in it is caught and logged.
fn run_isolated(f: impl FnOnce()) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err() {
        log::error!("a GPU health subscriber panicked");
    }
    #[cfg(target_arch = "wasm32")]
    f();
}

/// `fault` with its detail cut to [`MAX_DETAIL`] characters.
fn bounded(fault: Fault) -> Fault {
    let cut = |s: String| if s.chars().count() > MAX_DETAIL { s.chars().take(MAX_DETAIL).collect() } else { s };
    match fault {
        Fault::Lost(s) => Fault::Lost(cut(s)),
        Fault::Error(s) => Fault::Error(cut(s)),
    }
}

/// What one bounded poll reported.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlicePoll {
    Done,
    Pending,
    Failed,
}

/// Wait for `total`, polling in slices no longer than 100 ms and checking `cancelled` between
/// them. A slice that times out is pending, not a failure; only the overall deadline or a failed
/// poll ends the wait early.
#[cfg(not(target_arch = "wasm32"))]
fn wait_slices(total: Duration, cancelled: &(dyn Fn() -> bool + Sync), mut poll: impl FnMut(Duration) -> SlicePoll) -> WaitOutcome {
    const SLICE: Duration = Duration::from_millis(100);
    let start = Instant::now();
    loop {
        if cancelled() {
            return WaitOutcome::Cancelled;
        }
        let remaining = total.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return WaitOutcome::TimedOut;
        }
        match poll(remaining.min(SLICE)) {
            SlicePoll::Done => return WaitOutcome::Finished,
            SlicePoll::Pending => {}
            SlicePoll::Failed => return WaitOutcome::TimedOut,
        }
    }
}

/// The fault a wgpu error stands for; `out_of_memory` when it is wgpu's own out-of-memory error.
///
/// An error about an invalid or lost object follows a device loss whose callback may not have run
/// yet (it fires on the next poll), so it counts as a loss rather than as an error. This is read
/// from the message text ("lost", "destroyed"): wgpu 30 offers no typed signal that an error was
/// caused by a lost device. Replace this with that signal once wgpu has one.
fn fault_of(out_of_memory: bool, text: &str) -> Fault {
    let lower = text.to_ascii_lowercase();
    if out_of_memory {
        Fault::Error(OUT_OF_MEMORY.into())
    } else if lower.contains("lost") || lower.contains("destroyed") {
        Fault::Lost(first_line(text))
    } else {
        Fault::Error(first_line(text))
    }
}

/// The first non-empty line of `s`, at most 200 characters (and an ellipsis).
fn first_line(s: &str) -> String {
    let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let mut out: String = line.chars().take(200).collect();
    if line.chars().count() > 200 {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests;
