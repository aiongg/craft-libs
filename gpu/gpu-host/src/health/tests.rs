use super::*;
use std::sync::atomic::AtomicUsize;

fn scope_error(kind: ScopeKind) -> ScopeError {
    ScopeError { kind, message: format!("{kind:?}") }
}

/// The fault latch: the first fault is kept, a loss upgrades an error, an out-of-memory error
/// upgrades another error but never a loss (PhotoCraft #6564), and a recorded loss is not wgpu
/// reporting the device lost (#3438).
#[test]
fn first_fault_wins_but_loss_upgrades_an_error() {
    let h = DeviceHealth::new();
    assert!(h.is_ok());
    assert_eq!(h.fault(), None);
    h.mark(Fault::Error("Validation Error".into()));
    assert!(!h.is_ok());
    let c = h.clone();
    c.mark(Fault::Error("second".into()));
    assert_eq!(h.fault(), Some(Fault::Error("Validation Error".into())));
    c.mark(Fault::Error(OUT_OF_MEMORY.into()));
    assert!(h.fault().is_some_and(|f| f.is_out_of_memory()), "out of memory replaces another error");
    c.mark(Fault::Lost("reset".into()));
    assert_eq!(h.fault(), Some(Fault::Lost("reset".into())));
    c.mark(Fault::Lost("later".into()));
    c.mark(Fault::Error(OUT_OF_MEMORY.into()));
    assert_eq!(h.fault(), Some(Fault::Lost("reset".into())), "nothing replaces a loss");
    assert!(!h.device_lost());
    assert!(!Fault::Error("Validation Error: buffer allocation would run out of memory".into()).is_out_of_memory());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn wait_slices_cancel_timeout_and_completion() {
    let never = || false;
    assert_eq!(wait_slices(Duration::ZERO, &never, |_| SlicePoll::Done), WaitOutcome::TimedOut);
    assert_eq!(wait_slices(Duration::from_secs(1), &never, |_| SlicePoll::Done), WaitOutcome::Finished);
    assert_eq!(wait_slices(Duration::from_secs(1), &never, |_| SlicePoll::Failed), WaitOutcome::TimedOut);

    let mut polls = 0;
    let outcome = wait_slices(Duration::from_secs(1), &never, |_| {
        polls += 1;
        if polls < 3 { SlicePoll::Pending } else { SlicePoll::Done }
    });
    assert_eq!(outcome, WaitOutcome::Finished);
    assert_eq!(polls, 3, "a timed-out slice must be retried, not treated as failure");

    let cancelled = || true;
    let mut called = false;
    assert_eq!(
        wait_slices(Duration::from_secs(1), &cancelled, |_| {
            called = true;
            SlicePoll::Done
        }),
        WaitOutcome::Cancelled
    );
    assert!(!called, "cancellation must be checked before polling");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn wait_slices_observes_cancel_while_pending() {
    let flag = AtomicBool::new(false);
    let cancelled = || flag.load(Ordering::SeqCst);
    let mut polls = 0;
    let outcome = wait_slices(Duration::from_secs(1), &cancelled, |_| {
        polls += 1;
        flag.store(true, Ordering::SeqCst);
        SlicePoll::Pending
    });
    assert_eq!(outcome, WaitOutcome::Cancelled);
    assert_eq!(polls, 1, "cancellation set during a pending poll must end the wait");
}

/// A fault before anyone subscribed (a compositor that failed while it was built) is replayed to
/// a later subscriber, outside the lock, and a repeated fault is not delivered again.
#[test]
fn a_construction_failure_is_replayed_after_notifier_registration() {
    let h = DeviceHealth::new();
    h.mark(Fault::Error("pipeline allocation failed".into()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let copy = seen.clone();
    let probe = Arc::downgrade(&h.0);
    let _sub = h.subscribe(move |event| {
        assert!(probe.upgrade().unwrap().state.try_lock().is_ok(), "subscribers run outside the health lock");
        copy.lock().unwrap().push(event.clone());
    });
    assert_eq!(*seen.lock().unwrap(), [HealthEvent::Faulted(Fault::Error("pipeline allocation failed".into()))]);
    h.mark(Fault::Error("duplicate".into()));
    assert_eq!(seen.lock().unwrap().len(), 1);
}

/// A fault that arrives after subscribing is delivered once, and a later loss arrives once more as
/// an upgrade (the presentation must be recreated); further errors are not delivered.
#[test]
fn a_late_fault_notifies_once_and_a_loss_upgrades_it() {
    let h = DeviceHealth::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let copy = seen.clone();
    let _sub = h.subscribe(move |event| copy.lock().unwrap().push(event.clone()));
    let dropped = Arc::new(AtomicUsize::new(0));
    let count = dropped.clone();
    let gone = h.subscribe(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
    });
    h.mark(Fault::Error("uncaptured allocation failure".into()));
    drop(gone);
    h.mark(Fault::Lost("device lost".into()));
    h.mark(Fault::Error("another error".into()));
    assert_eq!(
        *seen.lock().unwrap(),
        [HealthEvent::Faulted(Fault::Error("uncaptured allocation failure".into())), HealthEvent::Upgraded(Fault::Lost("device lost".into()))]
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 1, "a dropped subscription hears nothing more");
}

/// A fault and a subscription racing on two threads deliver exactly once.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn concurrent_registration_and_failure_deliver_exactly_once() {
    for _ in 0..64 {
        let h = DeviceHealth::new();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let calls = Arc::new(AtomicUsize::new(0));
        let reporter = h.clone();
        let ready = barrier.clone();
        let thread = std::thread::spawn(move || {
            ready.wait();
            reporter.mark(Fault::Error("device failed during subscription".into()));
        });
        let count = calls.clone();
        barrier.wait();
        let _sub = h.subscribe(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
        });
        thread.join().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

/// A driver message can run to pages: the kept detail is bounded. A subscriber that panics inside
/// wgpu's callback doesn't take the handler down; the next fault is still recorded.
#[test]
fn diagnostics_are_bounded_and_native_consumer_panics_are_isolated() {
    let h = DeviceHealth::new();
    #[cfg(not(target_arch = "wasm32"))]
    let _sub = h.subscribe(|_| panic!("synthetic consumer failure"));
    h.mark(Fault::Error("é".repeat(3000)));
    assert_eq!(h.fault().unwrap().detail().chars().count(), MAX_DETAIL);
    h.mark(Fault::Lost("device lost after consumer failure".into()));
    assert!(h.fault().unwrap().is_lost());
}

/// Scoped errors reach each subscriber once per kind (a failure every frame can't flood it), each
/// subscription and each device keeps its own record, and the device stays usable.
#[test]
fn categories_are_independent_bounded_and_not_global() {
    let h = DeviceHealth::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let copy = seen.clone();
    let _sub = h.subscribe_scoped(move |kind| copy.lock().unwrap().push(kind));
    for _ in 0..100 {
        h.observe(&scope_error(ScopeKind::Internal));
        h.observe(&scope_error(ScopeKind::Validation));
        h.observe(&scope_error(ScopeKind::OutOfMemory));
    }
    assert_eq!(*seen.lock().unwrap(), [ScopeKind::Internal, ScopeKind::Validation, ScopeKind::OutOfMemory]);
    assert!(h.is_ok(), "an observed error leaves the device usable");

    let later = Arc::new(AtomicUsize::new(0));
    let count = later.clone();
    let _second = h.subscribe_scoped(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
    });
    h.observe(&scope_error(ScopeKind::Internal));
    assert_eq!(later.load(Ordering::SeqCst), 1, "a new subscription keeps its own record");
    DeviceHealth::new().observe(&scope_error(ScopeKind::Panicked));
    assert_eq!(seen.lock().unwrap().len(), 3, "another device's errors are not this one's");
}
