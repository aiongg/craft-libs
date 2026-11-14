//! The Color Range dialog's body.

use craft_ui::canvas::RasterSource;
use craft_ui::contract::{Bind, EditResponse, NoValue, Outcome, Pattern, Props};
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

use super::ColorRangeValue;

/// What the Color Range body reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ColorRangeOutput {
    /// The edit of the value.
    pub edit: EditResponse<ColorRangeValue>,
    /// The body's response.
    pub response: Response,
}

impl Outcome for ColorRangeOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Color Range body: the select, the sliders, the selection preview and the sampling tools.
#[must_use = "show the dialog body with `show`"]
#[derive(Pattern, Props)]
#[allow(dead_code)] // P3(dialogs-w9): dl.K54
pub struct ColorRangeDialog<'a> {
    label: Label,
    value: Bind<'a, ColorRangeValue>,
    /// The mask the preview shows.
    #[prop]
    mask: Option<&'a dyn RasterSource>,
    /// The picture the preview shows.
    #[prop]
    image: Option<&'a dyn RasterSource>,
    /// The integer proxy factor `k`, halved by integer division.
    #[prop]
    picture_scale: u32,
    /// An error to show under the preview.
    #[prop]
    error: Option<Label>,
    /// The canvas the sampling presses reach.
    #[prop]
    canvas: Option<egui::Id>,
}

impl<'a> ColorRangeDialog<'a> {
    /// The body named `label` over `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, ColorRangeValue>) -> Self {
        let _ = (label.into(), value);
        todo!("dl.K54")
    }

    /// Draws the body.
    pub fn show(self, ui: &mut Ui) -> ColorRangeOutput {
        let _ = (self, ui);
        todo!("dl.K54")
    }
}
