//! [`SymmetryOverlay`]: the symmetry axes drawn on the canvas.

use egui::{Painter, Pos2, Rect, vec2};
use kurbo::Size;

use super::model::Symmetry;
use super::style::SymmetryStyle;
use craft_ui::canvas::{CanvasView, PartFocus};
use craft_ui::primitives::{Handle, HandleShape};

/// The guides of a painting symmetry: each axis clipped to the document, a bare line in
/// `overlay.line`, drawn while a painting tool is picked or the symmetry is being transformed;
/// while [editing](Self::editing) also the centre anchor and a handle line to the rotation
/// handle. Drawing only: the transform's keyboard parts are [`SymmetryTransform`](super::SymmetryTransform)'s.
pub struct SymmetryOverlay<'a> {
    symmetry: &'a Symmetry,
    size: Size,
    editing: bool,
    ring: Option<u64>,
}

impl<'a> SymmetryOverlay<'a> {
    /// The guides of `symmetry` over a document of `size`.
    pub fn new(symmetry: &'a Symmetry, size: Size) -> Self {
        Self { symmetry, size, editing: false, ring: None }
    }

    /// Transform Symmetry is open: the centre anchor and the rotation handle show.
    pub fn editing(mut self, editing: bool) -> Self {
        self.editing = editing;
        self
    }

    /// The picked part (the transform's [`Part::key`](craft_ui::canvas::Part::key)) to ring while
    /// the keyboard has it.
    pub fn ring(mut self, part: Option<u64>) -> Self {
        self.ring = part;
        self
    }

    /// Paints the guides over `view`, clipped to the view's rect.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &SymmetryStyle) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:280 `draw` (the anchor and handle line through
        // `Handle`, the library's anchor and handle painter).
        let painter = painter.with_clip_rect(painter.clip_rect().intersect(view.rect));
        for line in self.symmetry.segments(self.size) {
            painter.line_segment([view.to_screen(line.p0), view.to_screen(line.p1)], style.line);
        }
        if !self.editing {
            return;
        }
        let center = view.to_screen(self.symmetry.center);
        let handle = view.to_screen(self.symmetry.rotation_handle(self.size, style.handle_share));
        painter.line_segment([center, handle], style.line);
        let ppp = painter.ctx().pixels_per_point();
        painter.extend(Handle::shapes_with(&style.handle, HandleShape::Square, center, ppp));
        painter.extend(Handle::shapes_with(&style.handle, HandleShape::Round, handle, ppp));
        for (key, at) in [(0, center), (1, handle)] {
            if self.ring == Some(key) {
                PartFocus::ring(&painter, part_rect(at, style));
            }
        }
    }
}

/// The screen rect of a part centred on `at`: the square of the pointer's reach.
pub(super) fn part_rect(at: Pos2, style: &SymmetryStyle) -> Rect {
    Rect::from_center_size(at, vec2(2.0 * style.hit, 2.0 * style.hit))
}
