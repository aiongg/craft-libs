//! Stories of the keyframe lane, the keyframe navigator and the animation toggle.

use craft_behavior::Selection;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Rect, Ui, vec2};

use super::{AnimationToggle, KeyframeId, KeyframeInterpolation, KeyframeLane, KeyframeMark, KeyframeNavigator};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;
use crate::ui::TimelineView;

/// The lane's zoom, in points per second.
const POINTS_PER_SECOND: f64 = 50.0;
/// The lane's height, in points.
const LANE_HEIGHT: f32 = 28.0;

fn seconds(at: f64) -> Tick {
    Tick::from_seconds_f64(at)
}

fn keys() -> Vec<KeyframeMark> {
    let key = |id: u64, at: f64, interpolation| KeyframeMark::new(KeyframeId(id), seconds(at), interpolation);
    vec![
        key(1, 1.0, KeyframeInterpolation::Linear),
        key(2, 3.0, KeyframeInterpolation::Hold),
        key(3, 5.0, KeyframeInterpolation::Bezier),
        key(4, 7.5, KeyframeInterpolation::Linear),
        key(5, 10.0, KeyframeInterpolation::Bezier),
    ]
}

#[story(KeyframeLane<'static>, covers = [Default], drive = Drive::Never)]
fn linear_hold_and_bezier_keys(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let keys = keys();
    let selection = Selection::new(vec![KeyframeId(2), KeyframeId(3)], Some(KeyframeId(3)), Some(KeyframeId(2)));
    let view = data.value("view", TimeView::new(Tick::ZERO, POINTS_PER_SECOND));
    let format = TimeFormat::timecode(FrameRate::FPS_24);
    let area = ui.available_rect_before_wrap();
    let lane = Rect::from_min_size(area.min, vec2(area.width(), LANE_HEIGHT));
    TimelineView::new("Timeline", Bind::bound(view), format, seconds(12.0))
        .show(ui, |ui, _| KeyframeLane::new("Keyframes", &keys, &selection).playhead(seconds(4.0)).show_at(ui, lane))
        .inner
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(KeyframeNavigator, covers = [Default], drive = Drive::Never)]
fn navigator_between_keys(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let out = KeyframeNavigator::new("Keyframe navigator", true, true, false).show(ui);
    out.response.map(StoryOutput::from).unwrap_or_default()
}

#[story(AnimationToggle<'static>, covers = [Default])]
fn stopwatch(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AnimationToggle::new(Bind::bound(data.value("animated", false))).show(ui).into()
}
