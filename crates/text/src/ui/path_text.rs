//! `PathTextMarkers`: the start, end and overflow marks of text laid on a path, a canvas control.

use craft_behavior::Metrics;
use craft_behavior::kurbo::{Point, Vec2 as PathVec};
use craft_ui::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, Outcome, Props as _, Role};
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Response, Vec2};

/// The path the text runs along, measured by arc length.
pub trait PathTrack {
    /// The path's length in document units.
    fn length(&self) -> f64;

    /// Whether the path is closed, so a range may wrap past its end.
    fn closed(&self) -> bool;

    /// The point at arc length `s` and the direction of the path there, or `None` off the path.
    fn at(&self, s: f64) -> Option<(Point, PathVec)>;

    /// The arc length of the path's point nearest `p`; `near` prefers the candidate closest to that arc length.
    fn project(&self, p: Point, near: Option<f64>) -> Option<f64>;
}

/// The part of the path the text fills: arc lengths of its start and end, and whether it runs against the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathRange {
    /// The arc length of the text's start.
    pub start: f64,
    /// The arc length of the text's end.
    pub end: f64,
    /// The text sits on the other side of the path and runs backwards.
    pub flip: bool,
}

/// The part of the markers under the pointer or being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathGrab {
    /// The start mark.
    Start,
    /// The end mark.
    End,
    /// The path between the marks.
    Body,
}

/// The style of the markers.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PathTextMarkersStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The start and end marks of type on a path, and the overflow mark when the text does not fit.
///
/// The value is the [`PathRange`]. A drag of a mark moves that end, a drag of the body slides the range, and a drag
/// across the path flips the text to its other side.
#[must_use = "show the markers with `show`"]
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(canvas_control)]
#[allow(dead_code)] // P3(tools-w4): tl.T18
pub struct PathTextMarkers<'a> {
    #[common]
    common: Common,
    range: Bind<'a, PathRange>,
    track: &'a dyn PathTrack,
    /// The arc length the text's anchor sits at, for text that is centred or right aligned.
    #[prop]
    anchor: f64,
    /// The text overflows the range: the end mark shows a plus.
    #[prop]
    overflow: bool,
    /// The text's body as quads (four corners each), for the grab of the whole range.
    #[prop]
    body: &'a [[Point; 4]],
    /// The markers show but take no input.
    #[prop]
    locked: bool,
}

impl<'a> PathTextMarkers<'a> {
    /// The markers named `label` over `range` on `track`.
    pub fn new(label: impl Into<Label>, range: Bind<'a, PathRange>, track: &'a dyn PathTrack) -> Self {
        PathTextMarkers { common: Common::new(), range, track, anchor: 0.0, overflow: false, body: &[], locked: false }.label(label)
    }
}

impl Component for PathTextMarkers<'_> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = PathTextMarkersStyle;
    type Output = PathTextMarkersOutput;

    fn style(_: &StyleCx<'_>, _: PathTextMarkersStyleKey) -> PathTextMarkersStyle {
        PathTextMarkersStyle { metrics: Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn bind(&mut self) -> Option<BindMut<'_, PathRange>> {
        Some(self.range.reborrow())
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        todo!("tl.T18")
    }

    fn finish(self, cx: Cx<'_, Self>) -> PathTextMarkersOutput {
        PathTextMarkersOutput { range: cx.edit_response(), hovered: None }
    }
}

/// What the markers report.
#[derive(Debug)]
#[non_exhaustive]
pub struct PathTextMarkersOutput {
    /// The range as the user drags it (`Begin`, `Update`, `Commit`) or a key sets it.
    pub range: EditResponse<PathRange>,
    /// The part under the pointer.
    pub hovered: Option<PathGrab>,
}

impl Outcome for PathTextMarkersOutput {
    type Value = PathRange;

    fn response(&self) -> &Response {
        &self.range.response
    }
}
