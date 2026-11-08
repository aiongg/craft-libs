//! Icons in data colours and the widths a content painter strokes with.

use craft_behavior::Mixed;
use craft_i18n::Direction;
use craft_theme::{Contrast, Mode};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::primitives::{IconSpec, StrokeWidth, ToolIcon};
use craft_ui::style::{DataColor, Ink};
use craft_ui::testing::style::{paint_frame, tokens};
use craft_ui::testing::{Fixture, Frames, trial};
use craft_ui::value_drag::PointPicker;
use egui::{Color32, Rect, Shape, pos2};
use std::cell::Cell;
use std::rc::Rc;

const RECT: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 40.0));
const PICTOGRAM: Color32 = Color32::from_rgb(200, 30, 90);
const FULL: std::ops::RangeInclusive<f64> = 0.0..=100.0;
const DENSE: f32 = 2.0;

#[trial]
fn an_icon_with_a_data_colour_paints_in_that_colour() {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let spec = IconSpec::svg(ToolIcon::Brush.icon()).data_color(DataColor::new(PICTOGRAM));
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| draw.icon(RECT, &spec));
    let tinted = shapes.iter().any(|shape| matches!(shape, Shape::Mesh(mesh) if mesh.vertices.iter().all(|v| v.color == PICTOGRAM)));
    assert!(tinted, "the icon is tinted with the data colour and not the panel ink");
}

#[trial]
fn a_hairline_is_one_physical_pixel() {
    let seen = Rc::new(Cell::new(None));
    let record = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let record = record.clone();
        PointPicker::new(Bind::given(Mixed::each([50.0, 50.0])), FULL, FULL)
            .backdrop(move |cx, _| record.set(Some(cx.stroke_width(StrokeWidth::Hairline))))
            .show(ui);
    });
    fixture.ctx().set_pixels_per_point(DENSE);
    fixture.settle();
    let width = seen.get().unwrap_or_default();
    assert!((width * DENSE - 1.0).abs() < 1e-3, "a hairline is {width} points, one pixel at {DENSE} pixels per point");
}
