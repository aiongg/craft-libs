//! Stories of the marker strip and the Marker dialog.

use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::{DialogStack, StackDialog as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Rect, Ui, vec2};

use super::{MarkerDialog, MarkerStrip};
use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;
use crate::ui::TimelineView;

/// The strip's zoom, in points per second.
const POINTS_PER_SECOND: f64 = 40.0;
/// The strip's height, in points.
const STRIP_HEIGHT: f32 = 28.0;

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

#[story(MarkerStrip<'static>, covers = [Default], drive = Drive::Never)]
fn point_and_range_markers(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let markers = [point(), range()];
    let view = data.value("view", TimeView::new(Tick::ZERO, POINTS_PER_SECOND));
    let area = ui.available_rect_before_wrap();
    let strip = Rect::from_min_size(area.min, vec2(area.width(), STRIP_HEIGHT));
    TimelineView::new("Timeline", Bind::bound(view), format(), seconds(16.0))
        .show(ui, |ui, _| MarkerStrip::new("Markers", &markers).bin().show_at(ui, strip))
        .inner
        .map(StoryOutput::from)
        .unwrap_or_default()
}

#[story(MarkerDialog, covers = [Default], drive = Drive::Never)]
fn dialog_on_a_range_marker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let dialog = data.value("dialog", MarkerDialog::new(&range(), format()));
    let _ = dialog.show(ui, &DialogStack::default());
    StoryOutput::default()
}
