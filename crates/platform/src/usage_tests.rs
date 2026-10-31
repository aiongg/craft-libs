//! Memory use.

use crate::usage::{MemorySample, sample};

#[test]
#[cfg(target_os = "linux")]
// On a desktop platform sample() is Some with 0 < used <= available.
fn sample_reports_used_below_available() {
    let MemorySample { used, available } = sample().expect("Linux reports memory use");
    assert!(0 < used && used <= available, "used {used}, available {available}");
}
