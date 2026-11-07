//! The window host's fixtures: install inputs that open no OS windows, and the scripted system.

use crate::runtime::InstallSpec;
use craft_host::target::ShellInfo;
use std::sync::Arc;

pub use crate::runtime::egui::fixture::{FullOutput, RawInput, dragged_id, record_input_hooks};

pub use craft_window::system::{FakeSystem, NoWindows};

/// An install spec for the running build's shell and a system with no OS services.
pub fn install_spec() -> InstallSpec {
    InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows))
}
