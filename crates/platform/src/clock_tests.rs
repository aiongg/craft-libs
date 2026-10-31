//! Behaviour tests of `clock`.
use super::*;

// A manual clock reads what the test set, moves only by `advance`, never backwards, and `set(None)` returns to real time.
#[test]
fn a_manual_clock_moves_only_when_the_test_moves_it() {
    let mut clock = Clock::manual(5.0);
    assert!(clock.is_manual());
    assert_eq!(clock.now(), 5.0);
    clock.advance(2.5);
    assert_eq!(clock.now(), 7.5);
    clock.advance(-1.0);
    clock.advance(f64::NAN);
    clock.advance(f64::INFINITY);
    assert_eq!(clock.now(), 7.5);
    clock.set(Some(100.0));
    assert_eq!(clock.now(), 100.0);
    clock.set(None);
    assert!(!clock.is_manual());
    assert!(clock.now() < 1.0);

    let mut real = Clock::new();
    real.advance(30.0);
    assert!(real.is_manual());
    assert!(real.now() >= 30.0);
}

// A real clock starts near 0, and `now` and `monotonic_ms` never decrease across calls.
#[test]
fn the_real_clock_counts_up_from_zero() {
    let clock = Clock::new();
    let first = clock.now();
    assert!((0.0..1.0).contains(&first));
    let (mut last, mut last_ms) = (first, monotonic_ms());
    for _ in 0..100 {
        let (now, ms) = (clock.now(), monotonic_ms());
        assert!(now >= last && ms >= last_ms);
        (last, last_ms) = (now, ms);
    }
}
