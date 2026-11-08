//! Stories of the reading view.

use craft_ui::contract::Show as _;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{ReadingNav, ReadingState, ReadingView};
use crate::ui::fixtures::DemoPages;

const PAGES: usize = 6;

/// Two columns of the document, the second at the leading edge after one turn.
#[story(ReadingView<'static>, covers = [Default], drive = Drive::Never)]
fn reading_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let source = DemoPages::new(PAGES);
    let state = data.value("state", ReadingState::default());
    let rect = ui.available_rect_before_wrap();
    ReadingView::new("Document", &source, state).show_at(ui, rect).into()
}

#[story(ReadingNav<'static>, covers = [], drive = Drive::Never)]
fn reading_nav(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let state = data.value("state", ReadingState { column: 1, ..ReadingState::default() });
    ReadingNav::new(state, PAGES).show(ui).into()
}
