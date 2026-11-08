//! Stories of `TimelineView`: the time area over a 30 s sequence with its bar.

use super::TimelineView;
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(TimelineView<'static>, covers = [Default], drive = Drive::Never)]
fn timeline_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let format = TimeFormat::timecode(FrameRate::FPS_29_97).drop_frame(true);
    let view = *data.value("view", TimeView::new(Tick::ZERO, 24.0));
    let out = TimelineView::new("Time", Bind::given(view), format, Tick::from_seconds_f64(30.0)).show(ui, |_, _| {});
    out.output.into()
}
