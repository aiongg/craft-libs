//! Stories of the chord recorder.

use super::{ChordRecorder, IdleField, RecordingField};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(IdleField, covers = [], drive = Drive::Never)]
fn chord_recorder(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ChordRecorder::new("Shortcut", false, "Cmd+K").id_salt("recorder").show(ui).response.into()
}

#[story(RecordingField, covers = [], drive = Drive::Never)]
fn chord_recorder_recording(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ChordRecorder::new("Shortcut", true, "press keys").id_salt("recording").show(ui).response.into()
}
