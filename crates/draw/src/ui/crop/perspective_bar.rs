//! [`PerspectiveCropBar`]: the Perspective Crop tool's options.

use super::{CropDocument, CropSizeFields};
use craft_ui::contract::{Bind, NoValue, Outcome};
use egui::{Response, Ui};

/// The Perspective Crop options: W × H × Resolution, Front Image, Clear, a divider and Show Grid.
#[must_use = "show the bar's children with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
pub struct PerspectiveCropBar<'a> {
    size: &'a mut CropSizeFields,
    document: &'a CropDocument,
    show_grid: Bind<'a, bool>,
}

/// What [`PerspectiveCropBar::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies what the bar reports"]
pub struct PerspectiveCropBarOutput {
    /// The strip's response.
    pub response: Response,
    /// A field changed.
    pub changed: bool,
    /// A typed value was out of range.
    pub out_of_range: Option<craft_ui::text::RangeAlert>,
}

impl Outcome for PerspectiveCropBarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl<'a> PerspectiveCropBar<'a> {
    /// The bar editing `size` over `document`, with Show Grid bound to `show_grid`.
    pub fn new(size: &'a mut CropSizeFields, document: &'a CropDocument, show_grid: Bind<'a, bool>) -> Self {
        PerspectiveCropBar { size, document, show_grid }
    }

    /// Shows the bar's children.
    pub fn show(self, ui: &mut Ui) -> PerspectiveCropBarOutput {
        let PerspectiveCropBar { size, document, show_grid } = self;
        let _ = (ui, size, document, show_grid);
        todo!("tl.T48")
    }
}
