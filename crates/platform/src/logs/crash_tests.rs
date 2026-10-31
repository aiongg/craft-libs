//! Behaviour tests of `logs::crash`.
use super::*;
use std::sync::Mutex;

/// The panic hook and the last-panic slot are process-wide: tests that panic run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

// A panic on a named worker thread is kept for take_last with its message and thread, and appended to the crash log.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_worker_panic_is_kept_and_written_to_the_crash_log() {
    let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = std::env::temp_dir().join(format!("craft-crash-{}", std::process::id()));
    install("CrashTest", "1.2.3", Some(dir.clone()));
    let _ = take_last();
    let joined = std::thread::Builder::new().name("crash-test-worker".into()).spawn(|| panic!("boom in a worker")).expect("spawn").join();
    assert!(joined.is_err());
    let last = take_last().expect("the hook kept the panic");
    assert!(last.contains("boom in a worker") && last.contains("thread crash-test-worker"), "{last}");
    assert!(take_last().is_none(), "take_last empties the slot");
    let path = log_path().expect("crash log path");
    assert_eq!(path.parent(), Some(dir.as_path()));
    let log = std::fs::read_to_string(&path).expect("crash log");
    assert!(log.contains("=== CrashTest 1.2.3 (unix ") && log.contains("boom in a worker"), "{log}");
    record("window could not be created");
    assert!(std::fs::read_to_string(&path).expect("crash log").contains("window could not be created"));
    let _ = std::fs::remove_dir_all(dir);
}

// catch returns the value, or Panicked with the message when f panics.
#[test]
fn catch_turns_a_panic_into_an_error() {
    let _serial = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(catch(|| 3), Ok(3));
    let err = catch(|| -> () { panic!("boom") }).expect_err("panicked");
    assert_eq!(err, Panicked { message: "boom".into() });
    assert_eq!(err.to_string(), "boom");
    let err = catch(|| -> () { std::panic::panic_any(7_u8) }).expect_err("panicked");
    assert_eq!(err.message, "panic");
    let _ = take_last();
}
