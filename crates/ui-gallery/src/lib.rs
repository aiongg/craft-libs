//! A Storybook-like gallery of `craft_ui::story` stories, themed with `craft_ui::theme`.
//!
//! The options bar at the top sets the appearance and the view. Below it, dockable panels:
//! Components (a flat list under a label per level), Stories, Properties (the story's args, and
//! the compare options) and Inspect, and in the centre the Canvas tab (the story, or a grid of
//! appearances in compare view) and the Tokens tab (the token browser). Inspect mode is on while
//! the Inspect tab is shown: it draws padding, hit areas and focus order over the story and lists
//! its AccessKit tree.

mod appearance;
mod browser;
#[cfg(feature = "capture")]
pub mod capture;
pub mod faces;
mod fields;
mod gallery;
mod inspector;
mod layout;
mod persist;

pub use appearance::Appearance;
pub use gallery::Gallery;
