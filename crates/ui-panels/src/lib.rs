#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! The panels of the craft UI library: sections, dialogs, docking and rails (layer 5), and the canvas data that
//! patterns read without the canvas.
//!
//! The root re-exports craft-ui-widgets' modules and the ones it re-exports.
#![warn(missing_docs)]

// The derives' output names the crate as `craft_ui`, here as in every other crate.
extern crate self as craft_ui;

#[cfg(feature = "testing")]
pub use craft_ui_widgets::registry;
#[cfg(feature = "suite")]
#[doc(hidden)]
#[cfg(spike_full)]
pub use craft_ui_widgets::story_kit;
#[doc(hidden)]
pub use craft_ui_widgets::{__private, __register_command_set, __register_contract, __register_pattern};
#[cfg(spike_full)]
pub use craft_ui_widgets::{bars, edit, graph, table, tabs};
pub use craft_ui_widgets::{
    buttons, choice, controls, display, form, layout_components, list, menu, notices, pickers, popover, text, tile_grid, value_drag,
};
pub use craft_ui_widgets::{contract, fonts, layout, primitives, runtime, style, words};

#[doc(hidden)]
#[cfg(spike_full)]
pub mod canvas_data;
#[cfg(spike_full)]
mod collapse;
#[cfg(spike_full)]
pub mod dialog;
#[cfg(spike_full)]
pub mod docking;
#[cfg(spike_full)]
pub mod rails;
pub mod sections;
#[cfg(feature = "testing")]
mod testing_internals;

/// The fixture, the suite runner and the story helpers, with `internals`: crate-private logic the trials check
/// without a frame.
#[cfg(feature = "testing")]
pub mod testing {
    pub use craft_ui_widgets::testing::*;

    pub mod internals {
        //! Crate-private logic the trials check without a frame.
        pub use crate::testing_internals::*;
    }

    #[cfg(feature = "suite")]
    crate::registry!();
}
