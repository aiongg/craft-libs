//! The gradient stop bar's parts: [`GradientStops`], the value, with the stop rules in [`stops`];
//! [`GradientRamp`], the paint; [`GradientSwatch`], a well that shows a gradient; and
//! [`GradientEditor`], the stop bar that edits the stops.
//!
//! Colour stops and opacity stops live in two rows ([`StopRows::Split`], PhotoCraft) or in one,
//! the opacity being the colour's alpha ([`StopRows::Combined`], DesignCraft and VectorCraft).
//! The rules are pure functions of the stops, so an on-canvas gradient line and the bar can't
//! disagree. A ramp and a swatch sample through a [`Sampler`]; the default mixes straight lines
//! in encoded sRGB, each segment bent so its midpoint gives the half mix.
//!
//! Colours are the `swatch.*` tokens; the bar's own are `gradient_editor.*`.

mod editor;
mod editor_stories;
#[cfg(test)]
mod editor_tests;
mod ramp;
pub mod stops;
#[cfg(test)]
mod stops_tests;
mod swatch;
#[cfg(test)]
mod swatch_tests;
mod value;

pub(crate) use editor::marker_texts;
pub use editor::{GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientLabels, StopDrop, StopRef};
pub use ramp::{GradientRamp, GradientRampStyle};
pub use stops::{Sampler, StopRules, default_sampler};
pub use swatch::GradientSwatch;
pub use value::{ColorStop, GradientStops, OpacityStop, StopRow, StopRows};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["GradientSwatch", "GradientEditor"];

pub(crate) fn stories() -> Vec<crate::story::Story> {
    let mut all = swatch::stories();
    all.extend(editor_stories::stories());
    all
}
