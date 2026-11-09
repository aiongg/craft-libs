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

