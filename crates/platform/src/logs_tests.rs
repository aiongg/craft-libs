//! Behaviour tests of `logs` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The default filter shows the app's info and other crates' warnings; directives follow env_logger, the most specific wins, hostile specs never panic.
#[test]
#[ignore = "skeleton"]
fn filter_directives_follow_env_logger() {
    todo!()
}

// Timestamps are UTC with milliseconds, the epoch before 1970, and full years past 9999.
#[test]
#[ignore = "skeleton"]
fn timestamps_are_utc_with_milliseconds() {
    todo!()
}

// Each start shifts app.log to app.1.log and app.2.log; while another instance holds the owner lock, a start writes app-<pid>.log and leaves the chain alone.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn rotation_keeps_previous_logs_and_a_running_instance_log() {
    todo!()
}

// Lines before the file opens are written on attach, at most MAX_PENDING with a dropped-count line; the file stops at its cap with one note.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn the_sink_keeps_early_lines_and_stops_at_its_cap() {
    todo!()
}

// The logger writes only records its filter passes, as whole lines from many threads, and keeps logging to stderr when the directory is unusable.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn the_logger_filters_and_writes_whole_lines() {
    todo!()
}
