//! Rails: `ToolRail` (slots of tools), `PanelRail` (icons that open panels) and the
//! `FlyoutButton` a tool slot is made of, drawn with the `rail.*` tokens.

pub(crate) mod flyout;
pub(crate) mod panel_rail;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_rail_tests;
pub(crate) mod tool;

pub use panel_rail::{PanelRail, PanelRailItem, PanelRailOutput, PanelRailRequest, PanelRailStyle};
