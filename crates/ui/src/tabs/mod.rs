//! `TabStrip`, `Tab` and `DocumentTabs`.

mod words;

// Tab and the strip rules.
mod rules;
mod tab;

pub use rules::{CloseSide, FitMode, OverflowMenu, OverflowPlace, TabKind, close_side};
pub use tab::{Tab, TabOutput, TabPartEvent, TabStyle};
pub use words::TabWords;
