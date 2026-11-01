//! [`PolygonLassoTool`]: the Polygonal Lasso, click by click (task SE-F9).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task SE-F9")]

use craft_ui::canvas::draft::{DraftKind, DraftReach, OutlineDraft};
use craft_ui::canvas::{CanvasTool, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use egui::{Modifiers, Painter};

use super::drag::InsideDrag;
use super::{SelectEvent, SelectPhase, SelectToolPass};

/// Photoshop's Polygonal Lasso: each click adds a vertex; the outline closes into a selection.
///
/// The drawing rules are [`OutlineDraft`]'s with [`DraftKind::Polygonal`]: ⇧ (without ⌥) turns
/// the new segment to a multiple of 45° from the last vertex (not the first vertex), ⌥-drag draws
/// freehand. The first click latches the operation (⌥ with nothing selected is New). The outline
/// closes on a click within `select.close_radius` (8 pt) of the first vertex (3 or more), on a
/// click within `select.same_point` (3 pt) of the last, on Enter or a double-click. Backspace,
/// Delete or a secondary click ([`retract`](Self::retract)) retract the last vertex (the last one cancels); Esc cancels; a
/// document change or [`reset`](Self::reset) drops it.
///
/// Click-driven: a plain press inside the selection places a vertex. Only ⌘ drags the selection
/// (pixels; module docs), and not while an outline is open. A ⌘ press outside the selection is
/// [`ToolAction::Ignored`].
///
/// While a New outline is open the selection it will replace is hidden:
/// [`replaces_selection`](Self::replaces_selection) is for `SelectionOverlay::visible`.
#[derive(Clone, Debug)]
pub struct PolygonLassoTool {
    draft: OutlineDraft,
    bar: SelectionOperation,
    operation: SelectionOperation,
    modifiers: Modifiers,
    inside: Option<InsideDrag>,
    events: Vec<SelectEvent>,
}

impl Default for PolygonLassoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PolygonLassoTool {
    /// A Polygonal Lasso with the operation New and no outline.
    pub fn new() -> Self {
        Self {
            // The reach comes from the tokens in every pass (`each_pass`, `OutlineDraft::set_reach`).
            draft: OutlineDraft::new(DraftKind::Polygonal, DraftReach { close: 0.0, same_point: 0.0 }),
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

    /// Drop the outline and any selection drag without an event (a tool switch).
    pub fn reset(&mut self) {
        // new: `OutlineDraft::cancel`, drop `inside`.
        todo!()
    }

    /// What the tool is doing.
    pub fn phase(&self) -> SelectPhase {
        // new: as `LassoTool::phase`.
        todo!()
    }

    /// Retract the last vertex (the last one cancels the outline); `false` when no outline is open
    /// or a button is down. The app calls it for a secondary click on the canvas while an outline
    /// is open (the host passes the secondary button to no selection tool) and shows no context
    /// menu then.
    pub fn retract(&mut self) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:197-207 `polygon_retract` (right-click, #1229).
        todo!()
    }

    /// An outline with the operation New is open: the app hides the selection's ants.
    pub fn replaces_selection(&self) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:227-229 `polygon_replaces_selection`.
        todo!()
    }
}

impl CanvasTool for SelectToolPass<'_, PolygonLassoTool> {
    /// The reach from the tokens, the document check, modifier changes to the draft, and the kept
    /// keys while an outline is open.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/gesture.rs:214-223 `polygon_cancel_stale`.
        todo!()
    }

    /// The polygon lasso icon with the operation's badge; with ⌘ over the selection
    /// `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408.
        todo!()
    }

    /// ⌘ inside the selection with no outline open: a pixel drag. Otherwise the click goes to the
    /// draft (the first latches the operation); a double-click closes.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/gesture.rs:174-195 `polygon_click`;
        // crates/ui-egui/src/canvas/tool_input.rs:588-592 (click-driven tools need ⌘).
        todo!()
    }

    /// An ⌥-drag adds freehand points; a selection drag moves.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: `OutlineDraft::drag`; `InsideDrag::drag`.
        todo!()
    }

    /// Ends an ⌥-drag segment or a selection drag; the outline stays open.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: `OutlineDraft::release`; `InsideDrag::end`.
        todo!()
    }

    /// The rubber band follows the pointer, constrained to 45° with ⇧.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/gesture.rs:155-169 `constrained_polygon_point`.
        todo!()
    }

    /// Drops an ⌥-drag segment's gesture (the outline stays for Esc between clicks); a selection
    /// drag reports `Cancel`.
    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: `InsideDrag::cancel`.
        todo!()
    }

    /// Enter closes, Esc cancels, Backspace and Delete retract.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:197-207 `polygon_retract`.
        todo!()
    }

    /// The outline (`MarqueeLook::Drawing`) and its rubber band.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/tool_feedback.rs:178-182.
        todo!()
    }
}
