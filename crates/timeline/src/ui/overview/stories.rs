//! Stories of `TimelineOverview`: eight tracks of runs, the visible window and the playhead.

use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, Ui, Vec2};

use super::{TimelineOverview, overview_runs};
use crate::time::{Tick, TimeRange, TimeView};

/// The tracks of the story and the clips of each.
const TRACKS: usize = 8;
/// The clips of a track, one every `STEP` seconds, each `LENGTH` seconds long.
const CLIPS: usize = 6;
const STEP: f64 = 9.0;
const LENGTH: f64 = 6.0;
/// The content's end, in seconds.
const END: f64 = CLIPS as f64 * STEP;
/// The colours of the tracks, in turn.
const COLORS: [Color32; 3] = [Color32::from_rgb(0x38, 0x0e, 0xa7), Color32::from_rgb(0x1d, 0x4a, 0x64), Color32::from_rgb(0x35, 0x54, 0x18)];

fn tracks() -> Vec<Vec<(TimeRange, Color32)>> {
    (0..TRACKS)
        .map(|track| {
            (0..CLIPS)
                .map(|clip| {
                    let start = Tick::from_seconds_f64(clip as f64 * STEP + track as f64 * 0.5);
                    (TimeRange::new(start, Tick::from_seconds_f64(LENGTH)), COLORS[track % COLORS.len()])
                })
                .collect()
        })
        .collect()
}

#[story(TimelineOverview<'static>, covers = [Default], drive = Drive::Never)]
fn eight_tracks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", TimeView::new(Tick::from_seconds_f64(10.0), 40.0));
    let width = ui.available_width();
    let runs = overview_runs(&tracks(), Tick::from_seconds_f64(END), Vec2::new(width, ui.available_height()));
    TimelineOverview::new("Overview", &runs, Bind::bound(view), Tick::from_seconds_f64(END))
        .shown_width(width)
        .playhead(Tick::from_seconds_f64(END / 2.0))
        .show(ui)
        .into()
}
