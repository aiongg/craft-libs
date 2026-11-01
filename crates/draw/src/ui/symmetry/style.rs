//! [`SymmetryStyle`]: the symmetry guides' look and the transform's reach, from tokens (task TK2).

use egui::Stroke;

use craft_ui::primitives::{Handle, HandleShape, HandleStyle};
use craft_ui::theme::{ToEgui, UiTokens};

/// What the symmetry guides and the Transform Symmetry handles draw. Every size is the same at
/// every Size: they sit on the document.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SymmetryStyle {
    /// Each axis: `symmetry.line` (1) of `overlay.line` (the accent), bare, no halo.
    pub line: Stroke,
    /// A press within this many screen points of the centre or the rotation handle takes it:
    /// `symmetry.hit` (8).
    pub hit: f32,
    /// The rotation handle's distance from the centre as a share of the document's shorter side:
    /// `symmetry.handle_share` (0.35).
    pub handle_share: f64,
    /// The centre anchor and the rotation handle while transforming: the `Handle` primitive's
    /// look (`Handle::style(tokens, HandleShape::Square)` for the centre).
    pub handle: HandleStyle,
}

impl SymmetryStyle {
    /// The guides' look in `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:280 `draw` (1 pt accent), :111 `cursor` and
        // :233 `pointer` (8 / zoom), :227 `rotation_handle` (0.35).
        let s = &tokens.component.symmetry;
        Self {
            line: Stroke::new(s.line, tokens.color.overlay.line.to_egui()),
            hit: s.hit,
            handle_share: f64::from(s.handle_share),
            handle: Handle::style(tokens, HandleShape::Square),
        }
    }
}
