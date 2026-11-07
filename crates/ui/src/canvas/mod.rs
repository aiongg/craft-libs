//! The `Canvas` container with its tool host, pasteboard, split views, rulers, navigator, cursors and raster display.
//!
//! `Canvas` is an arena root: the leaf components its body shows with `show_at(canvas_rect)` (canvas controls,
//! such as `CropOverlay`) join its arena, and a press away from all of their parts reaches the active tool through
//! the canvas's background part. A canvas given a view ([`Canvas::view`]) publishes a [`CanvasSpace`] to its body
//! and reports the wheel and a middle-button pan.

mod crop;
mod guide;
mod space;
#[cfg(feature = "testing")]
mod stories;
mod words;

mod tool;
mod tool_groups;
mod tool_host;

pub use crop::{CropOverlay, CropStyle};
pub use guide::{Guide, GuideAxis, GuideDrag, GuideKey, GuideLine};
pub use space::{CanvasSpace, RulerText, canvas_space};
pub use words::CanvasWords;

pub use tool::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer};
pub use tool_groups::{PickStep, tool_pick};
pub use tool_host::{ToolHost, ToolHostResponse, ToolInput, ToolReadout};

mod raster_source;
pub use raster_source::{PixelRect, RasterSource};

use crate::contract::{Common, Component, Container, ContainerProps, Cx, NoValue, Outcome, PartsCx, Pointer as _, Props as _, Role, Sizing};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{
    CanvasTransform, CanvasView, HitLevel, HitPriority, Metrics, PartKey, PointerButton, PressPhase, PressState, Sense, Wheel, WheelTaker,
};
use egui::{Pos2, Rect, Response, Vec2};

const BACKGROUND: PartKey = PartKey::new("background");

/// How the active tool is reached without a pointer (8.3, owner question 15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolPath {
    /// The tool creates or selects geometry: a command form takes the same numbers. The text names the command
    /// ("the Crop To... command").
    Command(&'static str),
    /// The tool is freehand (brush, pencil, eraser, smudge, clone): pointer only, a named exception.
    Freehand,
}

impl ToolPath {
    const fn reason(self) -> &'static str {
        match self {
            ToolPath::Command(reason) => reason,
            ToolPath::Freehand => "freehand input",
        }
    }
}

/// The style of a canvas: its surface comes from `ContainerProps`.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CanvasStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A canvas: a pasteboard the caller's canvas controls and the active tool share.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct Canvas {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    path: ToolPath,
    /// The view the canvas shows its document through. The body reads it as a [`CanvasSpace`] with
    /// [`canvas_space`]; its controls register parts in the canvas root's screen space through
    /// `CanvasView::to_screen`. The view sets no body transform: a body of components that keep canvas-space geometry
    /// (a node graph) also sets `transform(view.layer_transform())`, which exists for an unrotated, unmirrored,
    /// square view only.
    #[prop]
    view: Option<CanvasView>,
    wheel: Option<Wheel>,
}

impl Canvas {
    /// A canvas named `label` whose active tool is reached through `path`.
    pub fn new(label: impl Into<Label>, path: ToolPath) -> Self {
        let props = ContainerProps::new().surface(SurfaceKind::Canvas).arena(true).sizing(Sizing::Fill);
        Canvas { common: Common::new(), props, path, view: None, wheel: None }.label(label)
    }

    /// Shows the body zoomed and moved by `transform`, so controls and members keep canvas-space geometry.
    #[must_use]
    pub fn transform(mut self, transform: CanvasTransform) -> Self {
        self.props = self.props.transform(transform);
        self
    }
}

/// What a canvas reports: its response and the press the tool gets.
#[derive(Debug)]
#[non_exhaustive]
pub struct CanvasOutput {
    /// The canvas's response.
    pub response: Response,
    /// The press on the background part: a press no canvas control claimed. A middle-button press pans and is never
    /// a tool press, so this is idle while one runs.
    pub press: PressState,
    /// The wheel of this pass over the canvas: scroll in points and zoom factor, with the modifiers held.
    pub wheel: Option<Wheel>,
    /// How far a middle-button drag on the background moved the pointer this pass, in the canvas root's screen space.
    pub pan: Option<Vec2>,
}

impl CanvasOutput {
    /// Where a press on the background was released without a drag. A tool that creates geometry opens its command
    /// form at this point.
    pub fn click_at(&self) -> Option<Pos2> {
        match self.press.phase {
            PressPhase::Release { pos, click: true, .. } => Some(pos),
            _ => None,
        }
    }
}

impl Outcome for CanvasOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl Component for Canvas {
    const ROLE: Role = Role::CANVAS;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = CanvasStyle;
    type Output = CanvasOutput;

    fn style(_: &StyleCx<'_>, _: CanvasStyleKey) -> CanvasStyle {
        CanvasStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS | Sense::TAKE_FOCUS | Sense::WHEEL
    }

    /// The wheel share of this pass, kept for `finish`.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.wheel = cx.take_wheel();
    }

    /// The canvas's [`CanvasSpace`] for its body; a canvas without a view hides an outer canvas's.
    fn content_env(&self, cx: &Cx<'_, Self>, values: &mut crate::runtime::env::AppValues) {
        space::publish(values, self.view.map(|view| CanvasSpace::new(view, cx.env.scale().0)));
    }

    /// The background part the active tool gets. It sits in the canvas root's space, the parent layer's.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(BACKGROUND, rect)
            .priority(HitPriority::new(HitLevel::Background, 0))
            .sense(Sense::CLICK | Sense::DRAG)
            .pointer_only(self.path.reason());
    }

    fn finish(self, cx: Cx<'_, Self>) -> CanvasOutput {
        let press = cx.part(BACKGROUND).press;
        let middle = press.phase.start().is_some_and(|start| start.button == PointerButton::Middle);
        let (press, pan) = match press.phase {
            PressPhase::Drag { delta, .. } if middle => (PressState::IDLE, Some(delta)),
            _ if middle => (PressState::IDLE, None),
            _ => (press, None),
        };
        CanvasOutput { response: cx.response.clone(), press, wheel: self.wheel, pan }
    }
}

impl Container for Canvas {}
