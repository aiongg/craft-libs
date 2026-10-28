#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! Small, reusable egui components for the Craft frontends.
//!
//! Applications own their assets, themes, layouts, commands and document semantics. Components
//! accept focused style values and return egui responses; no application engine is required.

pub mod behavior;
pub mod buttons;
pub mod canvas;
pub mod capture;
pub mod combo;
pub mod composites;
pub mod controls;
pub mod direction;
pub mod docking;
pub mod edit;
mod focus;
pub mod fonts;
pub mod icons;
pub mod ime;
pub mod inspect;
pub mod layout;
pub mod list;
pub mod localized;
pub mod menu;
pub mod panes;
mod popup_list;
pub mod primitives;
pub mod printing;
pub mod story;
pub mod tab_strip;
pub mod tabs;
pub mod testing;
pub mod text;
pub mod theme;

#[allow(deprecated, reason = "re-exports the deprecated combo box for the apps that still use it")]
pub use combo::ComboBox;
