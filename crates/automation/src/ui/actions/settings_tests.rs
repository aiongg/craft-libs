//! The persisted Actions settings.

use super::ActionsSettings;

// `ActionsSettings` round-trips through JSON in camelCase and ignores PhotoCraft's `selected`,
// `selectedStep` and `expanded` fields.
#[test]
fn actions_settings_survive_a_serde_round_trip() {
    let settings = ActionsSettings { button_mode: true, collapsed_sets: vec!["Effects".into(), "Old".into()] };
    let text = serde_json::to_string(&settings).unwrap();
    assert_eq!(text, r#"{"buttonMode":true,"collapsedSets":["Effects","Old"]}"#);
    assert_eq!(serde_json::from_str::<ActionsSettings>(&text).unwrap(), settings);

    let photocraft = r#"{"buttonMode":true,"selected":"Sharpen","selectedStep":2,"expanded":["Sharpen"],"collapsedSets":["Effects"]}"#;
    let read: ActionsSettings = serde_json::from_str(photocraft).unwrap();
    assert_eq!(read, ActionsSettings { button_mode: true, collapsed_sets: vec!["Effects".into()] });
    assert_eq!(serde_json::from_str::<ActionsSettings>("{}").unwrap(), ActionsSettings::default());
}
