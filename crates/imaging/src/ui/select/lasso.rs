//! [`LassoTool`]: the freehand Lasso with ⌥-latched straight segments (task SE-F9).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task SE-F9")]

use craft_ui::canvas::draft::{DraftKind, DraftReach, OutlineDraft};
use craft_ui::canvas::{CanvasTool, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use egui::{Modifiers, Painter};

use super::drag::InsideDrag;
use super::{SelectEvent, SelectPhase, SelectToolPass};

/// Photoshop's Lasso: a drag draws a freehand outline; the release closes it into a selection.
///
/// The drawing rules are [`OutlineDraft`]'s with [`DraftKind::Mixed`]: ⌥ pressed after the press
/// switches to straight segments (each click adds a vertex, a drag draws freehand again), and
/// releasing ⌥ with the button up closes; ⌥ held at the press subtracts (the operation) until it
/// is released and pressed again, except with nothing selected, where it starts straight
/// segments. Between clicks the outline closes on a click within `select.close_radius` (8 pt) of
/// the first point (3 or more points), on a click within `select.same_point` (3 pt) of the last,
/// on Enter or on a double-click. Backspace or Delete (no modifier) retract the last vertex (the
/// last one cancels); the tool keeps those keys from Edit › Clear (`ToolContext::keep_keys`) while
/// [`OutlineDraft::takes_retract`]. Esc cancels. The outline is dropped when the document changes
/// ([`SelectionProbe::document`](super::SelectionProbe::document)) and on [`reset`](Self::reset).
///
/// A press inside the selection drags it (module docs) instead of drawing; a ⌘ press outside it is
/// [`ToolAction::Ignored`]. The closed outline is [`SelectEvent::Commit`] with a
/// `SelectionShape::Polygon` and the operation latched at the first press.
///
/// The outline being drawn is `Marquee::polyline` with `MarqueeLook::Drawing` (solid, not ants),
/// plus the rubber band from the last vertex to the pointer while straight segments are on.
#[derive(Clone, Debug)]
pub struct LassoTool {
    draft: OutlineDraft,
    bar: SelectionOperation,
    /// The operation latched at the outline's first press.
    operation: SelectionOperation,
    /// The modifiers of the last pass, to report changes to the draft.
    modifiers: Modifiers,
    inside: Option<InsideDrag>,
    events: Vec<SelectEvent>,
}

impl Default for LassoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LassoTool {
    /// A Lasso with the operation New and no outline.
    pub fn new() -> Self {
        Self {
            // The reach comes from the tokens in every pass (`each_pass`, `OutlineDraft::set_reach`).
            draft: OutlineDraft::new(DraftKind::Mixed, DraftReach { close: 0.0, same_point: 0.0 }),
            bar: SelectionOperation::New,
            operation: SelectionOperation::New,
            modifiers: Modifiers::NONE,
            inside: None,
            events: Vec::new(),
        }
    }

    /// The options bar's operation for outlines begun from now on.
    pub fn set_operation(&mut self, bar: SelectionOperation) {
        self.bar = bar;
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectEvent> {
        std::mem::take(&mut self.events)
    }

    /// Drop the outline and any selection drag without an event: the app switched tools (an
    /// outline open between clicks has no host gesture for `ToolHost::cancel` to end).
    pub fn reset(&mut self) {
        // new: `OutlineDraft::cancel`, drop `inside`.
        todo!()
    }

    /// What the tool is doing: an outline with its point count, or a selection drag.
    pub fn phase(&self) -> SelectPhase {
        // new: `Dragging` from `inside`, `Outline` while `draft.is_open()`.
        todo!()
    }
}

impl CanvasTool for SelectToolPass<'_, LassoTool> {
    /// Takes the reach from `SelectToolStyle::of(cx.tokens).reach()`, checks the document
    /// (`OutlineDraft::for_document`), and passes a change of `cx.modifiers` to
    /// `OutlineDraft::modifiers_changed` (⌥ released with the button up closes: `Commit`). Keeps
    /// the keys while the draft takes retract.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:95-100 `cancel_stale`, :108-125 `modifiers`.
        todo!()
    }

    /// The lasso icon with the operation's badge; over the selection `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408.
        todo!()
    }

    /// With no outline open: a selection drag inside the selection, `Ignored` for ⌘ outside,
    /// else the outline begins (`OutlineDraft::press`; the operation resolves from the
    /// `Started` intent). With one open: the press goes to the draft (a vertex, or a close). A
    /// second click of a run (`pointer.clicks` 2) closes (`OutlineDraft::close`).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:136-207 `pointer` (Down), :284-286 (double-click
        // commits).
        todo!()
    }

    /// Freehand points, or the selection drag.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:208-240 (Move).
        todo!()
    }

    /// The release closes a freehand outline (`Commit`) unless straight segments are on; ends a
    /// selection drag.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:237-239.
        todo!()
    }

    /// Between clicks the rubber band follows the pointer (`OutlineDraft::hover`); never adds a
    /// vertex.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:245-246 (hover moves only move the rubber band).
        todo!()
    }

    /// Esc during a drag, a lost pointer or `ToolHost::cancel`: drops the outline (no event); a
    /// selection drag reports `Cancel`.
    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:272-276 (window focus lost drops it).
        todo!()
    }

    /// Between clicks: Enter closes, Esc cancels, Backspace and Delete (no modifier) retract.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:60-75 `undo_last_vertex`.
        todo!()
    }

    /// The outline (`Marquee::polyline`, `MarqueeLook::Drawing`) and its rubber band.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/tool_feedback.rs:178-182 (the solid path's 3 pt light at
        // 0.55 under 1 pt dark, `MarqueeLook::Drawing`).
        todo!()
    }
}
