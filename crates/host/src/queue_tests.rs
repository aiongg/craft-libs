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

// With `lanes == 0` nothing runs on submit; `run_inline(1)` runs the best job and returns 1.

// A key's next job that waited for its running inline job gets a lane when the inline run ends.
#[test]
fn queue_inline_run_spawns_a_lane_for_the_keys_next_job() {
    let (queue, jobs, _) = queue(2);
    let queue = Arc::new(queue);
    let inner = Arc::clone(&queue);
    queue.submit("a", Priority::VISIBLE, move |_| {
        inner.submit("a", Priority::VISIBLE, |_| "second");
        "first"
    });
    assert_eq!(jobs.waiting(), 1);
    assert_eq!(queue.run_inline(1), 1);
    assert_eq!(jobs.waiting(), 2);
    jobs.run_all();
    assert_eq!(drain(&queue), ["first", "second"]);
}

// A `Jobs` that drops the lane unrun (a thread that could not start) does not leave the lane counted.
#[test]
fn queue_gives_back_a_lane_the_host_dropped() {
    use crate::Jobs;
    struct Refusing(AtomicUsize, ManualJobs);
    impl Jobs for Refusing {
        fn spawn(&self, name: &str, job: Box<dyn FnOnce() + Send>) {
            if self.0.load(Ordering::Relaxed) == 0 {
                drop(job);
            } else {
                self.1.spawn(name, job);
            }
        }
    }
    let jobs = Arc::new(Refusing(AtomicUsize::new(0), ManualJobs::new()));
    let queue = JobQueue::<&'static str, &'static str>::new(jobs.clone(), Wake::new(|| {}), QueueOptions { lanes: 1, name: "test" });
    queue.submit("a", Priority::VISIBLE, |_| "a");
    jobs.0.store(1, Ordering::Relaxed);
    queue.submit("b", Priority::VISIBLE, |_| "b");
    assert_eq!(jobs.1.waiting(), 1);
    jobs.1.run_all();
    assert_eq!(drain(&queue), ["b", "a"]);
}

// A job left waiting by a dropped lane gets a lane from `resume`, and `resume` adds none while a lane is alive.
#[test]
fn queue_resume_starts_a_lane_only_when_none_is_alive() {
    use crate::Jobs;
    struct Refusing(AtomicUsize, ManualJobs);
    impl Jobs for Refusing {
        fn spawn(&self, name: &str, job: Box<dyn FnOnce() + Send>) {
            if self.0.load(Ordering::Relaxed) == 0 {
                drop(job);
            } else {
                self.1.spawn(name, job);
            }
        }
    }
    let jobs = Arc::new(Refusing(AtomicUsize::new(0), ManualJobs::new()));
    let queue = JobQueue::<&'static str, &'static str>::new(jobs.clone(), Wake::new(|| {}), QueueOptions { lanes: 2, name: "test" });
    queue.submit("a", Priority::VISIBLE, |_| "a");
    jobs.0.store(1, Ordering::Relaxed);
    queue.resume();
    queue.resume();
    assert_eq!(jobs.1.waiting(), 1, "one lane for the waiting job");
    jobs.1.run_all();
    assert_eq!(drain(&queue), ["a"]);
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
