//! The automation widgets, for egui. Every target.
//!
//! - [`actions`]: the Actions panel.

pub mod actions;

pub use actions::ActionsLabels;

/// The labels tables of the automation widgets, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] =
    &[<actions::ActionsLabels as craft_ui::localized::Labels>::WORDS, <actions::stories::ActionsStoryLabels as craft_ui::localized::Labels>::WORDS];

/// The count words of the automation widgets, for the words registry.
pub const COUNTS: &[craft_ui::localized::words::Counted] = actions::COUNTS;
