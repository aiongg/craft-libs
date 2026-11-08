//! Stories of the selection statistics readout.

use super::{SelectionStats, SelectionStatsReadout, StatsShown, StatsWords, stats_parts};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

const STATS: SelectionStats = SelectionStats { count: 4, numeric: 2, sum: 7.0, min: 2.0, max: 5.0 };

fn every() -> StatsShown {
    StatsShown { average: true, count: true, numerical_count: true, minimum: true, maximum: true, sum: true }
}

fn readout(ui: &mut Ui, data: &mut StoryData, initial: StatsShown) -> StoryOutput {
    let shown = *data.value("shown", initial);
    let parts = stats_parts(&STATS, &shown, &StatsWords::default(), |value| format!("{value:.1}"));
    SelectionStatsReadout::new("Selection statistics", &parts, Bind::bound(data.value("shown", initial))).show(ui).into()
}

#[story(SelectionStatsReadout<'static>, covers = [Default])]
fn every_statistic(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    readout(ui, data, every())
}

#[story(SelectionStatsReadout<'static>, covers = [], drive = Drive::Never)]
fn some_hidden(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    readout(ui, data, StatsShown::default())
}
