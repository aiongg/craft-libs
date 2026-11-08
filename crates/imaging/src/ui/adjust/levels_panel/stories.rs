//! Stories of the Levels panel: the composite with the point pickers, and an isolated mask channel.

use super::LevelsPanel;
use crate::ui::adjust::{Levels, ToneChannel, ToneChannels, TonePicker, ToneSpace};
use crate::ui::curve::stories::story_bins;
use crate::ui::histogram::ChannelColor;
use craft_ui::contract::Bind;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

/// The host's values of the panel story.
#[derive(Clone, Debug, PartialEq)]
struct Rig {
    levels: Vec<Levels>,
    armed: Option<TonePicker>,
}

fn counts(shift: f32) -> [u32; 256] {
    let mut counts = [0u32; 256];
    for (slot, bin) in counts.iter_mut().zip(story_bins(shift)) {
        *slot = bin;
    }
    counts
}

#[story(LevelsPanel<'static>, covers = [Default], drive = Drive::Never)]
fn levels_panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rig = data.value("rig", Rig { levels: vec![Levels::identity(0.0..=255.0); 4], armed: Some(TonePicker::Black) });
    let histograms = [counts(0.0), counts(0.0), counts(12.0), counts(-12.0)];
    LevelsPanel::new("Levels", Bind::bound(&mut rig.levels), ToneChannels::of(ToneSpace::Rgb, false))
        .histograms(&histograms)
        .pickers(Bind::bound(&mut rig.armed))
        .show(ui)
        .into()
}

#[story(LevelsPanel<'static>, covers = [], drive = Drive::Never)]
fn levels_panel_isolated_mask(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let levels = data.value("mask", vec![Levels::identity(0.0..=255.0)]);
    let channels = ToneChannels::isolated(ToneChannel::Named { name: "Layer mask".to_owned(), color: ChannelColor::Neutral });
    LevelsPanel::new("Levels", Bind::bound(levels), channels).histograms(&[counts(0.0)]).show(ui).into()
}
