//! `Bar` and `BarItems`: the bar container and its strip of command items.

mod bar;
mod items;
#[cfg(feature = "testing")]
mod stories;
mod words;

pub use bar::{Bar, BarKind, BarOutput, BarStyle};
pub use items::{BarItem, BarItems, BarItemsStyle, Fit, FitItem, FitKeep, ItemDisplay, fit_items};
pub use words::BarWords;
