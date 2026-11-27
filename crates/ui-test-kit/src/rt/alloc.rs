//! A counting global allocator. A test or bench binary installs it with
//! `#[global_allocator] static A: CountingAlloc = CountingAlloc;` and reads deltas around the
//! sections it times.

#![allow(unsafe_code, reason = "implementing `GlobalAlloc` is unsafe; every method forwards to `System`")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::backtrace::Backtrace;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

use web_time::Instant;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static RECORDING: AtomicBool = AtomicBool::new(false);
static SITES: Mutex<Vec<Backtrace>> = Mutex::new(Vec::new());

thread_local! {
    /// Set while this thread captures a backtrace, whose own allocations are not recorded.
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
}

fn count(size: usize) {
    ALLOCS.fetch_add(1, Relaxed);
    BYTES.fetch_add(size as u64, Relaxed);
    if RECORDING.load(Relaxed) {
        record();
    }
}

fn record() {
    let _ = CAPTURING.try_with(|busy| {
        if !busy.replace(true) {
            let trace = Backtrace::force_capture();
            if let Ok(mut sites) = SITES.lock() {
                sites.push(trace);
            }
            busy.set(false);
        }
    });
}

/// Starts or stops keeping a backtrace for every allocation (slow; for profiling a few frames).
pub fn record_callers(on: bool) {
    RECORDING.store(on, Relaxed);
}

/// The backtraces kept since the last call, grouped by call site and summarised as a table of
/// allocations per frame (`frames` is the number of frames recorded). A site is the frame just
/// above the allocator plus the first three frames in `crates` (name prefixes).
pub fn take_callers(frames: usize, crates: &[&str]) -> String {
    let traces = std::mem::take(&mut *SITES.lock().unwrap());
    let mut sites: HashMap<String, u64> = HashMap::new();
    for trace in &traces {
        let text = trace.to_string();
        let mut frames: Vec<(&str, &str)> = Vec::new();
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(loc) = t.strip_prefix("at ") {
                if let Some(last) = frames.last_mut() {
                    last.1 = loc;
                }
            } else if let Some((n, name)) = t.split_once(": ").filter(|(n, _)| n.bytes().all(|b| b.is_ascii_digit())) {
                let _ = n;
                frames.push((name, ""));
            }
        }
        let machinery = ["rt/alloc.rs", "backtrace", "std/src/alloc.rs", "alloc/src/alloc.rs", "core/src/alloc"];
        let skip = frames.iter().position(|(_, loc)| !machinery.iter().any(|m| loc.contains(m)) && !loc.is_empty()).unwrap_or(0);
        let rest = &frames[skip..];
        let show =
            |(n, l): &(&str, &str)| format!("{n} [{}]", l.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/"));
        let mut key = rest.first().map(show).unwrap_or_default();
        for f in rest.iter().skip(1).filter(|(n, l)| crates.iter().any(|c| n.contains(c) || l.contains(c))).take(3) {
            let _ = write!(key, "\n      <- {}", show(f));
        }
        *sites.entry(key).or_default() += 1;
    }
    let mut rows: Vec<_> = sites.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = format!("{} allocations over {frames} frames = {:.1} per frame\n", traces.len(), traces.len() as f64 / frames.max(1) as f64);
    for (key, n) in rows {
        let _ = writeln!(out, "{:>8.2}  {key}", n as f64 / frames.max(1) as f64);
    }
    out
}

/// Counts allocations and requested bytes, then forwards to the system allocator.
pub struct CountingAlloc;

// SAFETY: every call forwards unchanged to `System`, which upholds the `GlobalAlloc` contract.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Totals since process start: (allocation count, bytes requested).
pub fn snapshot() -> (u64, u64) {
    (ALLOCS.load(Relaxed), BYTES.load(Relaxed))
}

/// Whether the counting allocator is the process's global allocator (it has served an allocation).
pub fn installed() -> bool {
    ALLOCS.load(Relaxed) > 0
}

/// What one timed section cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SectionCost {
    /// Wall time in milliseconds.
    pub ms: f64,
    /// Allocations made.
    pub allocs: u64,
    /// Bytes requested.
    pub bytes: u64,
}

/// Runs `f` and returns its result with its cost.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, SectionCost) {
    let start = Instant::now();
    let (a0, b0) = snapshot();
    let r = f();
    let (a1, b1) = snapshot();
    (r, SectionCost { ms: start.elapsed().as_secs_f64() * 1000.0, allocs: a1 - a0, bytes: b1 - b0 })
}
