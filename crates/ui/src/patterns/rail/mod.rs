//! Rails: `ToolRail` (slots of tools), `PanelRail` (icons that open panels) and the
//! `FlyoutButton` a tool slot is made of, drawn with the `rail.*` tokens.

pub(crate) mod flyout;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod flyout_tests;
mod gesture;
pub(crate) mod panel_rail;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_rail_tests;
pub(crate) mod tool;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tool_tests;

pub use flyout::{FlyoutButton, FlyoutItem, FlyoutResponse, FlyoutStyle};
pub use panel_rail::{PanelRail, PanelRailItem, PanelRailOutput, PanelRailRequest, PanelRailStyle};
pub use tool::{LastUsed, RailFooter, RailMetrics, RailTool, SlotId, ToolRail, ToolRailOutput, ToolSlot, shown_tool, visible_slots};
