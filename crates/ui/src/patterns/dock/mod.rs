//! The docking pattern: `DockArea` and `DockPanels` on `SplitArea` and `DockGroup`.

mod words;

mod area;
mod hide;
mod style;

pub use area::{DockArea, DockAreaOutput, InvalidGeometry};
pub use style::DockAreaStyle;
