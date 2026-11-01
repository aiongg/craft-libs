//! The Channels panel over plain rows: thumbnails, eyes, selection, rename, drag, the footer and
//! the menus. It reports events and holds no document (`ChannelsPanel`, `ChannelRow`,
//! `ChannelsEvent`).

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod channels_tests;
mod footer;
mod labels;
mod menu;
mod model;
mod panel;
mod rows;
pub(crate) mod stories;
mod style;

pub use footer::ChannelsFooterButton;
pub use labels::ChannelsLabels;
pub use menu::ChannelsCommand;
pub use model::{ChannelKind, ChannelRow, ChannelsEvent, ChannelsOutput, ColorIndicates, OVERLAY_OPACITIES, OverlayPreset, targeted_row};
pub use panel::ChannelsPanel;
pub use style::ChannelsStyle;
