//! The frame server.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cache::FrameCacheKey;
use crate::cache_tests::{TestFrame, TestKey};
use crate::frames::{CancelToken, FrameRenderer, FrameServer, PlaybackRequest, playback_plan, prefetch_depth};

struct Recorder {
    rendered: Mutex<Vec<(u8, i64)>>,
    ready: AtomicBool,
}

impl FrameRenderer for Recorder {
    type Key = TestKey;
    type Frame = TestFrame;
    type Job = ();

    fn render(&self, key: &TestKey, _: &(), _: &CancelToken) -> Option<TestFrame> {
        if !self.ready.load(Ordering::Relaxed) {
            return None;
        }
        self.rendered.lock().ok()?.push((key.series, key.frame));
        Some(TestFrame(10))
    }
}

/// A server without workers, so `pump` runs the jobs.
fn server() -> (FrameServer<Recorder>, Arc<Recorder>) {
    let renderer = Arc::new(Recorder { rendered: Mutex::new(Vec::new()), ready: AtomicBool::new(true) });
    (FrameServer::new(renderer.clone(), 0, 1 << 20), renderer)
}

fn rendered(renderer: &Recorder) -> Vec<i64> {
    renderer.rendered.lock().map(|r| r.iter().map(|(_, f)| *f).collect()).unwrap_or_default()
}

// (1, 1) while workers keep up, else lead and stride as FilmCraft.
#[test]
fn playback_plan_drops_frames_evenly_when_slow() {
    assert_eq!(playback_plan(0.0, 24.0, 6), (1, 1));
    assert_eq!(playback_plan(0.15, 24.0, 6), (1, 1));
    assert_eq!(playback_plan(0.5, 24.0, 6), (12, 3));
    assert_eq!(playback_plan(0.5, 24.0, 0), playback_plan(0.5, 24.0, 1));
}

/// A faster speed prefetches at least as deep, in either direction.
#[test]
fn prefetch_depth_by_speed() {
    assert!(prefetch_depth(2.0) >= prefetch_depth(1.0));
    assert_eq!(prefetch_depth(2.0), prefetch_depth(-2.0));
}

// a priority-0 request drops this view's queued priority-0 jobs.
#[test]
fn an_on_screen_request_replaces_older_ones() {
    let (frames, renderer) = server();
    frames.request(TestKey::new(0, 1), (), 0);
    frames.request(TestKey::new(0, 2), (), 0);
    frames.request(TestKey::new(0, 3), (), 1);
    frames.request(TestKey::new(1, 4), (), 0);
    frames.request(TestKey::new(0, 5), (), 0);
    assert_eq!(frames.queue_len(), 3);
    assert_eq!(frames.pump(Duration::from_secs(5)), 3);
    assert_eq!(rendered(&renderer), [4, 5, 3]);
}

// jobs behind the playhead or past the span are dropped.
#[test]
fn schedule_playback_drops_stale_jobs() {
    let (frames, renderer) = server();
    frames.request(TestKey::new(0, 100), (), 5);
    frames.request(TestKey::new(0, 500), (), 5);
    let request = PlaybackRequest { key: TestKey::new(0, 200), fps: 24.0, speed: 1.0, preroll: false, last: 1000 };
    frames.schedule_playback(&request, |_| ());
    assert_eq!(frames.queue_len(), 15);
    frames.pump(Duration::from_secs(5));
    assert_eq!(rendered(&renderer), (200..=214).collect::<Vec<_>>());
}

// preroll_ready is true once the next six frames are cached.
#[test]
fn preroll_ready_needs_six_frames() {
    let (frames, _) = server();
    let key = TestKey::new(0, 10);
    for f in 10..15 {
        frames.request(key.at_frame(f), (), 1);
    }
    frames.pump(Duration::from_secs(5));
    assert!(!frames.preroll_ready(&key, 1.0, 100));
    assert!(frames.preroll_ready(&key, 1.0, 14), "frames past the last are not needed");
    frames.request(key.at_frame(15), (), 1);
    frames.pump(Duration::from_secs(5));
    assert!(frames.preroll_ready(&key, 1.0, 100));
}

// with zero workers pump renders queued jobs by priority within the budget.
#[test]
fn pump_runs_jobs_without_workers() {
    let (frames, renderer) = server();
    for (frame, priority) in [(1, 3), (2, 1), (3, 2)] {
        frames.request(TestKey::new(0, frame), (), priority);
    }
    assert_eq!(frames.pump(Duration::ZERO), 1, "at least one job runs");
    assert_eq!(frames.pump(Duration::from_secs(5)), 2);
    assert_eq!(rendered(&renderer), [2, 3, 1]);

    renderer.ready.store(false, Ordering::Relaxed);
    frames.request(TestKey::new(0, 9), (), 0);
    assert_eq!(frames.pump(Duration::from_secs(5)), 0);
    assert_eq!(frames.queue_len(), 1, "a job whose source is not ready goes back");
    renderer.ready.store(true, Ordering::Relaxed);
    assert_eq!(frames.pump(Duration::from_secs(5)), 1);
    assert!(frames.get(&TestKey::new(0, 9)).is_some());
}

/// Renders on whatever thread calls it and records that thread's name.
struct Threads {
    threads: Mutex<Vec<(i64, String)>>,
}

impl FrameRenderer for Threads {
    type Key = TestKey;
    type Frame = TestFrame;
    type Job = ();

    fn render(&self, key: &TestKey, _: &(), _: &CancelToken) -> Option<TestFrame> {
        let name = std::thread::current().name().unwrap_or_default().to_owned();
        self.threads.lock().ok()?.push((key.frame, name));
        Some(TestFrame(10))
    }
}

// worker threads render every request once, off the calling thread, and wake the UI.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn worker_threads_render_requests_and_repaint() {
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    let renderer = Arc::new(Threads { threads: Mutex::new(Vec::new()) });
    let frames = FrameServer::new(renderer.clone(), 2, 1 << 20);
    assert_eq!(frames.workers(), 2);
    let repaints = Arc::new(AtomicUsize::new(0));
    let seen = repaints.clone();
    frames.set_repaint(move || {
        seen.fetch_add(1, Ordering::Relaxed);
    });
    for frame in 1..=6 {
        frames.request(TestKey::new(0, frame), (), 3);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while (1..=6).any(|frame| frames.get(&TestKey::new(0, frame)).is_none()) {
        assert!(Instant::now() < deadline, "the workers rendered {:?}", renderer.threads.lock().map(|r| r.len()));
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut done = renderer.threads.lock().map(|r| r.clone()).unwrap_or_default();
    done.sort();
    assert_eq!(done.iter().map(|(frame, _)| *frame).collect::<Vec<_>>(), (1..=6).collect::<Vec<_>>(), "each frame renders once");
    assert!(done.iter().all(|(_, name)| name.starts_with("frames-")), "{done:?}");
    assert!(repaints.load(Ordering::Relaxed) >= 1, "a finished frame asks for a repaint");
    assert_eq!(frames.queue_len(), 0);
}
