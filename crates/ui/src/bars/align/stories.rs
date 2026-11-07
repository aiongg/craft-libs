//! Stories of `AlignBar`.

use super::{AlignAction, AlignBar, DistributeButtons};
use crate::bars::{Bar, BarKind};
use crate::contract::ShowContainer as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(AlignBar<'static>, covers = [], drive = Drive::Never)]
fn align_bar(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Bar::new("Options", BarKind::Options)
        .show(ui, |ui, cx| {
            AlignBar::new().distribute(DistributeButtons::Row).enabled(|action| action != AlignAction::DistributeVertically).show_in(ui, cx);
        })
        .output
        .into()
}
