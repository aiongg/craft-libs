//! `ZoomField`: the one zoom field, a number field over a canvas view's zoom in percent.

use crate::contract::{NoValue, Outcome};
use crate::runtime::env::{Env, Label};
use craft_behavior::kurbo::Size;
use craft_behavior::{CanvasView, EditEvents};
use egui::{Response, Ui};

/// A zoom field: a `NumberField` over the view's device zoom in percent, with the zoom codec and the zoom ladder's
/// range. With no view it shows a disabled empty field. An edit writes the zoom about the view's centre; an empty
/// commit keeps the view.
#[derive(crate::contract::Pattern)]
pub struct ZoomField {
    label: Label,
    view: Option<CanvasView>,
    document: Size,
    stepper: bool,
    scrub: bool,
    enabled: bool,
}

impl ZoomField {
    /// A zoom field named `label` over `view` (`None` when no view is known) of a document of `document` units.
    pub fn new(label: impl Into<Label>, view: Option<CanvasView>, document: Size) -> Self {
        ZoomField { label: label.into(), view, document, stepper: false, scrub: false, enabled: true }
    }

    /// The arrow keys and the wheel step along the zoom ladder.
    #[must_use]
    pub fn stepper(mut self, on: bool) -> Self {
        self.stepper = on;
        self
    }

    /// The caller's side label scrubs the zoom.
    #[must_use]
    pub fn scrub(mut self, on: bool) -> Self {
        self.scrub = on;
        self
    }

    /// Whether the field takes input; it is disabled anyway while there is no view.
    #[must_use]
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }

    /// Shows the field and reports the view changes it made.
    pub fn show(self, ui: &mut Ui) -> ZoomFieldOutput {
        let _ = (&self.label, &self.view, self.document, self.stepper, self.scrub, self.enabled, ui);
        todo!("cw.C59")
    }
}

/// What a zoom field reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ZoomFieldOutput {
    /// The field's response.
    pub response: Response,
    /// The view changes of this pass.
    pub view: EditEvents<CanvasView>,
    /// A typed zoom outside the ladder's range was clamped, in the pass it committed.
    pub invalid_zoom: Option<InvalidZoom>,
}

impl Outcome for ZoomFieldOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The range a typed zoom missed, in percent.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InvalidZoom {
    /// The least zoom the field takes.
    pub low: f64,
    /// The greatest zoom the field takes.
    pub high: f64,
}

impl InvalidZoom {
    /// The message that says which zoom range the field takes, in the env's number format.
    pub fn message(&self, env: &Env) -> String {
        let _ = (self, env);
        todo!("cw.C59")
    }
}
