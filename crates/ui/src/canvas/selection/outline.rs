//! [`SelectionOutline`]: solid or dashed highlights over selected shapes.

use egui::{Color32, Painter};
use kurbo::BezPath;

use crate::canvas::CanvasView;

/// How a [`SelectionOutline`] strokes its shapes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionOutlineLook {
    /// A `selection_frame.line` (1 pt) line: VectorCraft's and DesignCraft's selected paths.
    #[default]
    Solid,
    /// A `selection_frame.outline` (1.5 pt) line in dashes of `selection_frame.dash` with gaps of
    /// `selection_frame.gap`: CadCraft's selection effect (SELECTIONEFFECT 0).
    Dashed,
}

/// Highlights over the selected shapes, drawn in the host's colour (a layer colour), else
/// `overlay.line`. Decoration: no pointer, keyboard or AccessKit node (the selection's own frame
/// or list carries those). Dashes start afresh on each subpath, in screen points, so they don't
/// crawl as the view zooms.
pub struct SelectionOutline<'a> {
    paths: &'a [BezPath],
    look: SelectionOutlineLook,
    colour: Option<Color32>,
}

impl<'a> SelectionOutline<'a> {
    /// A solid outline of `paths` (document units).
    pub fn solid(paths: &'a [BezPath]) -> Self {
        Self { paths, look: SelectionOutlineLook::Solid, colour: None }
    }

    /// A dashed outline of `paths` (document units).
    pub fn dashed(paths: &'a [BezPath]) -> Self {
        Self { paths, look: SelectionOutlineLook::Dashed, colour: None }
    }

    /// The line colour; `overlay.line` by default.
    pub fn colour(mut self, colour: Color32) -> Self {
        self.colour = Some(colour);
        self
    }

    /// Strokes the paths through `view` with `painter` (the host clips it to the canvas). Curves
    /// are flattened to within a quarter of a screen point.
    pub fn paint(self, painter: &Painter, view: &CanvasView) {
        // cadcraft: crates/ui-egui/src/canvas/draw.rs:266-270 `Look::selection` (1.5 pt, dashed when
        // SELECTIONEFFECT is 0); designcraft: crates/ui-egui/src/canvas/frames.rs:168-173 (each path 1 pt
        // in its layer colour). The widths, dash and gap from `SelectionFrame::style`.
        todo!()
    }
}
