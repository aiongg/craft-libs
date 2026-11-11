//! Stories of the hover and selection outlines.

use super::{HoverMask, HoverOutline, HoverShape, SelectionOutline};
use crate::canvas::testing::canvas_story::on_canvas;
use crate::contract::Show as _;
use crate::style::DataColor;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::kurbo::{Affine, BezPath, Rect, Shape as _};
use egui::{Color32, Ui};

/// Two layers' colours, as an app gives each layer its own.
const LAYERS: [Color32; 2] = [Color32::from_rgb(70, 130, 230), Color32::from_rgb(60, 180, 90)];

fn bounds() -> Rect {
    Rect::new(240.0, 145.0, 400.0, 255.0)
}

fn paths() -> Vec<BezPath> {
    vec![bounds().to_path(0.1), Rect::new(270.0, 165.0, 320.0, 215.0).to_path(0.1)]
}

/// The three hover shapes side by side: a rectangle in the style's colour, a path and a coverage tint in a layer's.
#[story(HoverOutline, covers = [], drive = Drive::Never, group = "Outline")]
fn hover_shapes(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let layer = DataColor::new(LAYERS[1]);
    let rect = Rect::new(60.0, 145.0, 200.0, 255.0);
    let mut path = BezPath::new();
    path.move_to((250.0, 255.0));
    path.line_to((320.0, 145.0));
    path.line_to((390.0, 255.0));
    path.close_path();
    let mask = HoverMask::sampled(Rect::new(440.0, 145.0, 580.0, 255.0), |x, y| if (x - 510.0).hypot(y - 200.0) < 55.0 { 1.0 } else { 0.0 });
    on_canvas(ui, |ui, at, _| {
        HoverOutline::new("Rectangle", Some(HoverShape::Rect(rect))).show_at(ui, at);
        HoverOutline::new("Path", Some(HoverShape::Path(path.clone()))).color(layer).show_at(ui, at);
        HoverOutline::new("Mask", Some(HoverShape::Mask(mask.clone()))).color(layer).show_at(ui, at)
    })
}

#[story(HoverOutline, covers = [Default], drive = Drive::Never, group = "Shapes", title = "Rollover")]
fn rollover(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, |ui, at, _| HoverOutline::new("Rollover", Some(HoverShape::Rect(bounds()))).show_at(ui, at))
}

#[story(HoverOutline, covers = [Default], drive = Drive::Never, group = "Shapes", title = "Coverage tint")]
fn coverage_tint(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mask = HoverMask::sampled(bounds(), |x, y| if (x - 320.0).hypot(y - 200.0) < 50.0 { 1.0 } else { 0.0 });
    on_canvas(ui, |ui, at, _| HoverOutline::new("Object", Some(HoverShape::Mask(mask.clone()))).show_at(ui, at))
}

/// The same paths solid on the left and dashed on the right, each path in its layer's colour.
#[story(SelectionOutline<'static>, covers = [], drive = Drive::Never, group = "Outline")]
fn solid_and_dashed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let moved = |dx: f64| paths().into_iter().map(|path| Affine::translate((dx, 0.0)) * path).collect::<Vec<_>>();
    let (left, right) = (moved(-160.0), moved(160.0));
    on_canvas(ui, |ui, at, _| {
        let mut shown = None;
        for (layer, color) in LAYERS.into_iter().enumerate() {
            let color = DataColor::new(color);
            SelectionOutline::solid("Solid paths", &left[layer..=layer]).color(color).show_at(ui, at);
            shown = Some(SelectionOutline::dashed("Dashed paths", &right[layer..=layer]).color(color).show_at(ui, at));
        }
        shown.expect("two layers")
    })
}

#[story(SelectionOutline<'static>, covers = [Default], drive = Drive::Never, group = "Lines", title = "Solid")]
fn solid(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let paths = paths();
    on_canvas(ui, |ui, at, _| SelectionOutline::solid("Selected paths", &paths).show_at(ui, at))
}

#[story(SelectionOutline<'static>, covers = [Default], drive = Drive::Never, group = "Lines", title = "Dashed")]
fn dashed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let paths = paths();
    on_canvas(ui, |ui, at, _| SelectionOutline::dashed("Selected paths", &paths).show_at(ui, at))
}
