//! [`SymmetryStyle`]: the symmetry guides' look and the transform's reach, from tokens
//! ([`SymmetryOverlay::style`](super::SymmetryOverlay::style)).

use egui::Stroke;

use craft_ui::primitives::HandleStyle;

/// What the symmetry guides and the Transform Symmetry handles draw. Every size is the same at
/// every Size: they sit on the document.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SymmetryStyle {
    /// Each axis: the line (1) of `overlay.line` (the accent), bare, no halo.
    pub line: Stroke,
    /// A press within this many screen points of the centre or the rotation handle takes it:
    /// the hit (8).
    pub hit: f32,
    /// The rotation handle's distance from the centre as a share of the document's shorter side:
    /// the handle share (0.35).
    pub handle_share: f64,
    /// The centre anchor and the rotation handle while transforming: the `Handle` primitive's
    /// look (`Handle::style(tokens, HandleShape::Square)` for the centre).
    pub handle: HandleStyle,
}
