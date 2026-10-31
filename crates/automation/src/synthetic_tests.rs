//! Behaviour tests of `synthetic` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A two-click ui.click injects move+press+release on one pass and press+release on the next; nothing more is injected until pass_finished; MAX_CLICKS bounds count.
#[test]
#[ignore = "skeleton"]
fn click_steps_land_on_separate_passes() {
    todo!()
}

// A deferred pass gets its step appended again once; a changed target withdraws the step from the input and clears the queue.
#[test]
#[ignore = "skeleton"]
fn in_flight_steps_are_reinjected_or_withdrawn() {
    todo!()
}
