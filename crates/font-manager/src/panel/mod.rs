//! The egui interface: the installed fonts, the Google Fonts catalogue, and installing and
//! removing through the font store.

mod i18n;

mod app;
mod faces;
mod jobs;

pub use app::{FontsPanel, Options, Settings, Tab, install_interface_fonts};
pub use jobs::Runner;
