//! The persisted scope settings.

use super::{ScopeSettings, ScopeView};

// `ScopeSettings` round-trips through JSON in camelCase; an unknown field is ignored and a missing
// one takes its default.
#[test]
fn scope_settings_survive_a_serde_round_trip() {
    let settings = ScopeSettings {
        view: ScopeView::AllChannels,
        channel: "red".into(),
        show_channels_in_color: true,
        show_statistics: false,
        source: "layer".into(),
        lab_readouts: true,
        vectorscope: true,
        selected_region: true,
        red_right: true,
        skin_tone: false,
        shadow_warning: true,
        highlight_warning: true,
    };
    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!(json["view"], "allChannels");
    assert_eq!(json["showChannelsInColor"], true);
    assert_eq!(json["redRight"], true);
    assert_eq!(serde_json::from_value::<ScopeSettings>(json.clone()).unwrap(), settings);

    let mut extra = json;
    extra["fromANewerVersion"] = serde_json::json!(7);
    assert_eq!(serde_json::from_value::<ScopeSettings>(extra).unwrap(), settings);

    let partial: ScopeSettings = serde_json::from_str(r#"{"view":"expanded","redRight":true}"#).unwrap();
    assert_eq!(partial, ScopeSettings { view: ScopeView::Expanded, red_right: true, ..ScopeSettings::default() });
    assert!(partial.show_statistics && partial.skin_tone, "the defaults that are on stay on");
}
