//! Styles of components that several modules draw, computed from the semantic tokens.
//!
//! A style holds only what the semantic set doesn't name directly: expressions of semantic
//! tokens and the component's own constants. Readers take semantic tokens straight from
//! [`craft_theme::Tokens`].

pub mod bezier;
pub mod brush;
pub mod button;
pub mod dialog;
pub mod dock;
pub mod field;
pub mod float;
pub mod grid;
pub mod guide;
pub mod kbd;
pub mod labels;
pub mod levels;
pub mod line_guide;
pub mod list;
pub mod notice;
pub mod progress;
pub mod rail;
pub mod ruler;
pub mod scroll;
pub mod segmented;
pub mod slider;
pub mod swatch;
pub mod tile;
pub mod tooltip;
pub mod transform;
