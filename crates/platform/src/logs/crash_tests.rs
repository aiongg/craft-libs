//! Behaviour tests of `logs::crash` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A panic on a named worker thread is kept for take_last with its message and thread, and appended to the crash log.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "skeleton"]
fn a_worker_panic_is_kept_and_written_to_the_crash_log() {
    todo!()
}

// catch returns the value, or Panicked with the message when f panics.
#[test]
#[ignore = "skeleton"]
fn catch_turns_a_panic_into_an_error() {
    todo!()
}
