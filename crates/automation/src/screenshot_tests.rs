//! Behaviour tests of `screenshot` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// More than MAX_PENDING or over the byte budgets is refused with the reply sender returned.
#[test]
#[ignore = "skeleton"]
fn admission_budget_refuses_excess() {
    todo!()
}

// A geometry change between issue and collection retries up to MAX_RETRIES, then fails.
#[test]
#[ignore = "skeleton"]
fn geometry_change_in_flight_retries() {
    todo!()
}

// A request before the first geometry waits and is issued once geometry is known.
#[test]
#[ignore = "skeleton"]
fn early_request_waits_for_geometry() {
    todo!()
}
