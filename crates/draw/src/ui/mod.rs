//! The draw tools' widgets, for egui.
//!
//! - [`gradient`]: the gradient stop bar ([`GradientEditor`]), its palette and swatch, the stop
//!   rules ([`gradient::stops`]) and the on-canvas [`GradientOverlay`] with the geometry of its
//!   line ([`gradient::gradient_line`]).

pub mod gradient;

#[cfg(test)]
mod words_tests;

pub use gradient::{
    ColorStop, GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientExtent, GradientGroup, GradientItem, GradientLabels,
    GradientLine, GradientLineNames, GradientLineStyle, GradientOverlay, GradientOverlayResponse, GradientPalette, GradientRamp, GradientRampStyle,
    GradientRef, GradientStops, GradientSwatch, LineCursor, LineGradient, LinePart, OpacityStop, PaletteLayout, Sampler as StopSampler, StopDrop,
    StopRef, StopRow, StopRows, StopRules, default_sampler,
};

craft_ui::story::register! {
    gradient,
    gradient::gradient_overlay::story,
}

/// The stories of the draw tools.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The labels tables of the draw tools, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] =
    &[<GradientLineNames as craft_ui::localized::Labels>::WORDS, <GradientLabels as craft_ui::localized::Labels>::WORDS];
