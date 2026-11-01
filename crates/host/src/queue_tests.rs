//! `JobQueue` and `ManualJobs`: order, replacement, one job per key, cancellation, wakes and
//! inline runs. Every test drives lanes with `ManualJobs` (or `lanes: 0`), so nothing depends on
//! thread timing.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::queue::{Cancel, Finished, JobQueue, Priority, QueueOptions};
use crate::{ManualJobs, Wake};

fn queue(lanes: usize) -> (JobQueue<&'static str, &'static str>, Arc<ManualJobs>, Arc<AtomicUsize>) {
    let jobs = Arc::new(ManualJobs::new());
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&wakes);
    let wake = Wake::new(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    let queue = JobQueue::new(jobs.clone(), wake, QueueOptions { lanes, name: "test" });
    (queue, jobs, wakes)
}

fn drain(queue: &JobQueue<&'static str, &'static str>) -> Vec<&'static str> {
    std::iter::from_fn(|| queue.try_recv()).map(|Finished { result, .. }| result).collect()
}

// With one lane over ManualJobs: Visible before Near before Background; newest first among equals.
#[test]
fn queue_runs_highest_priority_first() {
    let (queue, jobs, _) = queue(1);
    queue.submit("background", Priority::BACKGROUND, |_| "background");
    queue.submit("near", Priority::NEAR, |_| "near");
    queue.submit("old visible", Priority::VISIBLE, |_| "old visible");
    queue.submit("new visible", Priority::VISIBLE, |_| "new visible");
    assert_eq!(jobs.run_all(), 4);
    assert_eq!(drain(&queue), ["new visible", "old visible", "near", "background"]);
}

// A second submit for a waiting key runs only the second job; the first never runs.
#[test]
fn queue_replaces_queued_job_of_same_key() {
    let (queue, jobs, _) = queue(1);
    queue.submit("a", Priority::VISIBLE, |_| "first");
    queue.submit("a", Priority::VISIBLE, |_| "second");
    assert_eq!(queue.counts().queued, 1);
    jobs.run_all();
    assert_eq!(drain(&queue), ["second"]);
}

// With two lanes: a key's second job waits until its first ends, while another key's job runs.
#[test]
fn queue_runs_one_job_per_key() {
    let (queue, jobs, _) = queue(2);
    let queue = Arc::new(queue);
    let inner = Arc::clone(&queue);
    let ran_meanwhile = Arc::new(AtomicUsize::new(usize::MAX));
    let seen = Arc::clone(&ran_meanwhile);
    queue.submit("a", Priority::VISIBLE, move |_| {
        inner.submit("a", Priority::VISIBLE, |_| "a2");
        inner.submit("b", Priority::BACKGROUND, |_| "b");
        // Only "b" may start while "a" runs, although "a2" ranks higher.
        seen.store(inner.run_inline(5), Ordering::Relaxed);
        "a1"
    });
    jobs.run_next();
    assert_eq!(ran_meanwhile.load(Ordering::Relaxed), 1);
    assert!(queue.contains(&"a"));
    jobs.run_all();
    assert_eq!(drain(&queue), ["b", "a1", "a2"]);
}

// `reprioritize` returning None drops waiting jobs and sets Cancel on running ones; neither result arrives.
#[test]
fn queue_dropped_keys_are_cancelled() {
    let (queue, jobs, wakes) = queue(1);
    let queue = Arc::new(queue);
    queue.submit("waiting", Priority::BACKGROUND, |_| "waiting");
    let inner = Arc::clone(&queue);
    let saw_cancel = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&saw_cancel);
    queue.submit("running", Priority::VISIBLE, move |cancel: &Cancel| {
        let dropped = inner.reprioritize(|_, _| None);
        assert_eq!(dropped, ["waiting", "running"]);
        seen.store(usize::from(cancel.is_cancelled()), Ordering::Relaxed);
        "running"
    });
    jobs.run_all();
    assert_eq!(saw_cancel.load(Ordering::Relaxed), 1);
    assert!(drain(&queue).is_empty());
    assert_eq!(wakes.load(Ordering::Relaxed), 0);
    assert!(!queue.contains(&"waiting") && !queue.contains(&"running"));
}

// The wake is called once per finished job, never for a dropped or cancelled one.
#[test]
fn queue_wakes_after_each_finish() {
    let (queue, jobs, wakes) = queue(1);
    queue.submit("a", Priority::VISIBLE, |_| "a");
    queue.submit("b", Priority::NEAR, |_| "b");
    queue.submit("dropped", Priority::BACKGROUND, |_| "dropped");
    assert_eq!(queue.reprioritize(|k, p| (*k != "dropped").then_some(p)), ["dropped"]);
    assert_eq!(wakes.load(Ordering::Relaxed), 0);
    jobs.run_next();
    assert_eq!(wakes.load(Ordering::Relaxed), 1);
    jobs.run_all();
    assert_eq!(wakes.load(Ordering::Relaxed), 2);
    assert_eq!(queue.counts().finished, 2);
}

// With `lanes == 0` nothing runs on submit; `run_inline(1)` runs the best job and returns 1.
#[test]
fn queue_runs_inline_on_demand() {
    let (queue, jobs, wakes) = queue(0);
    queue.submit("low", Priority::BACKGROUND, |_| "low");
    queue.submit("high", Priority::VISIBLE, |_| "high");
    assert_eq!(jobs.waiting(), 0);
    assert_eq!(queue.run_inline(1), 1);
    assert_eq!(drain(&queue), ["high"]);
    assert_eq!(wakes.load(Ordering::Relaxed), 1);
    assert_eq!(queue.run_inline(5), 1);
    assert_eq!(queue.run_inline(5), 0);
    assert_eq!(drain(&queue), ["low"]);
}

// `shutdown` drops waiting jobs, cancels running ones, and returns true once they have ended.
#[test]
fn queue_shutdown_waits_for_running_jobs() {
    let jobs = Arc::new(crate::ThreadJobs);
    let queue = Arc::new(JobQueue::<&'static str, &'static str>::new(jobs, Wake::none(), QueueOptions { lanes: 1, name: "test" }));
    let started = Arc::new(AtomicUsize::new(0));
    let flag = Arc::clone(&started);
    queue.submit("running", Priority::VISIBLE, move |cancel: &Cancel| {
        flag.store(1, Ordering::Relaxed);
        while !cancel.is_cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        "running"
    });
    queue.submit("waiting", Priority::BACKGROUND, |_| "waiting");
    while started.load(Ordering::Relaxed) == 0 {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(queue.shutdown(Duration::from_secs(10)));
    assert_eq!(queue.counts().running, 0);
    assert!(!queue.contains(&"waiting"));
    assert!(queue.try_recv().is_none());
    queue.submit("late", Priority::VISIBLE, |_| "late");
    assert!(!queue.contains(&"late"));
}

// `ManualJobs::run_next` runs the oldest job only; a job spawned while it runs waits behind the others.
#[test]
fn manual_jobs_run_one_job_per_call() {
    use crate::Jobs;
    let jobs = Arc::new(ManualJobs::new());
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let (spawner, first_log) = (Arc::clone(&jobs), Arc::clone(&log));
    jobs.spawn(
        "a",
        Box::new(move || {
            first_log.lock().unwrap().push("a");
            let nested = Arc::clone(&first_log);
            spawner.spawn("c", Box::new(move || nested.lock().unwrap().push("c")));
        }),
    );
    let second_log = Arc::clone(&log);
    jobs.spawn("b", Box::new(move || second_log.lock().unwrap().push("b")));
    assert_eq!(jobs.waiting(), 2);
    assert!(jobs.run_next());
    assert_eq!(log.lock().unwrap().as_slice(), ["a"]);
    assert_eq!(jobs.waiting(), 2);
    assert_eq!(jobs.run_all(), 2);
    assert_eq!(log.lock().unwrap().as_slice(), ["a", "b", "c"]);
    assert!(!jobs.run_next());
}
