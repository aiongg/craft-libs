//! [`VanishingPointCommand`]: what OK answers, in the params shape of `filter.vanishingPoint`.

use serde::Serialize;

use super::editor::{PasteImage, StampStroke, VanishingPointSettings};
use super::planes::VanishingPlane;

/// The engine command a Vanishing Point OK answers: the planes, the paste, the clone strokes and the bar's options.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VanishingPointCommand {
    /// Every plane.
    pub planes: Vec<VanishingPlane>,
    /// The pasted image, when the session has one.
    pub paste: Option<PasteImage>,
    /// The clone strokes, in order.
    pub strokes: Vec<StampStroke>,
    /// The bar's options.
    #[serde(flatten)]
    pub settings: VanishingPointSettings,
}
