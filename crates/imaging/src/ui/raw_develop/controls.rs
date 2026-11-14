//! The control column: sections of sliders over the settings.

use craft_ui::contract::{Bind, EditResponse, NoValue, Outcome, Pattern, Props};
use egui::{Response, Ui};

use super::Demosaic;
use crate::raw::RawDevelopSettings;

/// What the control column reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct RawDevelopPanelOutput {
    /// The column's response.
    pub response: Response,
    /// The edit of the settings.
    pub edit: EditResponse<RawDevelopSettings>,
    /// The Auto button was pressed.
    pub auto: bool,
}

impl Outcome for RawDevelopPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The control column: Light, Color, Effects, Curve, Color Mixer, Color Grading, Detail and Calibration sections. Each
/// row shows `Bind::given(f64::from(value))` and writes its edit back as `f32`.
#[must_use = "show the panel with `show`"]
#[derive(Pattern, Props)]
#[allow(dead_code)] // P3(dialogs-w18): dl.K67
pub struct RawDevelopPanel<'a> {
    settings: Bind<'a, RawDevelopSettings>,
    /// The demosaic select shows while a raw is being opened.
    #[prop]
    demosaic: Option<Demosaic>,
}

impl<'a> RawDevelopPanel<'a> {
    /// The panel over `settings`.
    pub fn new(settings: Bind<'a, RawDevelopSettings>) -> Self {
        let _ = settings;
        todo!("dl.K67")
    }

    /// Draws the sections.
    pub fn show(self, ui: &mut Ui) -> RawDevelopPanelOutput {
        let _ = (self, ui);
        todo!("dl.K67")
    }
}
