//! Painting symmetry: the axes ([`Symmetry`]), the guides on the canvas ([`SymmetryOverlay`]),
//! the Transform Symmetry session ([`SymmetryTransform`]) and the options bar's menu button
//! ([`SymmetryMenu`]).
//!
//! Stroke reflection, path symmetry and the brush engine stay in the app's paint engine; the
//! library carries only the axis geometry the screens need (lead call 22).

mod menu;
#[cfg(test)]
mod menu_tests;
mod model;
#[cfg(test)]
mod model_tests;
mod overlay;
pub(crate) mod stories;
mod style;
mod transform;
#[cfg(test)]
mod transform_tests;

pub use menu::{SymmetryChoice, SymmetryCommand, SymmetryLabels, SymmetryMenu, SymmetryMenuResponse, SymmetryPathRow};
pub use model::{MAX_COORDINATE, Symmetry, SymmetryMode, TURN_STEP_DEGREES};
pub use overlay::SymmetryOverlay;
pub use style::SymmetryStyle;
pub use transform::{SymmetryEvent, SymmetryInput, SymmetryTransform, SymmetryTransformOutput, SymmetryTransformPass};
