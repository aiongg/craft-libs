//! The viewer's chrome: the options bar, the panels, the status bar and the window that holds them.

mod describe;
mod options;
mod panels;
mod persist;
mod shell;

pub use options::{options_bar, status_bar};
pub use panels::Panels;
pub use persist::Persisted;
pub use shell::Gallery;
