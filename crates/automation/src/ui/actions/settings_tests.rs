//! The persisted Actions settings.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `ActionsSettings` round-trips through JSON in camelCase and ignores PhotoCraft's `selected`,
// `selectedStep` and `expanded` fields.
#[test]
#[ignore = "skeleton"]
fn actions_settings_survive_a_serde_round_trip() {
    todo!()
}
