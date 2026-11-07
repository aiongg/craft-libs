//! The settings notices: which messages the failure count calls for.

use craft_theme::variant::Tone;
use craft_ui::notices::{SaveResult, SettingsLoad, SettingsNoticeState, SettingsStatus};
use craft_ui::testing::trial;

fn failed() -> SaveResult {
    SaveResult::Failed("disk full".to_owned())
}

/// The first failure speaks in the status line, the third adds the notice, and the next success clears it and says so.
#[trial]
fn repeated_save_failures_escalate_and_a_success_clears_them() {
    let mut state = SettingsNoticeState::new();
    state.ticked(&failed());
    assert_eq!(state.take_status(), Some(SettingsStatus::SaveFailed("disk full".to_owned())), "the first failure reaches the status line");
    assert!(!state.has_notice(), "one failure shows no notice");
    state.ticked(&failed());
    assert_eq!(state.take_status(), None, "a second failure adds no status");
    state.ticked(&failed());
    assert!(state.has_notice(), "the third failure in a row shows the notice");
    state.ticked(&SaveResult::Saved);
    assert!(!state.has_notice(), "a success removes the notice");
    let status = state.take_status().expect("a success after failures says so");
    assert_eq!(status.tone(), Tone::Good);
}

/// A success with no failure before it is silent, and stored settings that were reset show their notice until a save.
#[trial]
fn a_plain_success_is_silent_and_a_save_clears_the_reset_notice() {
    let mut state = SettingsNoticeState::new();
    state.loaded(&SettingsLoad::new().reset(vec!["tools.brushSize".to_owned()]).moved_aside("prefs.bad"));
    assert!(state.has_notice(), "reset settings show a notice");
    assert_eq!(state.take_status(), Some(SettingsStatus::MovedAside("prefs.bad".to_owned())));
    state.ticked(&SaveResult::Saved);
    assert!(!state.has_notice(), "the save wrote the defaults");
    assert_eq!(state.take_status(), None, "no failure came first, so there is nothing to report");
}
