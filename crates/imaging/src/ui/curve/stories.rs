//! Stories of the curve editor, the preview and the tone ramp.

use super::graph::PLOT;
use super::{Curve, CurveChannel, CurveEditor, CurvePreview, ToneRamp};
use crate::ui::histogram::{ChannelColor, HistogramData};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::display::Text;
use craft_ui::style::DataColor;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, Ui};

/// Sample bins of an image's channel: two hills, the channel's mean shifted by `shift`.
pub(crate) fn story_bins(shift: f32) -> Vec<u32> {
    (0..256)
        .map(|k| {
            let x = k as f32;
            let hill = |mean: f32, spread: f32, height: f32| height * (-((x - mean) / spread).powi(2)).exp();
            (hill(70.0 + shift, 28.0, 900.0) + hill(180.0 - shift, 40.0, 600.0) + 20.0) as u32
        })
        .collect()
}

fn channels() -> [CurveChannel; 4] {
    [
        CurveChannel::new("RGB", ChannelColor::Neutral),
        CurveChannel::new("Red", ChannelColor::Red),
        CurveChannel::new("Green", ChannelColor::Green),
        CurveChannel::new("Blue", ChannelColor::Blue),
    ]
}

fn bent() -> Curve {
    Curve::smooth(vec![[0.0, 0.0], [64.0, 40.0], [128.0, 150.0], [255.0, 255.0]])
}

/// The RGB composite with the other channels' curves behind it and a histogram under it.
#[story(CurveEditor<'static>, covers = [Default], drive = Drive::Drag(PLOT))]
fn curve_editor(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let overlays = data
        .value("overlays", vec![bent(), Curve::identity(255.0), Curve::smooth(vec![[0.0, 20.0], [255.0, 235.0]]), Curve::identity(255.0)])
        .clone();
    let histogram = HistogramData::tone(&story_bins(0.0), ChannelColor::Neutral);
    let curve = data.value("curve", bent());
    let output = CurveEditor::new("Curve", Bind::bound(curve))
        .channels(&channels())
        .overlays(&overlays)
        .histogram(&histogram)
        .hint("Click the graph to add a point")
        .show(ui);
    output.into()
}

/// Pencil mode: a drag draws one output per input level.
#[story(CurveEditor<'static>, covers = [], drive = Drive::Drag(PLOT))]
fn curve_editor_pencil(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let curve = data.value("drawn", Curve::Map((0..=255).map(|k| k as f32).collect()));
    CurveEditor::new("Curve", Bind::bound(curve)).pencil().show(ui).into()
}

/// The contour editor: corner points, percent fields and a host row between the graph and the fields.
#[story(CurveEditor<'static>, covers = [], drive = Drive::Never)]
fn curve_editor_corners(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let curve =
        data.value("contour", Curve::from_parts(vec![[0.0, 0.0], [90.0, 160.0], [180.0, 100.0], [255.0, 255.0]], vec![false, true, false, false]));
    CurveEditor::new("Contour", Bind::bound(curve))
        .corners()
        .percent()
        .bars(false)
        .between(|ui, _| {
            Text::new("Name").show(ui);
        })
        .show(ui)
        .into()
}

/// Fields in percent of the levels, with the selection kept by the host.
#[story(CurveEditor<'static>, covers = [], drive = Drive::Never)]
fn curve_editor_percent(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let selected = *data.value("selected", Some(1usize));
    let curve = data.value("percent", bent());
    let output = CurveEditor::new("Curve", Bind::bound(curve)).percent().selection(selected).show(ui);
    *data.value("selected", Some(1usize)) = output.selected;
    output.into()
}

/// One curve, read-only.
#[story(CurvePreview<'static>, covers = [Default], drive = Drive::Never)]
fn curve_preview(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let curve = data.value("curve", bent()).clone();
    CurvePreview::new("Preset", &curve).show(ui).into()
}

/// The ramp the tone bars and the slider tracks share.
#[story(ToneRamp<'static>, covers = [Default], drive = Drive::Never)]
fn tone_ramp(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let stops = [DataColor::new(Color32::from_gray(0)), DataColor::new(Color32::from_gray(u8::MAX))];
    ToneRamp::new(&stops).show(ui).into()
}
