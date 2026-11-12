//! The process's memory use, for a status readout ("Scratch: 1.2G/8.0G") and the Efficiency item,
//! and the machine's physical memory, for memory budgets (GPU residency, image caches).

/// One sample of memory use, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemorySample {
    /// The process's resident memory.
    pub used: u64,
    /// Physical memory the process may still use: the system's available memory plus `used`.
    pub available: u64,
}

/// Samples memory use now; `None` where the platform gives no reading (the web) or the reading
/// fails. Linux reads `/proc`; macOS and Windows ask `sysinfo`. Cheap enough to call once a
/// second, not every frame.
// new: PhotoCraft computes document sizes only (panels/status_bar.rs:69) and samples no process
// memory. Linux reads `VmRSS` of `/proc/self/status` and `MemAvailable` of `/proc/meminfo`, both
// in kibibytes, which needs no page size. `sysinfo` reads the available memory exactly.
pub fn sample() -> Option<MemorySample> {
    #[cfg(target_os = "linux")]
    {
        let used = kibibytes(&std::fs::read_to_string("/proc/self/status").ok()?, "VmRSS:")?;
        let free = kibibytes(&std::fs::read_to_string("/proc/meminfo").ok()?, "MemAvailable:")?;
        Some(MemorySample { used, available: free.saturating_add(used) })
    }
    #[cfg(any(target_os = "macos", windows))]
    {
        use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
        let pid = sysinfo::get_current_pid().ok()?;
        let mut system = System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()));
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_memory());
        let used = system.process(pid)?.memory();
        Some(MemorySample { used, available: system.available_memory().saturating_add(used) })
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}

/// The bytes of the `key` line of a `/proc` file ("VmRSS:   1234 kB").
#[cfg(target_os = "linux")]
fn kibibytes(text: &str, key: &str) -> Option<u64> {
    let rest = text.lines().find_map(|line| line.strip_prefix(key))?;
    let mut parts = rest.split_whitespace();
    let n: u64 = parts.next()?.parse().ok()?;
    match parts.next() {
        Some("kB") => n.checked_mul(1024),
        _ => None,
    }
}

/// The variable that overrides [`physical_memory`], in MiB, to simulate a smaller machine.
pub const RAM_MB: &str = "CRAFT_RAM_MB";

/// Physical memory of this machine in bytes, if known; [`RAM_MB`] overrides it. Read once per
/// process. `None` where nothing reports it (the web, other platforms).
pub fn physical_memory() -> Option<u64> {
    static RAM: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    *RAM.get_or_init(|| {
        if let Some(mb) = std::env::var(RAM_MB).ok().and_then(|v| v.trim().parse::<u64>().ok()) {
            return mb.checked_mul(1 << 20);
        }
        detect_physical_memory()
    })
}

/// Physical memory as the operating system reports it: `MemTotal` of `/proc/meminfo` on Linux,
/// `sysinfo` on macOS and Windows, `hw.physmem` on FreeBSD.
fn detect_physical_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        kibibytes(&std::fs::read_to_string("/proc/meminfo").ok()?, "MemTotal:")
    }
    #[cfg(any(target_os = "macos", windows))]
    {
        use sysinfo::{MemoryRefreshKind, RefreshKind, System};
        let system = System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()));
        Some(system.total_memory()).filter(|b| *b > 0)
    }
    #[cfg(target_os = "freebsd")]
    {
        let out = std::process::Command::new("/sbin/sysctl").args(["-n", "hw.physmem"]).output().ok()?;
        String::from_utf8(out.stdout).ok()?.trim().parse().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows, target_os = "freebsd")))]
    {
        None
    }
}
