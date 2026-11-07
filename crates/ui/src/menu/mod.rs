//! Menu presets with `MenuItem` rows, menu-bar keys and mnemonics.

pub mod bar;
mod edit;
mod item;
mod panel;
mod popup;
#[cfg(feature = "testing")]
mod stories;
mod words;

pub use bar::{HelpEntry, MenuBar, MenuBarOutput, MenuBarStyle, MenuHelp, MenuTitle};
pub use edit::EditMenu;
pub use item::{MenuItem, MenuItemOutput, MenuItemStyle};
pub use panel::{GroupItem, PanelMenu, PanelRow};
pub use popup::{Menu, MenuOutput, MenuPick, MenuStyle};
pub(crate) use popup::{entry_rows, layer, pick_from};
pub use words::{MenuBarWords, PanelMenuWords};
