//! `runtime::install`: adds the runtime plugin once and sets the egui options the runtime owns.

use crate::runtime::egui::{focus, input, plugin};
use crate::runtime::{commands, frame};
use crate::style::{DuplicateStyle, StyleReplacements};
use egui::Context;

/// What `install` takes.
#[non_exhaustive]
pub struct InstallSpec {
    /// The end hook's ghost-layer painter, called once per pass. The runtime names no contract type, so it
    /// holds an erased function; `craft_ui::install` fills it with the overlay host's painter.
    pub frame_layers: Option<fn(&Context)>,
    /// The app's style replacements, frozen at install.
    pub looks: StyleReplacements,
}

impl InstallSpec {
    /// A spec with no ghost-layer painter.
    #[expect(clippy::new_without_default, reason = "`new` takes the shell info and window system once they exist, so the spec has no default")]
    pub fn new() -> Self {
        InstallSpec { frame_layers: None, looks: StyleReplacements::new() }
    }

    /// Sets the app's style replacements.
    pub fn looks(mut self, looks: impl Into<StyleReplacements>) -> Self {
        self.looks = looks.into();
        self
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
    /// Two style replacements name one style key.
    DuplicateStyle(DuplicateStyle),
    /// A command set's definitions are unusable, or two commands share an id.
    Commands(commands::CommandDefError),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::AlreadyInstalled => f.write_str("the runtime is already installed on this context"),
            InstallError::DuplicateStyle(DuplicateStyle(key)) => write!(f, "two style replacements for the style key {key}"),
            InstallError::Commands(error) => write!(f, "command registry: {error}"),
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
    let looks = spec.looks.freeze().map_err(InstallError::DuplicateStyle)?;
    commands::install(ctx).map_err(InstallError::Commands)?;
    looks.store(ctx);
    focus::never_surrender_focus(ctx);
    input::clear_shortcuts(ctx);
    frame::set_frame_layers(ctx, spec.frame_layers);
    frame::seed_hooked_passes(ctx);
    plugin::register(ctx);
    Ok(())
}
