#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! The craft UI library: runtime, style, fonts, primitives, the component contract, layout,
//! components, patterns and the shell (layers 2 to 7).

// The derives' output names the crate as `craft_ui`, here as in every other crate.
extern crate self as craft_ui;

pub use craft_ui_macros::words;

pub mod bars;
pub mod buttons;
pub mod canvas;
pub mod canvas_controls;
pub mod choice;
pub mod contract;
pub mod dialog;
pub mod display;
pub mod docking;
pub mod edit;
pub mod fonts;
pub mod form;
pub mod graph;
pub mod layout;
pub mod layout_components;
pub mod list;
pub mod menu;
pub mod notices;
pub mod patterns;
pub mod pickers;
pub mod popover;
pub mod primitives;
pub mod rails;
pub mod runtime;
pub mod sections;
pub mod shell;
pub mod style;
pub mod table;
pub mod tabs;
#[cfg(feature = "testing")]
pub mod testing;
pub mod text;
pub mod tile_grid;
pub mod value_drag;

pub use runtime::{InstallError, InstallSpec};

/// Adds the runtime to `ctx`: `runtime::install` with the overlay host's ghost-layer painter in the end hook.
pub fn install(ctx: &egui::Context, spec: InstallSpec) -> Result<(), InstallError> {
    runtime::install(ctx, spec.frame_layers(Some(contract::overlay::paint_frame_layers)))
}

#[doc(hidden)]
pub mod __private {
    pub use egui;
    pub use linkme;
}
