//! Stories of `BreadcrumbBar`.

use super::BreadcrumbBar;
use crate::contract::{Entry, Show as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(BreadcrumbBar<'static, u8>, covers = [Default], drive = Drive::Never)]
fn breadcrumb_bar(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let crumbs: Vec<Entry<u8>> = ["photos", "2026", "trip", "day-one", "selects"].into_iter().zip(0u8..).map(|(name, at)| Entry::new(at, name)).collect();
    BreadcrumbBar::new("Path", &crumbs).show(ui).into()
}
