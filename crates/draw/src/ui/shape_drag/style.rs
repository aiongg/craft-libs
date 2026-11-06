//! [`ShapeDragStyle`]: the Shape tools' preview look and click slop, from tokens.

use egui::Stroke;

use super::ShapeDragTool;
use craft_ui::theme::{ToEgui, UiTokens};

/// A press and release closer than this many screen points is a click.
const CLICK_SLOP: f32 = 3.0;

/// What [`ShapeDragTool`] draws while a drag runs, and how far a press may move and still be a
/// click. The same at every Size (it sits on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShapeDragStyle {
    /// The outline of the shape being drawn, in `overlay.line`.
    pub line: Stroke,
    /// The dark outline under it: the line's width plus `stroke.thin` on each side, in `overlay.halo`.
    pub halo: Stroke,
    /// A press and release closer than this many screen points is a click; the tool divides it
    /// by the zoom.
    pub click_slop: f32,
}

impl ShapeDragTool {
    /// The Shape tools' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ShapeDragStyle {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:148-166 `draw_shape_preview` (accent path over a dark halo, :164 `contrast_path`);
        // crates/ui-egui/src/dialogs/shape_dialog.rs:51 (`slop = 3.0 / zoom`).
        let (stroke, o) = (&tokens.stroke, &tokens.color.overlay);
        ShapeDragStyle {
            line: Stroke::new(stroke.medium, o.line.to_egui()),
            halo: Stroke::new(stroke.medium + 2.0 * stroke.thin, o.halo.to_egui()),
            click_slop: CLICK_SLOP,
        }
    }
}
