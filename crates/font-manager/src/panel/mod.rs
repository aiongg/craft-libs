//! The egui interface: the installed fonts, the Google Fonts catalogue, and installing and
//! removing through the font store.

mod i18n;

mod app;
mod browse;
mod details;
mod faces;
mod jobs;
mod look;
mod samples;
mod sources;
mod widgets;

pub use app::{FontsPanel, FontsPanelSettings, Options, Preset, Tab, install_interface_fonts};
pub use look::{Look, Palette};
