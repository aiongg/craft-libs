//! The docking pattern: `DockArea` and `DockPanels` on `SplitArea` and `DockGroup`.

mod words;

mod area;
mod hide;
mod style;
mod switcher;

pub use area::{DockArea, DockAreaOutput, InvalidGeometry};
pub use style::DockAreaStyle;
pub use switcher::{WorkspaceSwitcher, WorkspaceSwitcherOutput};
