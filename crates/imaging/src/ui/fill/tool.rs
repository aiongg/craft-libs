//! Content-Aware Fill's tools and the canvas tool (task WS-CA1).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task WS-CA1")]

use egui::{Modifiers, Painter, PointerButton};

use craft_ui::canvas::brush::BrushResize;
use craft_ui::canvas::draft::OutlineDraft;
use craft_ui::canvas::{CanvasTool, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::edit::GestureId;
use craft_ui::patterns::ToolSlot;

use super::FillEvent;
use super::FillLabels;
use super::session::{FillLimits, FillSession, SamplingStroke};

/// The tools of Content-Aware Fill's strip: Sampling Brush (B), Lasso (L), Hand (H), Zoom (Z).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillTool {
    #[default]
    SamplingBrush,
    Lasso,
    Hand,
    Zoom,
}

impl FillTool {
    pub const ALL: [FillTool; 4] = [FillTool::SamplingBrush, FillTool::Lasso, FillTool::Hand, FillTool::Zoom];

    /// B, L, H, Z.
    pub fn shortcut(self) -> char {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:189 `tool_tip`.
        todo!()
    }

    /// One slot per tool, `group_end` after the Lasso; labels from `labels`, letters as shortcuts.
    pub fn slots(labels: &FillLabels) -> Vec<ToolSlot<FillTool>> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:180 `tool_icon`, :189 `tool_tip`.
        todo!()
    }
}

/// The tools' options: the bar's values, outside the history and not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillToolOptions {
    /// The bar's + (true) or −; the sampling option sets it (Custom adds) and Alt reverses it.
    pub brush_add: bool,
    /// The Sampling Brush's diameter, 1..=5000; a tenth of the sampling window's side when the
    /// workspace opens (20 px for a 50 px selection).
    pub brush_size: f64,
    /// The Lasso's operation in the bar (Shift adds, Alt subtracts, both intersect at the press).
    pub lasso: SelectionOperation,
    /// Expand and Contract's amount, 1..=100 px (3 when the workspace opens; PhotoCraft
    /// `caf_ui/lifecycle.rs:30`).
    pub grow_by: f64,
}

impl FillToolOptions {
    /// `[`: × 0.9 rounded, at least 1. `]`: × 1.1 rounded, at least a pixel larger, at most 5000.
    pub fn step_size(&mut self, larger: bool) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:164-169.
        todo!()
    }
}

/// The state of Content-Aware Fill's canvas between passes: the stroke being painted, the lasso
/// being drawn, the brush resize, the events not yet taken.
#[derive(Debug, Default)]
pub struct FillCanvas {
    stroke: Option<(GestureId, SamplingStroke)>,
    lasso: Option<(OutlineDraft, SelectionOperation)>,
    resize: BrushResize,
    events: Vec<FillEvent>,
}

impl FillCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass. `session` and `limits` decide whether a stroke may begin
    /// or take another point.
    pub fn tool<'a>(
        &'a mut self,
        tool: FillTool,
        options: &'a mut FillToolOptions,
        session: &'a FillSession,
        limits: &'a FillLimits,
    ) -> FillCanvasTool<'a> {
        FillCanvasTool { canvas: self, tool, options, session, limits }
    }

    pub fn take_events(&mut self) -> Vec<FillEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn busy(&self) -> bool {
        self.stroke.is_some() || self.lasso.is_some() || self.resize.active()
    }
}

/// [`FillCanvas`] as the `CanvasTool` of one pass.
///
/// - **Sampling Brush**: press, drag, release is one [`SamplingStroke`] (the bar's mode, reversed
///   by Alt at the press; points equal to the last are skipped), reported as
///   [`FillEvent::Stroke`] Begin, Update with each new point (the app paints its sampling plane
///   as the stroke goes), Commit. A stroke the limits refuse ([`FillSession::may_begin_stroke`])
///   does not begin: `FillEvent::Refused` with the message. A stroke that reaches the points
///   limit stops taking points.
/// - **Lasso**: an [`OutlineDraft`] of `DraftKind::Freehand`; the operation is the bar's, or
///   Shift (Add), Alt (Subtract), Shift+Alt (Intersect) at the press; the release reports
///   [`FillEvent::FillArea`] with [`FillAreaEdit::lasso`](super::FillAreaEdit::lasso) when it has
///   3 points or more.
/// - **Brush resize**: Alt + right-drag or Control + Alt + drag, size only, `max_diameter` 5000.
/// - Paint: the lasso path, the brush outline (`BrushCursor`; PhotoCraft's hand-drawn 1.5 pt black
///   and 0.75 pt white circle is replaced).
pub struct FillCanvasTool<'a> {
    canvas: &'a mut FillCanvas,
    tool: FillTool,
    options: &'a mut FillToolOptions,
    session: &'a FillSession,
    limits: &'a FillLimits,
}

impl CanvasTool for FillCanvasTool<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:405-409 (the brush circle).
        todo!()
    }

    fn button_for(&self, modifiers: Modifiers) -> PointerButton {
        // new: the brush resize chord (`BrushResize::chord`).
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:109 (brush Down), :118-127 (lasso Down).
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:110-113, :128-134.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:114-117, :135-141.
        todo!()
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:111 (`hover`).
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the stroke or lasso.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: Esc cancels a stroke or lasso in progress.
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:399-404 (the lasso path, 1 pt
        // `t.accent`), :405-409 (the brush circle).
        todo!()
    }
}
