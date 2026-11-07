//! `CanvasSpace`: the canvas view a member of a canvas body reads, and the text of ruler units.
//!
//! A `Canvas` given a view publishes its [`CanvasSpace`] to its body through `Component::content_env`. A canvas
//! control reads it with [`canvas_space`]; the canvas's tool reads it with `ToolContext::space`, handed over by
//! `CanvasOutput::space`.

use crate::runtime::env::{AppValues, EnvValue, EnvView, UnitKind, Units};
use craft_behavior::canvas::zoom;
use craft_behavior::{CanvasView, RulerUnits};
use std::ops::RangeInclusive;

/// How a canvas shows its document, as its body and its tool see it: the view, and the interface scale it is
/// shown at.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasSpace {
    /// The view: document to the canvas root's screen space.
    pub view: CanvasView,
    /// Device pixels per point.
    pub pixels_per_point: f32,
}

impl CanvasSpace {
    /// The space of `view` at `pixels_per_point` device pixels per point.
    pub fn new(view: CanvasView, pixels_per_point: f32) -> Self {
        Self { view, pixels_per_point }
    }

    /// Device pixels per document unit (1 is 100 %) of the view at this scale.
    pub fn zoom_device(&self) -> f64 {
        zoom::device(&self.view, self.pixels_per_point)
    }
}

/// The env value that carries the space of the innermost canvas; `None` outside a canvas and inside a canvas that
/// has no view.
#[derive(Clone, Copy, Debug, PartialEq)]
struct InCanvas(Option<CanvasSpace>);

impl EnvValue for InCanvas {
    fn root() -> Self {
        InCanvas(None)
    }
}

/// Publishes `space` (or none) to a canvas's body.
pub(super) fn publish(values: &mut AppValues, space: Option<CanvasSpace>) {
    values.set(InCanvas(space));
}

/// The space of the innermost `Canvas` around the component that has a view, or `None` outside one.
///
/// A canvas control registers its parts in the canvas root's screen space: it maps document geometry with
/// `space.view.to_screen` (and screen positions back with `to_doc`) and shows itself at the canvas rect, so it works
/// under any rotation, flip or aspect. The body's layer transform exists only for an unrotated, unmirrored,
/// square view (`CanvasView::layer_transform`), for bodies of node-graph components that keep canvas-space geometry;
/// a control that registers in screen space must not also read through it.
pub fn canvas_space(env: &EnvView<'_>) -> Option<CanvasSpace> {
    env.get::<InCanvas>().0
}

/// The text of one axis' ruler units: its tick maths with the unit kind it is written in, over `Units` (the one
/// number formatter of the env).
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RulerText {
    /// The ticks and the size of the unit.
    pub units: RulerUnits,
    /// What the unit counts, which gives its spelling and default decimals.
    pub kind: UnitKind,
}

impl RulerText {
    /// Ruler `units` written as `kind`.
    pub const fn new(units: RulerUnits, kind: UnitKind) -> Self {
        Self { units, kind }
    }

    /// The distance of `position` (document units) from `zero`, with the unit and between the least and most digits
    /// of `decimals`: `103.5 px`.
    pub fn format(&self, numbers: &Units, position: f64, zero: f64, decimals: RangeInclusive<usize>) -> String {
        numbers.text(self.kind, self.units.distance(position, zero), decimals)
    }

    /// [`format`](Self::format) for a readout or a value: at least one digit after the decimal mark, so pixels show
    /// tenths (PhotoCraft's `X: 103.2 px`), other units at the digits of their kind.
    pub fn format_value(&self, numbers: &Units, position: f64, zero: f64) -> String {
        let digits = (*self.kind.decimals().end()).max(1);
        self.format(numbers, position, zero, digits..=digits)
    }

    /// The number a ruler prints at the tick `value` (ruler units from the zero point) of labelled step `step`, with
    /// no unit.
    pub fn tick_label(&self, numbers: &Units, value: f64, step: f64) -> String {
        let digits = self.units.label_decimals(step);
        numbers.number_ungrouped(self.units.label_value(value, step), digits..=digits)
    }

    /// The unit's short name: `px`.
    pub fn unit_name(&self, numbers: &Units) -> String {
        numbers.symbol(self.kind)
    }
}
