//! Stories of the selection statistics readout.

use super::{SelectionStats, SelectionStatsReadout, StatsShown, StatsWords, stats_parts};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn stats() -> SelectionStats {
    SelectionStats::new(4, 2, 7.0, 2.0, 5.0)
}

fn readout(ui: &mut Ui, data: &mut StoryData, initial: StatsShown) -> StoryOutput {
    let shown = *data.value("shown", initial);
    let parts = stats_parts(&stats(), &shown, &StatsWords::default(), |value| format!("{value:.1}"));
    SelectionStatsReadout::new("Selection statistics", &parts, Bind::bound(data.value("shown", initial))).show(ui).into()
}

#[story(SelectionStatsReadout<'static>, covers = [Default])]
fn every_statistic(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    readout(ui, data, StatsShown::all())
}

#[story(SelectionStatsReadout<'static>, covers = [], drive = Drive::Never)]
fn some_hidden(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    readout(ui, data, StatsShown::default())
}
