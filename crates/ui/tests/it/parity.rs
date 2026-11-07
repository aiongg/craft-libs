//! The parity replay: a fixture story that reproduces a captured legacy story, replayed against its capture file.

use craft_ui::testing::layout::wrapped_text;
use craft_ui::testing::parity::{self, ParityState, Replay, line_diff};
use craft_ui::testing::trial;

#[derive(Clone)]
struct Text(String);

impl ParityState for Text {
    fn args(&self) -> Vec<(String, String)> {
        vec![("text".to_owned(), format!("Text({:?})", self.0))]
    }
}

const ID: &str = "composites/section-label/default";

#[trial]
fn replay_compares_a_story_with_its_capture_and_reports_the_differences_as_text() {
    let mut harness = parity::harness(Text("Blend mode".to_owned()), |ui, state| wrapped_text(ui, &state.0));
    let replay: Replay = parity::replay(ID, &mut harness);
    assert!(!replay.missing, "{replay}");
    // The legacy tree nests the label in generic containers the new shell does not add, so the replay differs; the
    // report names those lines and the shared script sections (Tab walk, drive) match.
    assert!(replay.differences.iter().any(|line| line.contains("Label")), "{replay}");
    assert!(replay.differences.iter().all(|line| !line.contains("drive:") && !line.contains("tab")), "{replay}");
    println!("{replay}");
}

#[trial]
fn replay_matching_runs_only_the_stories_the_track_has_migrated() {
    let replays = parity::replay_matching("section-label", |id| {
        (id == ID).then(|| parity::harness(Text("Blend mode".to_owned()), |ui, state| wrapped_text(ui, &state.0)))
    });
    assert_eq!(replays.len(), 1, "one story of the filter is migrated: {replays:?}");
    assert!(!parity::captured_ids("section-label").is_empty());
}

#[trial]
fn line_diff_reports_removed_and_added_lines_in_order() {
    assert_eq!(line_diff("a\nb\nc\n", "a\nx\nc\n"), ["- b", "+ x"]);
    assert!(line_diff("a\n", "a\n").is_empty());
}
