//! A counting global allocator. A test or bench binary installs it with
//! `#[global_allocator] static A: CountingAlloc = CountingAlloc;` and reads deltas around the
//! sections it times.

#![allow(unsafe_code, reason = "implementing `GlobalAlloc` is unsafe; every method forwards to `System`")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use web_time::Instant;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// Counts allocations and requested bytes, then forwards to the system allocator.
pub struct CountingAlloc;

// SAFETY: every call forwards unchanged to `System`, which upholds the `GlobalAlloc` contract.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(layout.size() as u64, Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(layout.size() as u64, Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(new_size as u64, Relaxed);
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
