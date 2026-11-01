//! [`SymmetryOverlay`]: the symmetry axes drawn on the canvas.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-S2)")]

use egui::Painter;
use kurbo::Size;

use super::model::Symmetry;
use super::style::SymmetryStyle;
use craft_ui::canvas::CanvasView;

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
        todo!()
    }
}
