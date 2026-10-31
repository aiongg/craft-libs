//! The process's memory use, for a status readout ("Scratch: 1.2G/8.0G") and the Efficiency item.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

/// One sample of memory use, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemorySample {
    /// The process's resident memory.
    pub used: u64,
    /// Physical memory the process may still use: the system's available memory plus `used`.
    pub available: u64,
}

/// Samples memory use now; `None` where the platform gives no reading (the web). Cheap enough to
/// call once a second, not every frame.
// new: PhotoCraft computes document sizes only (panels/status_bar.rs:69) and samples no process
// memory. The workspace forbids `unsafe`, so the macOS (`task_info`) and Windows
// (`GetProcessMemoryInfo`) readings need a crate with a safe API; Linux reads `/proc/self/statm`
// and `/proc/meminfo`. The dependency is a lead call in specs/step3-shell.md.
pub fn sample() -> Option<MemorySample> {
    todo!()
}
