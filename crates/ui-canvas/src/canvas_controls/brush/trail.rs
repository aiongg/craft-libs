//! `StrokeTrail`: the footprint of the stroke being painted, over a coverage raster with partial uploads.

use crate::contract::{Access, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleColor, StyleCx};
use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{Input, Metrics};
use egui::Response;

/// A coverage raster over a document rect: one byte per texel, `scale` document units wide, stamped by the tool as the
/// stroke grows and uploaded by [`StrokeTrail`] by its changed rows.
#[derive(Debug)]
#[allow(dead_code)] // P3(tools-w17): tl.T52
pub struct CoverageMask {
    bounds: Rect,
    max_side: u32,
}

impl CoverageMask {
    /// A mask over `bounds` with no side longer than `max_side` texels.
    pub fn new(bounds: Rect, max_side: u32) -> Self {
        let _ = (bounds, max_side);
        todo!("tl.T52")
    }

    /// Stamps the capsule from `from` to `to` (document units) of radius `radius`, keeping the larger coverage.
    pub fn stamp_capsule(&mut self, from: Point, to: Point, radius: f64) {
        let _ = (from, to, radius);
        todo!("tl.T52")
    }

    /// Writes `max(coverage(point), base)` into the texels of `rect`; `coverage` is asked at document points.
    pub fn restamp(&mut self, rect: Rect, coverage: impl Fn(Point) -> f32) {
        let _ = (rect, coverage);
        todo!("tl.T52")
    }

    /// Starts a stroke: the coverage so far becomes the base `restamp` keeps.
    pub fn begin_stroke(&mut self) {
        todo!("tl.T52")
    }

    /// The coverage at the document point `at`, 0 outside the mask.
    pub fn coverage_at(&self, at: Point) -> u8 {
        let _ = at;
        todo!("tl.T52")
    }

    /// Document units per texel.
    pub fn scale(&self) -> f64 {
        todo!("tl.T52")
    }

    /// The mask's size in texels, width then height.
    pub fn size(&self) -> (u32, u32) {
        todo!("tl.T52")
    }
}

/// What the trail shows its footprint as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrailTone {
    /// The marquee's colour.
    #[default]
    Light,
    /// The halo's colour.
    Shade,
    /// The bad status colour.
    Danger,
}

/// The style of the stroke trail: the texture carries the colour, so only the metrics.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct StrokeTrailStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The colour of a [`TrailTone::Light`] footprint.
    light: StyleColor,
    /// The colour of a [`TrailTone::Shade`] footprint.
    shade: StyleColor,
    /// The colour of a [`TrailTone::Danger`] footprint.
    danger: StyleColor,
}

/// The footprint of the stroke being painted, drawn from a [`CoverageMask`] as one textured quad over the mask's
/// document rect. Display only.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(canvas_control, content_painter)]
pub struct StrokeTrail<'a> {
    #[common]
    common: Common,
    #[allow(dead_code)] // P3(tools-w17): tl.T52
    mask: &'a mut CoverageMask,
    /// What the footprint shows as.
    #[prop]
    #[allow(dead_code)] // P3(tools-w17): tl.T52
    tone: TrailTone,
}

impl<'a> StrokeTrail<'a> {
    /// A trail named `label` over `mask`.
    pub fn new(label: impl Into<Label>, mask: &'a mut CoverageMask) -> Self {
        StrokeTrail { common: Common::new(), mask, tone: TrailTone::Light }.label(label)
    }
}

impl Component for StrokeTrail<'_> {
    const ROLE: Role = Role::GROUP;
    const INPUT: Input = Input::Display;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = StrokeTrailStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: StrokeTrailStyleKey) -> StrokeTrailStyle {
        let color = &cx.tokens.color;
        StrokeTrailStyle {
            metrics: Metrics::new(),
            light: cx.color(color.overlay.marquee),
            shade: cx.color(color.overlay.halo),
            danger: cx.color(color.status.bad.solid),
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::ZERO
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let _ = (ui, cx);
        todo!("tl.T52")
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.role(Role::None);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
