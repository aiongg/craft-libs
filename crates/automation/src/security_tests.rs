//! Behaviour tests of `security` (skeleton stubs).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A connection's first line must be a matching `auth`; anything else is refused and closes.
#[test]
#[ignore = "skeleton"]
fn first_line_must_authenticate() {
    todo!()
}

// Tokens are 64 lower-case hex; a file token is trimmed; both supplied is a conflict.
#[test]
#[ignore = "skeleton"]
fn server_token_sources_and_validation() {
    todo!()
}

// A line over MAX_REQUEST_BYTES is TooLong and the rest is discarded.
#[test]
#[ignore = "skeleton"]
fn bounded_line_read_stops_at_the_limit() {
    todo!()
}
