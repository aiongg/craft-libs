//! The viewer's chrome: the options bar, the dock of panels and the window that holds them.

mod describe;
mod dock;
mod options;
mod overview;
mod panels;
mod persist;
mod shell;

pub use options::options_bar;
pub use panels::Panels;
pub use persist::Persisted;
pub use shell::Gallery;
