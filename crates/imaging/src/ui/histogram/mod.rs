//! Stub of img-hist's planned API, for type-checking the scopes track alone.
use craft_behavior::{Metrics, Sense};
use craft_ui::contract::{Bind, BindMut, Common, Component, Cx, EditResponse, MeasureCx, PaintUi, Props as _, Role};
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Response, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ChannelColor {
    #[default]
    Neutral,
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistogramData;
impl HistogramData {
    pub fn tone(_: &[u32], _: ChannelColor) -> Self {
        HistogramData
    }
    pub fn rgb(_: [&[u32]; 3]) -> Self {
        HistogramData
    }
    pub fn channels(_: &[(&[u32], ChannelColor)]) -> Self {
        HistogramData
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelRange {
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlotHeight {
    Standard,
    Short,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StaleMark {
    Button(Label),
}

#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub struct HistStyle {
    #[style(metrics)]
    metrics: Metrics,
}

pub struct HistogramOutput {
    pub selection: EditResponse<Option<LevelRange>>,
    pub hovered_level: Option<f32>,
    pub refresh: bool,
    pub response: Response,
}

impl craft_ui::contract::Outcome for HistogramOutput {
    type Value = craft_ui::contract::NoValue;
    fn response(&self) -> &Response {
        &self.response
    }
}

#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(content_painter)]
pub struct Histogram<'a> {
    #[common]
    common: Common,
    data: &'a HistogramData,
    /// Doc.
    #[prop]
    value: Option<Label>,
    /// Doc.
    #[prop]
    outline: Option<&'a HistogramData>,
    /// Doc.
    #[prop]
    markers: &'a [(f32, Label)],
    selection: Bind<'a, Option<LevelRange>>,
    /// Doc.
    #[prop]
    stale: Option<StaleMark>,
    /// Doc.
    #[prop]
    height: Option<PlotHeight>,
}

impl<'a> Histogram<'a> {
    pub fn new(label: impl Into<Label>, data: &'a HistogramData) -> Self {
        Histogram { common: Common::new(), data, value: None, outline: None, markers: &[], selection: Bind::kept(), stale: None, height: None }.label(label)
    }
    pub fn selection(mut self, selection: Bind<'a, Option<LevelRange>>) -> Self {
        self.selection = selection;
        self
    }
}

impl Component for Histogram<'_> {
    const ROLE: Role = Role::IMAGE;
    type Style = HistStyle;
    type Output = HistogramOutput;
    fn style(_: &StyleCx<'_>, _: HistogramStyleKey) -> HistStyle {
        HistStyle { metrics: Metrics::new() }
    }
    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }
    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }
    fn paint(&mut self, _: &mut PaintUi<'_>, _: &Cx<'_, Self>) {}
    fn finish(self, cx: Cx<'_, Self>) -> HistogramOutput {
        HistogramOutput { response: cx.response.clone(), selection: craft_ui::testing::contract::edit_response(cx.response.clone()), hovered_level: None, refresh: false }
    }
}
