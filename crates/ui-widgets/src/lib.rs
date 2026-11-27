#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! The components of the craft UI library: buttons, choices, display, forms, lists, menus, notices, text fields, value
//! drags, pickers, tabs, tables, graphs and bars (layer 5).
//!
//! The root re-exports craft-ui-contract's modules, so components name them as `crate::runtime::…` and derive output
//! as `craft_ui::…`.
#![warn(missing_docs)]

// The derives' output names the crate as `craft_ui`, here as in every other crate.
extern crate self as craft_ui;

#[cfg(feature = "testing")]
pub use craft_ui_contract::registry;
#[doc(hidden)]
pub use craft_ui_contract::{__private, __register_command_set, __register_contract, __register_pattern};
pub use craft_ui_contract::{contract, fonts, layout, primitives, runtime, style, words};

// Root names the derives and components use as `craft_ui::…`, the same as the facade's.
#[doc(hidden)]
pub use craft_ui_contract::contract::{Context, Id, IdSalt, Response, Shown, Ui, UiBuilder};
#[doc(hidden)]
pub use craft_ui_contract::primitives::Color32;
// The contract's `Draw` and `PaintShape` still take egui's corner radius.
#[doc(hidden)]
pub use craft_ui_rt::{WindowCommand, WindowId};
#[doc(hidden)]
pub use egui::CornerRadius;

/// Geometry in logical points.
#[doc(hidden)]
pub mod geom {
    pub use craft_ui_rt::{Align, Pos2, Rangef, Rect, Vec2, lerp, pos2, vec2};
}

/// Keys, modifiers, pointer buttons and cursors.
#[doc(hidden)]
pub mod input {
    pub use craft_ui_rt::{CursorIcon, Key, Mods, PointerButton};
}

pub mod bars;
pub mod buttons;
pub mod choice;
pub mod controls;
pub mod display;
pub mod edit;
pub mod form;
pub mod graph;
pub mod layout_components;
pub mod list;
pub mod menu;
pub mod notices;
pub mod pickers;
pub mod popover;
#[cfg(feature = "suite")]
#[doc(hidden)]
pub mod story_kit;
pub mod table;
pub mod tabs;
pub mod text;
pub mod tile_grid;
pub mod value_drag;

/// The fixture, the suite runner and the story helpers, with the helpers that show components: `props` and
/// `story_props`.
#[cfg(feature = "testing")]
pub mod testing {
    #[cfg(feature = "suite")]
    pub use crate::story_kit::{props, story_props};
    pub use craft_ui_contract::testing::*;

    /// Pointer inputs for `Harness::send`.
    pub mod pointer {
        use craft_ui::geom::{Pos2, Vec2};
        use craft_ui::input::{Mods, PointerButton};
        use craft_ui_platform_headless::{Contact, Input};
        use craft_ui_rt::PointerPhase;

        /// The pointer moving to `pos`.
        pub fn moved(pos: Pos2) -> Input {
            Input::mouse_move(pos)
        }

        /// The primary button going down at `pos`.
        pub fn press(pos: Pos2) -> Input {
            Input::mouse_down(pos)
        }

        /// The primary button going up at `pos`.
        pub fn release(pos: Pos2) -> Input {
            Input::mouse_up(pos)
        }

        fn button(button: PointerButton, pos: Pos2, pressed: bool) -> Input {
            let phase = if pressed { PointerPhase::Down } else { PointerPhase::Up };
            Input::pointer(Contact { button, ..Contact::MOUSE }, phase, pos)
        }

        /// The secondary button going down or up at `pos`.
        pub fn secondary(pos: Pos2, pressed: bool) -> Input {
            button(PointerButton::Secondary, pos, pressed)
        }

        /// The middle button going down or up at `pos`.
        pub fn middle(pos: Pos2, pressed: bool) -> Input {
            button(PointerButton::Middle, pos, pressed)
        }

        /// A wheel step of `dy` points at `pos`.
        pub fn wheel(pos: Pos2, dy: f32) -> Input {
            Input::Wheel { pos, delta: Vec2::new(0.0, dy), mods: Mods::NONE }
        }
    }

    #[cfg(feature = "suite")]
    crate::registry!();
}
