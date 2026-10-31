//! Behaviour tests of `clock` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A manual clock reads what the test set, moves only by `advance`, never backwards, and `set(None)` returns to real time.
#[test]
#[ignore = "skeleton"]
fn a_manual_clock_moves_only_when_the_test_moves_it() {
    todo!()
}

// A real clock starts near 0, and `now` and `monotonic_ms` never decrease across calls.
#[test]
#[ignore = "skeleton"]
fn the_real_clock_counts_up_from_zero() {
    todo!()
}
