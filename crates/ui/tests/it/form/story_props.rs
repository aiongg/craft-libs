//! The Properties form of a story edits the values the story reads, and a restore puts them back.

use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, story_props, trial};
use egui::Ui;

fn show(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    data.value("on", false);
    data.choice("mode", 0u8, &[("Normal", 0), ("Screen", 2)]);
    story_props(ui, data);
    StoryOutput::default()
}

/// A story that read its values only at start would ignore the form; the next pass must see the edit, and a restore
/// must bring the start values back.
#[trial]
fn editing_a_story_value_changes_what_the_story_reads_and_restore_undoes_it() {
    let mut fixture = Fixture::story(&Story::new("props", show));
    fixture.settle();
    let start = fixture.data().snapshot();
    let check = fixture.by_label("On").expect("a check box for the bool");
    fixture.click(&check);
    fixture.settle();
    assert_eq!(fixture.data().changed_since(&start), ["on"], "the check box wrote the story's value");
    let select = fixture.by_label("Mode").expect("a select for the choice");
    fixture.click(&select);
    fixture.settle();
    let row = fixture.by_label("Screen").expect("the choice's rows");
    fixture.click(&row);
    fixture.settle();
    assert_eq!(fixture.data().changed_since(&start), ["mode", "on"], "the pick wrote the choice's value");
    fixture.data().restore(&start);
    fixture.settle();
    assert!(fixture.data().changed_since(&start).is_empty(), "restore brings every value back to its start");
}
