//! The parity replay: a fixture story that reproduces a captured legacy story, replayed against its capture file.

use craft_ui::testing::layout::wrapped_text;
use craft_ui::testing::parity::{self, ParityState, Replay, line_diff};
use craft_ui::testing::{Story, StoryData, StoryOutput, trial};

#[derive(Clone)]
struct Text(String);

impl ParityState for Text {
    fn args(&self) -> Vec<(String, String)> {
        vec![("text".to_owned(), format!("Text({:?})", self.0))]
    }
}

const ID: &str = "composites/section-label/default";

fn section_label(text: &str) -> parity::Scene<Text> {
    parity::scene(Text(text.to_owned()), |ui, state| wrapped_text(ui, &state.0))
}

/// A story shown with an arg the capture does not have differs from the capture in that arg, and the report says
/// which line each side has; the report is text and the replay does not fail on the difference.
#[trial]
fn replay_reports_a_known_difference_from_its_capture_as_text() {
    let replay: Replay = parity::replay(ID, &mut section_label("Other text"));
    assert!(!replay.missing, "{replay}");
    assert!(!replay.identical(), "{replay}");
    assert!(replay.differences.iter().any(|line| line.starts_with("- ") && line.contains("Blend mode")), "{replay}");
    assert!(replay.differences.iter().any(|line| line.starts_with("+ ") && line.contains("Other text")), "{replay}");
    assert!(replay.to_string().contains("differing lines"), "{replay}");
}

#[trial]
fn replay_matching_runs_only_the_stories_the_track_has_migrated() {
    let replays = parity::replay_matching("section-label", |id| (id == ID).then(|| section_label("Blend mode")));
    assert_eq!(replays.len(), 1, "one story of the filter is migrated: {replays:?}");
    assert!(!parity::captured_ids("section-label").is_empty());
}

#[trial]
fn line_diff_reports_removed_and_added_lines_in_order() {
    assert_eq!(line_diff("a\nb\nc\n", "a\nx\nc\n"), ["- b", "+ x"]);
    assert!(line_diff("a\n", "a\n").is_empty());
}

#[derive(Clone, Debug, PartialEq)]
struct Words(String);

fn show_words(ui: &mut egui::Ui, data: &mut StoryData) -> StoryOutput {
    let words = data.shown("text", Words("Other text".to_owned())).0.clone();
    wrapped_text(ui, &words);
    StoryOutput::default()
}

/// A registered story replays without a second scene: its args are its `shown` values, so the difference from the
/// capture names the arg's Debug text.
#[trial]
fn replay_story_reads_the_args_from_the_stories_shown_values() {
    let story = Story::new("words", show_words);
    let replay = parity::replay_story(ID, &story);
    assert!(!replay.missing, "{replay}");
    assert!(replay.differences.iter().any(|line| line.starts_with("- ") && line.contains(r#"Text("Blend mode")"#)), "{replay}");
    assert!(replay.differences.iter().any(|line| line.starts_with("+ ") && line.contains(r#"Words("Other text")"#)), "{replay}");
}
