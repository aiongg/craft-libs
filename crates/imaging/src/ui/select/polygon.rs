//! [`PolygonLassoTool`]: the Polygonal Lasso, click by click (task SE-F9).

use craft_ui::canvas::draft::{DraftKind, DraftPoint, DraftReach, OutlineDraft};
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode, ToolPointer};
use egui::{Key, Modifiers, Painter};

use super::drag::{InsideDrag, PressRule, drag_kind};
use super::lasso::{apply, begin_inside, cursor, outline_node, paint_outline, phase, sample};
use super::style::SelectToolStyle;
use super::{SelectEvent, SelectPhase, SelectToolPass};

/// `point` turned about `from` to the nearest multiple of 45°, keeping its distance: the draft's
/// own turn for a vertex, applied to the rubber band too.
fn turn_to_45(from: Point, point: Point) -> Point {
    // pc: crates/ui-egui/src/canvas/gesture.rs:157-169 `constrained_polygon_point`.
    let delta = point - from;
    let length = delta.hypot();
    if !length.is_finite() || length == 0.0 {
        return point;
    }
    let angle = (delta.y.atan2(delta.x) / std::f64::consts::FRAC_PI_4).round() * std::f64::consts::FRAC_PI_4;
    Point::new(from.x + length * angle.cos(), from.y + length * angle.sin())
}

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
        self.draft.cancel();
        self.inside = None;
    }

    /// What the tool is doing.
    pub fn phase(&self) -> SelectPhase {
        phase(&self.draft, self.inside.as_ref())
    }

    /// Retract the last vertex (the last one cancels the outline); `false` when no outline is open
    /// or a button is down. The app calls it for a secondary click on the canvas while an outline
    /// is open (the host passes the secondary button to no selection tool) and shows no context
    /// menu then.
    pub fn retract(&mut self) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:197-207 `polygon_retract` (right-click, #1229).
        self.draft.retract().is_some()
    }

    /// An outline with the operation New is open: the app hides the selection's ants.
    pub fn replaces_selection(&self) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:227-229 `polygon_replaces_selection`.
        self.draft.is_open() && self.operation == SelectionOperation::New
    }
}

impl CanvasTool for SelectToolPass<'_, PolygonLassoTool> {
    /// The reach from the tokens, the document check, modifier changes to the draft, and the kept
    /// keys while an outline is open.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/gesture.rs:214-223 `polygon_cancel_stale`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        tool.draft.set_reach(SelectToolStyle::of(cx.tokens).reach());
        if tool.draft.for_document(probe.document()).is_some() {
            tool.inside = None;
        }
        if cx.modifiers != tool.modifiers {
            tool.modifiers = cx.modifiers;
            let event = tool.draft.modifiers_changed(cx.modifiers);
            apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
        }
        if tool.draft.takes_retract() {
            cx.keep_keys();
        }
    }

    /// The crosshair with the operation's badge; with ⌘ over the selection
    /// `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408.
        let tool = &*self.tool;
        let rule = PressRule::Clicks { outline_open: tool.draft.is_open() };
        cursor(&tool.draft, tool.operation, tool.bar, tool.inside.as_ref(), rule, self.probe, cx)
    }

    /// ⌘ inside the selection with no outline open: a pixel drag. Otherwise the click goes to the
    /// draft (the first latches the operation); a double-click closes.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/gesture.rs:174-195 `polygon_click`;
        // crates/ui-egui/src/canvas/tool_input.rs:588-592 (click-driven tools need ⌘).
        let (tool, probe) = (&mut *self.tool, self.probe);
        let open = tool.draft.is_open();
        if let Some(kind) = drag_kind(probe, pointer.raw, pointer.modifiers, tool.bar, PressRule::Clicks { outline_open: open }) {
            begin_inside(kind, pointer, cx, &mut tool.inside, &mut tool.events);
            return ToolAction::Claimed;
        }
        if pointer.modifiers.command && !open {
            return ToolAction::Ignored;
        }
        let event = if open && pointer.clicks >= 2 { tool.draft.close() } else { tool.draft.press(sample(pointer, cx), probe.has_selection()) };
        apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
        ToolAction::Claimed
    }

    /// An ⌥-drag adds freehand points; a selection drag moves.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: `OutlineDraft::drag`; `InsideDrag::drag`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        if let Some(inside) = tool.inside.as_mut() {
            if let Some(event) = inside.drag(pointer.doc, pointer.modifiers, cx.view.zoom) {
                tool.events.push(event);
            }
            return;
        }
        let event = tool.draft.drag(sample(pointer, cx));
        apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
    }

    /// Ends an ⌥-drag segment or a selection drag; the outline stays open.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: `OutlineDraft::release`; `InsideDrag::end`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.end(pointer.doc, pointer.modifiers, cx.view.zoom));
            return;
        }
        let event = tool.draft.release(sample(pointer, cx));
        apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
    }

    /// The rubber band follows the pointer, constrained to 45° with ⇧.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/gesture.rs:155-169 `constrained_polygon_point`.
        let tool = &mut *self.tool;
        let mut at = sample(pointer, cx);
        if pointer.modifiers.shift
            && !pointer.modifiers.alt
            && let Some(&last) = tool.draft.points().last()
        {
            at = DraftPoint { at: turn_to_45(last, at.at), ..at };
        }
        if tool.draft.hover(at).is_some() {
            cx.request_repaint();
        }
    }

    /// Drops an ⌥-drag segment's gesture (the outline stays for Esc between clicks); a selection
    /// drag reports `Cancel`.
    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: `InsideDrag::cancel`. An ⌥-drag segment ends where it is: the release keeps the outline.
        let tool = &mut *self.tool;
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.cancel());
        }
        if tool.draft.is_down()
            && let Some(&last) = tool.draft.points().last()
        {
            tool.draft.release(DraftPoint::new(last, Modifiers::NONE, 1.0));
        }
    }

    /// Enter closes, Esc cancels, Backspace and Delete retract.
    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/canvas/gesture.rs:197-207 `polygon_retract`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        if !key.pressed || !tool.draft.takes_retract() {
            return false;
        }
        match key.key {
            Key::Enter if key.modifiers.is_none() => {
                let event = tool.draft.close();
                apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
            }
            Key::Escape => {
                tool.draft.cancel();
            }
            Key::Backspace | Key::Delete if key.modifiers.is_none() => {
                tool.draft.retract();
            }
            _ => return false,
        }
        true
    }

    /// The outline (`MarqueeLook::Drawing`) and its rubber band.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/tool_feedback.rs:178-182.
        paint_outline(&self.tool.draft, painter, cx);
    }

    /// The outline being drawn as one AccessKit object: named `SelectLabels::outline`, valued
    /// `OUTLINE_POINTS` of its points ("5 points"), over the points' bounds; `None` while no
    /// outline is open.
    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: SE deviation 3 (no `Ui` on the host); `ToolNode::new(..).value(..).bounds(..)`.
        outline_node(&self.tool.draft, cx)
    }
}
