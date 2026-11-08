//! Stories of the chart painter.
//!
//! `ChartSheet` is the component the stories paint on: a chart is painted through the `Draw` of a component, and
//! the table widgets have none that shows a bare primitive list.

use super::{ChartPrimitive, HAlign, VAlign, paint_chart};
use craft_ui::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show as _};
use craft_ui::style::{ComponentStyle, DataColor, StyleCx};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, Response, Ui, Vec2};

/// The style of the story's chart sheet.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub struct ChartSheetStyle {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// A sheet that paints a primitive list at a zoom.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
pub struct ChartSheet<'a> {
    #[common]
    common: Common,
    primitives: &'a [ChartPrimitive],
    size: Vec2,
    zoom: f32,
}

impl Component for ChartSheet<'_> {
    const ROLE: Role = Role::IMAGE;
    const OVERLAYS: bool = false;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = ChartSheetStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ChartSheetStyleKey) -> ChartSheetStyle {
        ChartSheetStyle { metrics: craft_behavior::Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        self.size * self.zoom
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        paint_chart(&ui.draw(), cx.rect.min, self.primitives, self.zoom);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn series(r: u8, g: u8, b: u8) -> DataColor {
    DataColor::new(Color32::from_rgb(r, g, b))
}

fn column(x: f32, height: f32, fill: DataColor) -> ChartPrimitive {
    ChartPrimitive::Rect { x, y: 110.0 - height, width: 24.0, height, fill: Some(fill), stroke: None, radius: 2.0 }
}

fn label(x: f32, y: f32, text: &str, bold: bool, align: HAlign, valign: VAlign) -> ChartPrimitive {
    ChartPrimitive::Text { x, y, text: text.to_owned(), color: None, bold, align, valign, rotation: 0.0 }
}

fn columns() -> Vec<ChartPrimitive> {
    let (blue, orange) = (series(66, 133, 244), series(244, 160, 0));
    vec![
        label(110.0, 6.0, "Sales", true, HAlign::Center, VAlign::Top),
        column(20.0, 40.0, blue),
        column(52.0, 70.0, orange),
        column(84.0, 55.0, blue),
        column(116.0, 85.0, orange),
        label(32.0, 116.0, "Q1", false, HAlign::Center, VAlign::Top),
        label(128.0, 116.0, "Q4", false, HAlign::Center, VAlign::Top),
        ChartPrimitive::Polygon {
            points: vec![[160.0, 100.0], [180.0, 70.0], [200.0, 90.0], [220.0, 50.0], [220.0, 110.0], [160.0, 110.0]],
            fill: series(52, 168, 83),
            stroke: None,
        },
        ChartPrimitive::Circle { center_x: 190.0, center_y: 30.0, radius: 6.0, fill: Some(series(219, 68, 55)), stroke: None },
    ]
}

fn doughnut() -> Vec<ChartPrimitive> {
    let tau = std::f32::consts::TAU;
    let shares = [(0.45, series(66, 133, 244)), (0.30, series(244, 160, 0)), (0.25, series(52, 168, 83))];
    let mut start = 0.0;
    let mut out = Vec::new();
    for (share, fill) in shares {
        let end = start + share * tau;
        out.push(ChartPrimitive::Wedge {
            center_x: 70.0,
            center_y: 70.0,
            outer_radius: 60.0,
            inner_radius: 30.0,
            start_angle: start,
            end_angle: end,
            fill,
            stroke: None,
        });
        start = end;
    }
    out.push(label(70.0, 70.0, "Share", true, HAlign::Center, VAlign::Center));
    out
}

fn shown(ui: &mut Ui, primitives: &[ChartPrimitive], size: Vec2) -> StoryOutput {
    ChartSheet { common: Common::new(), primitives, size, zoom: 1.0 }.label("Chart").show(ui).into()
}

#[story(ChartSheet<'static>, covers = [Default], drive = Drive::Never)]
fn column_chart(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    shown(ui, &columns(), Vec2::new(240.0, 130.0))
}

#[story(ChartSheet<'static>, covers = [], drive = Drive::Never)]
fn doughnut_chart(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    shown(ui, &doughnut(), Vec2::new(140.0, 140.0))
}
