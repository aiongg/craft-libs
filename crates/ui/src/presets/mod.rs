//! `PresetBrowser`: a library of named presets shown as thumbnails (DS-44), with groups, a grid
//! or list view, a size slider, search, rename and drag to reorder.
//!
//! The host keeps the library and every command; the browser draws, senses and reports
//! [`PresetEvent`]s. This module holds the data it meets the host through: [`PresetRow`] (the
//! host's flat display list), [`PresetState`] (the view settings the host stores),
//! [`PresetEvent`] and [`PresetLabels`]; [`layout`] is the pure geometry of the rows.

pub mod drag;
pub mod event;
pub mod footer;
pub mod labels;
pub(crate) mod layout;
pub mod menu;
pub mod model;
pub mod rename;
pub mod state;
pub mod style;
pub mod toolbar;
pub mod view;

pub use event::{PresetEvent, PresetOutput};
pub use labels::PresetLabels;
pub use model::{CellShape, PresetKind, PresetRow, Preview, ThumbInfo};
pub use state::{DefaultOpen, PresetState, PresetView};
pub use style::PresetStyle;
