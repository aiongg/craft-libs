//! A Storybook-like gallery of `craft_ui::story` stories, themed with `craft_ui::theme`.
//!
//! The left panel lists stories by level and component, the top bar sets the appearance, the
//! right panel edits the story's args and the centre shows the story, or a grid of appearances
//! in compare view. The "Tokens" toggle in the bar swaps all three for the token browser.

mod appearance;
mod browser;
pub mod faces;
mod gallery;
mod persist;

pub use appearance::Appearance;
pub use gallery::Gallery;
