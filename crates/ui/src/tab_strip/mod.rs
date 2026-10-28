//! Tabs and tab strips.
//!
//! - [`Tab`]: one tab as data the host passes each frame (key, label, optional detail text, icon,
//!   modified mark, closable, progress). [`Tab::show_at`] draws it at a rect in the state a
//!   strip gives it ([`TabView`]) and reports its × part; `tab.*` are its tokens, dumped from
//!   [`Tab::style`] ([`TabStyle`]).

mod tab;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tab_tests;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Tab"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    tab::stories()
}

pub use tab::{Tab, TabLayout, TabMetrics, TabParts, TabResponse, TabStripText, TabStyle, TabView};
