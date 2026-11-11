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
pub(crate) mod canvas_data;
pub mod choice;
pub mod contract;
pub mod controls;
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
#[cfg(feature = "testing")]
mod story_kit;
pub mod style;
pub mod table;
pub mod tabs;
#[cfg(feature = "testing")]
pub mod testing;
#[cfg(feature = "testing")]
mod testing_internals;
pub mod text;
pub mod tile_grid;
pub mod value_drag;

// split: every crate root re-exports these from craft-ui-contract
pub use contract::install;
pub use runtime::{InstallError, InstallSpec};

#[doc(hidden)]
pub mod __private {
    pub use egui;
    pub use inventory;
}
