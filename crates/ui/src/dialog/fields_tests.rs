//! The field map's snapshot, reset and remembered-value filtering.

// After `snapshot`, edits then `reset` bring every value back and report the change; `changed`
// is false again.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn reset_restores_the_opening_values() {
    todo!()
}

// A remembered map with a renamed key, an out-of-range number and a choice no longer offered
// keeps only the fitting values.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn remembered_values_that_no_longer_fit_are_dropped() {
    todo!()
}

// `int` reads `3.0` as 3 and `number` reads `Int(3)` as 3.0, as a script writes either.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn numbers_and_ints_read_each_other() {
    todo!()
}

// The map round-trips through TOML with every value kind, for `Remembered`.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_map_round_trips_through_toml() {
    todo!()
}
