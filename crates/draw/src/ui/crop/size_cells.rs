//! The W × H × Resolution cells both crop bars share, and the rules on [`CropSizeFields`].

use super::{CropDocument, CropSizeFields, CropBarWords};
use craft_ui::runtime::env::Units;
use egui::Ui;

/// The ranges of the cells' fields.
#[allow(dead_code)] // P3(tools-w14): tl.T47
pub(super) struct SizeLimits {
    /// The length range.
    pub length: (f64, f64),
    /// The resolution range.
    pub resolution: (f64, f64),
}

/// What the cells reported this pass.
#[allow(dead_code)] // P3(tools-w14): tl.T47
pub(super) struct SizeCells {
    /// A field changed.
    pub changed: bool,
    /// A typed value was out of range.
    pub out_of_range: Option<craft_ui::text::RangeAlert>,
}

#[allow(dead_code)] // P3(tools-w14): tl.T47
impl SizeLimits {
    /// The crop bar's ranges, in the field's own unit.
    pub(super) const CROP: Self = Self { length: (0.0, 0.0), resolution: (0.0, 0.0) };
    /// The perspective bar's ranges, in pixels.
    pub(super) const PERSPECTIVE: Self = Self { length: (0.0, 0.0), resolution: (0.0, 0.0) };
}

/// One W × H × Resolution cell group.
#[allow(dead_code)] // P3(tools-w14): tl.T47
pub(super) fn show(ui: &mut Ui, size: &mut CropSizeFields, limits: SizeLimits, document: &CropDocument, units: &Units, words: &CropBarWords) -> SizeCells {
    let _ = (ui, size, limits, document, units, words);
    todo!("tl.T47")
}

/// The size a crop commit resamples to.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropSizeTarget {
    /// The width in pixels.
    pub width: Option<f64>,
    /// The height in pixels.
    pub height: Option<f64>,
    /// The resolution in px/in.
    pub resolution: Option<f64>,
}

impl CropSizeFields {
    /// Swaps W and H.
    pub fn swap(&mut self) {
        todo!("tl.T47")
    }

    /// Fills W, H and the resolution from `document`; with `limits` the resolution is rounded
    /// inward to 3 decimals.
    pub fn front_image(&mut self, document: &CropDocument, limits: Option<(f64, f64)>) {
        let _ = (document, limits);
        todo!("tl.T47")
    }

    /// Empties W, H and the resolution and resets the resolution unit to pixels per inch.
    pub fn clear_size(&mut self) {
        todo!("tl.T47")
    }

    /// The target the fields name.
    pub fn target(&self, document: &CropDocument, units: &Units) -> CropSizeTarget {
        let _ = (document, units);
        todo!("tl.T47")
    }
}
