//! The docking pattern: `DockArea` and `DockPanels` on `SplitArea` and `DockGroup`, and `PaneArea` for task panes.

mod words;

// Task panes: docked panes as shell panes, floating panes as floating frames.
mod panes;

pub use panes::{
    PaneArea, PaneAreaOutput, PaneCommand, PaneContent, PaneFrame, PaneFrameOutput, PaneFrameStyle, PanePlace, PaneRequest, PaneRoot, PaneRootOutput,
    PaneRootStyle, PaneSpec, PaneState, SidePolicy,
};
