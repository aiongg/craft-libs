//! Liquify's canvas: the canvas tool, the backdrop layering, the mesh (task WS-LQ2).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task WS-LQ2")]

use egui::{Color32, Painter, Pos2};

use craft_ui::canvas::brush::BrushResize;
use craft_ui::canvas::draft::OutlineDraft;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CanvasView, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use egui::{Modifiers, PointerButton};

use super::LiquifyEvent;
use super::session::{BackdropMode, LiquifySettings};
use super::style::LiquifyStyle;
use super::tools::StrokeRecorder;

/// The displaced mesh Show Mesh draws, from [`LiquifySource::mesh`](super::LiquifySource::mesh):
/// `columns × rows` nodes in row order, in document pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiquifyMesh {
    pub columns: usize,
    pub rows: usize,
    pub points: Vec<Point>,
}

impl LiquifyMesh {
    /// The mesh's lines on screen (each horizontal and vertical edge between neighbouring nodes
    /// once), for `warp::paint_mesh`. A mesh whose `points` don't match `columns × rows` has no
    /// lines.
    pub fn edges(&self, view: &CanvasView) -> Vec<[Pos2; 2]> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1248 `draw_mesh` (a regular grid of the
        // field's cell, displaced).
        todo!()
    }
}

/// How the image and the backdrop are drawn for Show Backdrop: the tint of the image and of the
/// backdrop drawn behind or in front of it (white with the alpha of the opacity).
///
/// | Mode | Image | Backdrop |
/// |---|---|---|
/// | none shown | white | — |
/// | In Front | white | in front, at the opacity |
/// | Behind | at the opacity | behind, white |
/// | Blend | at 1 − opacity | in front, at the opacity |
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropLayering {
    pub image: Color32,
    pub behind: Option<Color32>,
    pub front: Option<Color32>,
}

impl BackdropLayering {
    /// The layering for a backdrop that is shown (`shown`) in `mode` at `opacity` (0..=100).
    pub fn of(shown: bool, mode: BackdropMode, opacity: f64) -> BackdropLayering {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:876 `backdrop_layering`.
        todo!()
    }
}

/// The state of Liquify's canvas between passes: the stroke being recorded, the Freeze Lasso
/// being drawn, the brush resize, and the events not yet taken. The workspace hands
/// [`LiquifyCanvasTool`] to its `FullWindowCanvas` while a Liquify tool is current, and the
/// library's `HandTool` or `ZoomTool` for H and Z (kept as private fields here).
#[derive(Debug, Default)]
pub struct LiquifyCanvas {
    recorder: StrokeRecorder,
    lasso: Option<(OutlineDraft, bool)>,
    resize: BrushResize,
    events: Vec<LiquifyEvent>,
}

impl LiquifyCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass over `settings` (the tool, the brush; the resize writes the
    /// brush's size and density).
    pub fn tool<'a>(&'a mut self, settings: &'a mut LiquifySettings, style: &'a LiquifyStyle) -> LiquifyCanvasTool<'a> {
        LiquifyCanvasTool { canvas: self, settings, style }
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<LiquifyEvent> {
        std::mem::take(&mut self.events)
    }

    /// A stroke, a lasso or a resize is in progress: the history waits (`settle(.., busy)`).
    pub fn busy(&self) -> bool {
        self.recorder.is_active() || self.lasso.is_some() || self.resize.active()
    }
}

/// [`LiquifyCanvas`] as the `CanvasTool` of one pass.
///
/// - Press, drag, release with a brush tool: one stroke through [`StrokeRecorder`] (Alt at the
///   press paints with [`LiquifyTool::alternate`](super::LiquifyTool::alternate)), reported as
///   [`LiquifyEvent::Stroke`] Begin, Update with each new point, Commit. Esc or a lost pointer
///   reports Cancel, then a Replay (the workspace adds it from the session).
/// - The Freeze Lasso: an [`OutlineDraft`] of `DraftKind::Freehand` (`DraftReach` from the
///   select tokens through `SelectToolStyle::reach`); the release closes it into a stroke with
///   `amount` 1, or 0 when Alt was held at the press; drawn in `liquify.freeze` (thaw:
///   `liquify.thaw`).
/// - Alt + right-drag, or Control + Alt + drag ([`BrushResize::chord`]; `button_for` gives the
///   secondary button while Alt is held): a [`BrushResize`] with `BrushLevel::Density`,
///   `max_diameter` 15000; horizontal travel sets the size, vertical the density over
///   `brush.resize_travel`; the outline stays at the anchor; no stroke. Each update writes
///   `settings.brush`.
/// - Hover and paint: the brush outline under the pointer (`BrushCursor` over `BrushTip::round`
///   of the size, `TipFalloff::from_hardness` of the density), at the resize's anchor while it
///   runs, none while the pointer is off the canvas; the lasso path while it is drawn.
/// - The cursor: the brush outline's (`BrushCursor::cursor`), the crosshair for the lasso.
pub struct LiquifyCanvasTool<'a> {
    canvas: &'a mut LiquifyCanvas,
    settings: &'a mut LiquifySettings,
    style: &'a LiquifyStyle,
}

impl CanvasTool for LiquifyCanvasTool<'_> {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1162-1216 (the preview's pointer input;
        // :1203-1207 the tools that act at rest keep dabbing at 30 dabs a second while the brush
        // is held still).
        todo!()
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:671 `brush_circle_at`.
        todo!()
    }

    fn button_for(&self, modifiers: Modifiers) -> PointerButton {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:676 `brush_resize_gesture`.
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:693 `pointer` (Down), :682 `resized_brush`.
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:704 (Move).
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:714 (Up), :299 `close_lasso`.
        todo!()
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:705 (`hover` follows the pointer).
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer ends the stroke without recording it (PhotoCraft has no cancel).
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: Esc cancels a stroke or lasso in progress; other keys are the workspace's.
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1142-1149 (the lasso: danger freezes,
        // accent thaws), :1151-1159 (the brush circle, at the anchor while resizing).
        todo!()
    }
}
