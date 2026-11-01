//! The persisted scope settings.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `ScopeSettings` round-trips through JSON in camelCase; an unknown field is ignored and a missing
// one takes its default.
#[test]
#[ignore = "skeleton"]
fn scope_settings_survive_a_serde_round_trip() {
    todo!()
}
