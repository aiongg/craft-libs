//! [`SampleSizeSelect`]: the Sample Size cell and setting.

use craft_ui::contract::{Bind, EditResponse, Pattern};
use egui::Ui;

use super::SampleSize;

/// One Sample Size cell and setting: a `Select` over [`SampleSize::ALL`] named with `EyedropperWords::size`, as
/// `EyedropperBar` draws it.
#[must_use = "show the select with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(tools-w12): tl.T40
pub struct SampleSizeSelect<'a> {
    size: Bind<'a, SampleSize>,
}

impl<'a> SampleSizeSelect<'a> {
    /// The select over `size`.
    pub fn new(size: Bind<'a, SampleSize>) -> Self {
        let _ = size;
        todo!("tl.T40")
    }

    /// Draws the select and reports the pick.
    pub fn show(self, ui: &mut Ui) -> EditResponse<SampleSize> {
        let _ = (self, ui);
        todo!("tl.T40")
    }
}
