//! [`ShapeDragStyle`]: the Shape tools' preview look and click slop, from tokens.

use egui::Stroke;

use super::ShapeDragTool;
use craft_ui::theme::{ToEgui, UiTokens};

/// What [`ShapeDragTool`] draws while a drag runs, and how far a press may move and still be a
/// click. The same at every Size (it sits on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShapeDragStyle {
    /// The outline of the shape being drawn: `shape_drag.preview_line` (1.5) of `overlay.line`.
    pub line: Stroke,
    /// The dark outline under it: `shape_drag.preview_line` plus `shape_drag.preview_halo` (1) each
    /// side, in `overlay.halo` (PhotoCraft's `contrast_path`, #172).
    pub halo: Stroke,
    /// A press and release closer than this many screen points is a click: `shape_drag.click_slop`
    /// (3); the tool divides it by the zoom.
    pub click_slop: f32,
}

impl ShapeDragTool {
    /// The Shape tools' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ShapeDragStyle {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:148-166 `draw_shape_preview` (accent path over a dark halo, :164 `contrast_path`);
        // crates/ui-egui/src/dialogs/shape_dialog.rs:51 (`slop = 3.0 / zoom`).
        let (s, o) = (&tokens.component.shape_drag, &tokens.color.overlay);
        ShapeDragStyle {
            line: Stroke::new(s.preview_line, o.line.to_egui()),
            halo: Stroke::new(s.preview_line + 2.0 * s.preview_halo, o.halo.to_egui()),
            click_slop: s.click_slop,
        }
    }
}
