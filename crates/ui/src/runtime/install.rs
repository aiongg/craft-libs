//! `runtime::install`: adds the runtime plugin once and sets the egui options the runtime owns.

use crate::runtime::egui::{focus, input, plugin};
use crate::runtime::frame;
use egui::Context;

/// What `install` takes.
#[derive(Default)]
#[non_exhaustive]
pub struct InstallSpec {
    /// The end hook's ghost-layer painter, called once per pass. The runtime names no contract type, so it
    /// holds an erased function; `craft_ui::install` fills it with the overlay host's painter.
    pub frame_layers: Option<fn(&Context)>,
}

impl InstallSpec {
    /// A spec with no ghost-layer painter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the end hook's ghost-layer painter.
    pub fn frame_layers(mut self, painter: Option<fn(&Context)>) -> Self {
        self.frame_layers = painter;
        self
    }
}

/// Why `install` refused a spec.
#[derive(Debug)]
#[non_exhaustive]
pub enum InstallError {
    /// The context already has the runtime plugin.
    AlreadyInstalled,
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::AlreadyInstalled => f.write_str("the runtime is already installed on this context"),
        }
    }
}

impl std::error::Error for InstallError {}

/// Adds the runtime plugin to `ctx` and sets the egui options the runtime owns: focus on press is the
/// runtime's, interface scale is a command, and quit goes through the close guard. A second call on the same
/// context is an error.
pub fn install(ctx: &Context, spec: InstallSpec) -> Result<(), InstallError> {
    if plugin::is_registered(ctx) {
        return Err(InstallError::AlreadyInstalled);
    }
    focus::never_surrender_focus(ctx);
    input::clear_shortcuts(ctx);
    frame::set_frame_layers(ctx, spec.frame_layers);
    plugin::register(ctx);
    Ok(())
}
