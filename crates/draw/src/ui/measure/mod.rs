//! The Ruler tool ([`RulerTool`]), its options bar ([`RulerBar`]), and the distances between two
//! boxes that the canvas's [`SmartGuides`](craft_ui::canvas::SmartGuides) draws
//! ([`box_distances`]).
//!
//! The Ruler line is document state outside the undo history (PhotoCraft's `set_quiet`): the tool
//! reports `EditEvent<Option<RulerLine>>` and the app stores the line.

mod bar;
#[cfg(test)]
mod bar_tests;
mod distances;
#[cfg(test)]
mod distances_tests;
mod model;
#[cfg(test)]
mod model_tests;
pub(crate) mod stories;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use bar::{RulerBar, RulerBarLabels, RulerBarOutput};
pub use distances::box_distances;
pub use model::{MIN_LINE, RulerLine, RulerPart, RulerReading};
pub use style::MeasureStyle;
pub use tool::{RulerInput, RulerPass, RulerTool, RulerToolLabels};
