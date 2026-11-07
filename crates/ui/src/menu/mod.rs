//! Menu presets with `MenuItem` rows, menu-bar keys and mnemonics.

pub mod bar;
mod edit;
pub(crate) mod item;
mod panel;
mod popup;
#[cfg(feature = "testing")]
mod stories;
mod words;

pub use bar::{HelpEntry, MenuBar, MenuBarOutput, MenuBarStyle, MenuHelp, MenuTitle};
pub use edit::{EditMenu, EditMenuOutput};
pub use item::{MenuItem, MenuItemOutput, MenuItemStyle};
pub use panel::{GroupItem, PanelMenu, PanelMenuOutput, PanelRow};
pub use popup::{Menu, MenuOutput, MenuPick, MenuStep, MenuStyle};
pub(crate) use popup::{entry_rows, layer, pick_values_from};
pub use words::MenuWords;
