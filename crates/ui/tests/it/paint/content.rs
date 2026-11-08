//! Icons in data colours and the widths a content painter strokes with.

use craft_behavior::Mixed;
use craft_i18n::Direction;
use craft_theme::{Contrast, Mode};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::primitives::{IconSpec, StrokeWidth, ToolIcon};
use craft_ui::runtime::env::Scale;
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
const SPARSE: f32 = 1.0;
const DENSE: f32 = 2.0;
const MIDDLE: f64 = 50.0;
const EPS: f32 = 1e-3;
const ONE_PIXEL: f32 = 1.0;

#[trial]
fn an_icon_with_a_data_colour_paints_in_that_colour() {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let spec = IconSpec::svg(ToolIcon::Brush.icon()).data_color(DataColor::new(PICTOGRAM));
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| draw.icon(RECT, &spec));
    let tinted = shapes.iter().any(|shape| matches!(shape, Shape::Mesh(mesh) if mesh.vertices.iter().all(|v| v.color == PICTOGRAM)));
    assert!(tinted, "the icon is tinted with the data colour and not the panel ink");
}

/// The width in points a content painter reports for a hairline, with the pixels per point it painted at.
fn hairline_at(scale: Scale) -> (f32, f32) {
    let seen = Rc::new(Cell::new(None));
    let record = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let record = record.clone();
        PointPicker::new(Bind::given(Mixed::each([MIDDLE, MIDDLE])), FULL, FULL)
            .backdrop(move |cx, _| record.set(Some((cx.stroke_width(StrokeWidth::Hairline), cx.painter.ctx().pixels_per_point()))))
            .show(ui);
    });
    fixture.scale(scale);
    fixture.settle();
    seen.get().unwrap_or_default()
}

#[trial]
fn a_hairline_stays_one_physical_pixel_as_the_density_grows() {
    let (sparse, dense) = (hairline_at(Scale::new(SPARSE)), hairline_at(Scale::new(DENSE)));
    assert!(dense.1 > sparse.1, "the dense display has more pixels per point");
    for (width, pixels_per_point) in [sparse, dense] {
        assert!((width * pixels_per_point - ONE_PIXEL).abs() < EPS, "{width} points at {pixels_per_point} pixels per point is not one pixel");
    }
}
