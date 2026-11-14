//! The Cylinder warp editor: a canvas control with the cylinder's handles over a [`WarpMesh`].

use craft_behavior::Metrics;
use craft_ui::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, Outcome, Props as _, Role};
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::{ComponentStyle, StyleColor, StyleCx};
use egui::{Response, Vec2};

use super::WarpMesh;

/// The cylinder's values: the box, the depths of its top and bottom edges and the wrap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CylinderValues {
    /// The left edge of the box.
    pub left: f64,
    /// The bottom edge of the box.
    pub bottom: f64,
    /// The right edge of the box.
    pub right: f64,
    /// The top edge of the box.
    pub top: f64,
    /// How far the top edge curves in depth.
    pub top_depth: f64,
    /// How far the bottom edge curves in depth.
    pub bottom_depth: f64,
    /// How far the surface wraps around the cylinder.
    pub wrap: f64,
}

impl CylinderValues {
    /// The values of a flat cylinder over `rect` (left, top, right, bottom).
    pub fn for_box(rect: [f64; 4]) -> Self {
        let _ = rect;
        todo!("tl.T16")
    }

    /// The values from `[left, bottom, right, top, top_depth, bottom_depth, wrap]`.
    pub fn from_array(values: [f64; 7]) -> Self {
        let _ = values;
        todo!("tl.T16")
    }

    /// The values as `[left, bottom, right, top, top_depth, bottom_depth, wrap]`.
    pub fn to_array(self) -> [f64; 7] {
        todo!("tl.T16")
    }
}

/// A handle of the cylinder editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CylinderHandle {
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bend of the top edge.
    Bend,
    /// The bottom edge.
    Bottom,
    /// The wrap.
    Wrap,
    /// The surface.
    Surface,
}

/// The look of the cylinder editor.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CylinderWarpStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The box and the cylinder's outline.
    line: StyleColor,
}

/// The cylinder editor over `values`, drawn over `mesh`.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(canvas_control)]
pub struct CylinderWarp<'a> {
    #[common]
    common: Common,
    values: Bind<'a, CylinderValues>,
    mesh: &'a WarpMesh,
}

impl<'a> CylinderWarp<'a> {
    /// The cylinder editor named `label` over `values` and `mesh`.
    pub fn new(label: impl Into<Label>, values: Bind<'a, CylinderValues>, mesh: &'a WarpMesh) -> Self {
        CylinderWarp { common: Common::new(), values, mesh }.label(label)
    }
}

/// What the cylinder editor reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct CylinderWarpOutput {
    /// The edit of the values.
    pub values: EditResponse<CylinderValues>,
    /// The handle under the pointer.
    pub hovered: Option<CylinderHandle>,
}

impl Outcome for CylinderWarpOutput {
    type Value = CylinderValues;

    fn response(&self) -> &Response {
        &self.values.response
    }
}

impl Component for CylinderWarp<'_> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = CylinderWarpStyle;
    type Output = CylinderWarpOutput;

    fn style(cx: &StyleCx<'_>, _: CylinderWarpStyleKey) -> CylinderWarpStyle {
        CylinderWarpStyle { metrics: Metrics::new(), line: cx.color(cx.tokens.color.overlay.line) }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn bind(&mut self) -> Option<BindMut<'_, CylinderValues>> {
        Some(self.values.reborrow())
    }

    fn finish(self, cx: Cx<'_, Self>) -> CylinderWarpOutput {
        let _ = self.mesh;
        CylinderWarpOutput { values: cx.edit_response(), hovered: None }
    }
}
