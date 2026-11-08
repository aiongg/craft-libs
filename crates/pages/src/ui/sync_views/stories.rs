//! Stories of the synced views.

use super::{Lead, SyncedViews, SyncedViewsState};
use crate::ui::fixtures::DemoPages;
use crate::ui::view::{PageView, PageViewState};
use craft_ui::contract::Bind;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(SyncedViews<'static>, covers = [], drive = Drive::Never)]
fn linked(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (pair, old, new) = data.value("pair", (SyncedViewsState::default(), PageViewState::default(), PageViewState::default()));
    let (old_pages, new_pages) = (DemoPages::new(3), DemoPages::new(3));
    let out = SyncedViews::new("Compare", pair).show(ui, (old, new), |ui, rect, state, half| match half {
        Lead::Left => PageView::new("Old", &old_pages, Bind::bound(state)).show_at(ui, rect, |_, _| ()),
        Lead::Right => PageView::new("New", &new_pages, Bind::bound(state)).show_at(ui, rect, |_, _| ()),
    });
    out.into()
}
