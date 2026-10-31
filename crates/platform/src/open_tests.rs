//! `craft_platform::open`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// "img2" sorts before "img10", "IMG3" between them ignoring case, "a01" beside "a1" by length.
#[test]
#[ignore = "skeleton"]
fn natural_order_compares_digit_runs_by_value() {
    todo!()
}

// Equal paths match without touching the disk; a symlink matches its target through the
// canonical path.
#[test]
#[ignore = "skeleton"]
fn same_file_resolves_only_when_needed() {
    todo!()
}

// OS open requests from another thread and drops arrive in order, Open batches sorted naturally,
// and the wake callback runs once per request.
#[test]
#[ignore = "skeleton"]
fn intake_keeps_arrival_order_and_wakes() {
    todo!()
}
