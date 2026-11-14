//! [`StrokeCells`]: the stroke width cell and the Stroke style button, shared by the Shape bar and the Pen tools'
//! Shape mode.

use super::{ShapeAppearance, ShapeChange};
use craft_behavior::EditEvent;
use craft_ui::contract::Bind;
use egui::Ui;

/// The stroke width cell and the Stroke style button of a shape's stroke, as one pattern shown as children of a
/// `Bar`.
#[must_use = "show the cells with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct StrokeCells<'a> {
    appearance: &'a ShapeAppearance,
    pixels_per_inch: f64,
    corners: bool,
    stroke_open: Option<Bind<'a, bool>>,
}

impl<'a> StrokeCells<'a> {
    /// The cells for `appearance`, in a document of `pixels_per_inch`.
    pub fn new(appearance: &'a ShapeAppearance, pixels_per_inch: f64) -> Self {
        StrokeCells { appearance, pixels_per_inch, corners: true, stroke_open: None }
    }

    /// Whether the style popover offers corner joins; off for an ellipse.
    pub fn corners(mut self, on: bool) -> Self {
        self.corners = on;
        self
    }

    /// The Stroke style popover's open flag; without it the button is not shown.
    pub fn stroke_open(mut self, open: Bind<'a, bool>) -> Self {
        self.stroke_open = Some(open);
        self
    }

    /// Adds the cells to the bar's row `ui` and reports each change.
    pub fn show(self, ui: &mut Ui) -> Vec<EditEvent<ShapeChange>> {
        let StrokeCells { appearance, pixels_per_inch, corners, stroke_open } = self;
        let _ = (ui, appearance, pixels_per_inch, corners, stroke_open);
        todo!("tl.T29")
    }
}
