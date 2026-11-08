//! Stories of the keyframe lane, the keyframe navigator and the animation toggle.

use craft_behavior::Selection;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{AnimationToggle, KeyframeId, KeyframeInterpolation, KeyframeLane, KeyframeMark, KeyframeNavigator};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;
use crate::ui::TimelineView;

/// The lane's zoom, in points per second.
const POINTS_PER_SECOND: f64 = 50.0;

fn seconds(at: f64) -> Tick {
    Tick::from_seconds_f64(at)
}

fn keys() -> Vec<KeyframeMark> {
    let key = |id: u64, at: f64, interpolation| KeyframeMark::new(KeyframeId::new(id), seconds(at), interpolation);
    vec![
        key(1, 1.0, KeyframeInterpolation::Linear),
        key(2, 3.0, KeyframeInterpolation::Hold),
        key(3, 5.0, KeyframeInterpolation::Bezier),
        key(4, 7.5, KeyframeInterpolation::Linear),
        key(5, 10.0, KeyframeInterpolation::Bezier),
    ]
}

/// The farthest the invalid story lets the keys move, in seconds.
const FARTHEST_SECONDS: f64 = 2.0;

/// The view a story starts from.
fn start_view() -> TimeView {
    TimeView::new(Tick::ZERO, POINTS_PER_SECOND)
}

/// Shows the lane `lane` makes inside a timeline view over `view`, which publishes the time axis the lane reads.
fn in_view<'a>(ui: &mut Ui, view: &mut TimeView, lane: impl FnOnce() -> KeyframeLane<'a>) -> StoryOutput {
    let mut lane = Some(lane);
    let format = TimeFormat::timecode(FrameRate::FPS_24);
    TimelineView::new("Timeline", Bind::bound(view), format, seconds(12.0))
        .show(ui, |ui, _| lane.take().map(|lane| lane().show(ui)))
        .inner
        .flatten()
        .map(StoryOutput::from)
        .unwrap_or_default()
}

fn selected() -> Selection<KeyframeId> {
    Selection::new(vec![KeyframeId::new(2), KeyframeId::new(3)], Some(KeyframeId::new(3)), Some(KeyframeId::new(2)))
}

#[story(KeyframeLane<'static>, covers = [Default])]
fn linear_hold_and_bezier_keys(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (keys, selection) = (keys(), selected());
    let mut view = *data.value("view", start_view());
    let delta = data.value("delta", Tick::ZERO);
    let shown = in_view(ui, &mut view, || KeyframeLane::new("Keyframes", &keys, &selection).playhead(seconds(4.0)).delta(Bind::bound(delta)));
    *data.value("view", start_view()) = view;
    shown
}

#[story(KeyframeLane<'static>, covers = [Invalid], drive = Drive::Never)]
fn keys_moved_past_the_limit(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (keys, selection) = (keys(), selected());
    let mut view = *data.value("view", start_view());
    let delta = data.value("far", seconds(FARTHEST_SECONDS + 1.0));
    let not_too_far = |delta: &Tick| {
        if *delta > seconds(FARTHEST_SECONDS) { Validity::Refuse(Message::new("The keys cannot move that far")) } else { Validity::Ok }
    };
    let shown = in_view(ui, &mut view, || KeyframeLane::new("Keyframes", &keys, &selection).delta(Bind::bound(delta).check(not_too_far)));
    *data.value("view", start_view()) = view;
    shown
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
