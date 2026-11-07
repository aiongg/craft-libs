//! `PanelRail`, `ToolRail`, `FlyoutButton` and `ToolbarDrawer`.

mod words;

// The tool rail and its flyout button.
mod flyout;
mod tool;

pub use flyout::{FlyoutButton, FlyoutButtonStyle, FlyoutOutput, SlotEdit};
pub use tool::{
    DropAt, LastUsed, RailFooter, RailTool, SlotId, ToolDrag, ToolDropped, ToolRail, ToolRailOutput, ToolRailStyle, ToolSlot, column_toggle_icon,
    drop_in_slot, remembered, shown_tool, visible_slots,
};
pub use words::RailWords;
