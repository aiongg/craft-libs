#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! Craft Font Manager as a library: the bundled font indexes and searching them, the installed
//! fonts by family, and the font store interface installs and removals go through.
//!
//! With the `ui` feature, [`FontsPanel`] is the whole interface as one panel that fills the `Ui`
//! it is given, with no window, menu or eframe of its own, so ArtCraft Launcher can show it as its
//! Fonts tab and the standalone app as its window. The host passes in the catalog, the store and
//! the index ([`Options`]) and installs a `craft_host::Host`: the panel keeps its
//! [`FontsPanelSettings`] in the Host's settings, runs its jobs through the Host's job service and
//! follows the Host's language. A host reads or sets the download consent through the panel.
//!
//! Also on `ui`: [`glyphs::GlyphsPanel`], [`missing::MissingFontsDialog`] and [`axes::AxisSliders`];
//! on `font-picker`: [`picker::FontPicker`], the font family field for text tools and panels.

pub mod filter;
pub mod index;
pub mod installed;
pub mod store;

#[cfg(feature = "ui")]
pub mod axes;
#[cfg(all(test, feature = "ui"))]
mod axes_tests;
#[cfg(feature = "ui")]
pub mod glyphs;
#[cfg(all(test, feature = "ui"))]
mod glyphs_tests;
#[cfg(feature = "ui")]
pub mod missing;
#[cfg(all(test, feature = "ui"))]
mod missing_tests;
#[cfg(feature = "ui")]
mod panel;
#[cfg(feature = "ui")]
pub mod picker;
#[cfg(all(test, feature = "ui"))]
mod picker_tests;
#[cfg(feature = "ui")]
mod text_stories;
#[cfg(feature = "ui")]
pub mod ui;

pub use craft_font_catalog;
pub use craft_font_catalog::{Catalog, FaceInfo, FontEnv, Source};
#[cfg(feature = "ui")]
pub use panel::{FontsPanel, FontsPanelSettings, Look, Options, Palette, Preset, Tab, install_interface_fonts};
