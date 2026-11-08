//! Stories of the marker strip and the Marker dialog.

use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::dialog::{DialogStack, StackDialog as _};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{MarkerDialog, MarkerSpan, MarkerStrip};
use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;
use crate::ui::TimelineView;

/// The strip's zoom, in points per second.
const POINTS_PER_SECOND: f64 = 40.0;

fn seconds(at: f64) -> Tick {
    Tick::from_seconds_f64(at)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(FrameRate::FPS_24)
}

fn point() -> Marker {
    Marker {
        id: MarkerId(1),
        start: seconds(3.0),
        duration: Tick::ZERO,
        name: "Intro".into(),
        comment: "Cut on the downbeat".into(),
        color: MarkerColor(2),
    }
}

fn range() -> Marker {
    Marker {
        id: MarkerId(2),
        start: seconds(8.0),
        duration: seconds(4.0),
        name: "Swell".into(),
        comment: "Strings rise".into(),
        color: MarkerColor(8),
    }
}

/// The latest start the invalid story accepts for a dragged marker, in seconds.
const LATEST_SECONDS: f64 = 12.0;

/// The view a story starts from.
fn start_view() -> TimeView {
    TimeView::new(Tick::ZERO, POINTS_PER_SECOND)
}

/// Shows the strip `strip` makes inside a timeline view over `view`, which publishes the time axis the strip reads.
fn in_view<'a>(ui: &mut Ui, view: &mut TimeView, strip: impl FnOnce() -> MarkerStrip<'a>) -> StoryOutput {
    let mut strip = Some(strip);
    TimelineView::new("Timeline", Bind::bound(view), format(), seconds(16.0))
        .show(ui, |ui, _| strip.take().map(|strip| strip().show(ui)))
        .inner
        .flatten()
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(MarkerStrip<'static>, covers = [Default])]
fn point_and_range_markers(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let markers = [point(), range()];
    let mut view = *data.value("view", start_view());
    let span = data.value("span", MarkerSpan::new(MarkerId(1), seconds(3.0), Tick::ZERO));
    let shown = in_view(ui, &mut view, || MarkerStrip::new("Markers", &markers).bin().span(Bind::bound(span)));
    *data.value("view", start_view()) = view;
    shown
}

#[story(MarkerStrip<'static>, covers = [Invalid], drive = Drive::Never)]
fn marker_dragged_past_the_limit(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let markers = [point(), range()];
    let mut view = *data.value("view", start_view());
    let late = MarkerSpan::new(MarkerId(1), seconds(LATEST_SECONDS + 1.0), Tick::ZERO);
    let span = data.value("late", late);
    let not_too_late = |span: &MarkerSpan| {
        if span.start > seconds(LATEST_SECONDS) { Validity::Refuse(Message::new("Markers end at the limit")) } else { Validity::Ok }
    };
    let shown = in_view(ui, &mut view, || MarkerStrip::new("Markers", &markers).span(Bind::bound(span).check(not_too_late)));
    *data.value("view", start_view()) = view;
    shown
}

#[story(MarkerDialog, covers = [Default], drive = Drive::Never)]
fn dialog_on_a_range_marker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let dialog = data.value("dialog", MarkerDialog::new(&range(), format()));
    let _ = dialog.show(ui, &DialogStack::default());
    StoryOutput::default()
}
