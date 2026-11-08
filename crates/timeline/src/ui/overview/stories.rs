//! Stories of `TimelineOverview`: eight tracks of runs, the visible window and the playhead.

use craft_behavior::Mixed;
use craft_ui::contract::{Bind, Show as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Color32, Pos2, Ui, Vec2};

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

/// The closest zoom the invalid story accepts, in points per second.
const CLOSEST_ZOOM: f64 = 100.0;
/// Where along the overview the drive presses and how far it drags, as shares of its width.
const PRESS_SHARE: f32 = 0.2;
const DRAG_SHARE: f32 = 0.15;

/// Presses the overview off the window's middle and drags: the window follows the pointer.
fn centre_the_window(fixture: &mut Fixture) {
    let shown = fixture.shown();
    let from = Pos2::new(shown.left() + shown.width() * PRESS_SHARE, shown.center().y);
    fixture.drag_at(from, Vec2::X * (shown.width() * DRAG_SHARE));
}

/// Shows the overview of the eight tracks over `view`.
fn shown(ui: &mut Ui, view: Bind<'_, TimeView>) -> StoryOutput {
    let width = ui.available_width();
    let runs = overview_runs(&tracks(), Tick::from_seconds_f64(END), Vec2::new(width, ui.available_height()));
    TimelineOverview::new("Overview", &runs, view, Tick::from_seconds_f64(END))
        .shown_width(width)
        .playhead(Tick::from_seconds_f64(END / 2.0))
        .show(ui)
        .into()
}

#[story(TimelineOverview<'static>, covers = [Default], drive = Drive::Custom(centre_the_window))]
fn eight_tracks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    shown(ui, Bind::bound(data.value("view", TimeView::new(Tick::from_seconds_f64(10.0), 40.0))))
}

#[story(TimelineOverview<'static>, covers = [Mixed], drive = Drive::Never)]
fn views_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    shown(ui, Bind::bound_mixed(data.value("mixed", Mixed::<TimeView>::Mixed)))
}

#[story(TimelineOverview<'static>, covers = [Invalid], drive = Drive::Never)]
fn window_closer_than_the_limit(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let not_too_close = |view: &TimeView| {
        if view.points_per_second > CLOSEST_ZOOM { Validity::Refuse(Message::new("The window cannot be that narrow")) } else { Validity::Ok }
    };
    shown(ui, Bind::bound(data.value("close", TimeView::new(Tick::ZERO, CLOSEST_ZOOM * 2.0))).check(not_too_close))
}
