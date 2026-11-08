//! `DockAreaStyle`: the sizes a dock area reads, resolved once from the theme's tokens.

use crate::docking::DockSizes;
use crate::style::ComponentStyle;
use craft_behavior::Metrics;

/// The style of a dock area: the docking sizes of its drop bands, columns, groups and floating windows. Colours,
/// surfaces and shapes belong to the parts the area assembles.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DockAreaStyle {
    #[style(metrics)]
    pub(super) metrics: Metrics,
    /// The docking sizes: bands, zones, least extents and the float window.
    pub(super) sizes: DockSizes,
}
