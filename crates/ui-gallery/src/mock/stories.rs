//! The story of the mock editor window.

use super::{EditorMock, MockState};
use craft_ui::testing::{StoryData, StoryOutput, story};
use egui::Ui;

#[story(EditorMock<'static>, covers = [])]
fn editor_window(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let state = data.value("mock", MockState::default());
    EditorMock::new("Editor", state).show(ui).into()
}
