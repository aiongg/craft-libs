//! [`LassoTool`]: the freehand Lasso with ⌥-latched straight segments.

use craft_ui::behavior::move_gesture::ConstrainStyle;
use craft_ui::canvas::draft::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft};
use craft_ui::canvas::kurbo::Rect;
use craft_ui::canvas::{
    CanvasTool, Marquee, MarqueeLook, PointerInk, SelectionBadge, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode,
    ToolPointer, selection_cursor,
};
use craft_ui::edit::CancelReason;
use craft_ui::localized::Words;
use egui::{CursorIcon, Key, Modifiers, Painter};

use super::drag::{InsideDrag, PressRule, cursor_over, drag_kind};
use super::{OUTLINE_POINTS, SelectEvent, SelectLabels, SelectPhase, SelectToolPass, SelectionProbe, SelectionShape, labels_in};

/// Turns a draft's answer into the tool's events: `Started` latches the operation from the
/// press's intent, `Closed` is the commit. Shared by the Lasso and the Polygonal Lasso.
pub(super) fn apply(
    event: Option<DraftEvent>,
    bar: SelectionOperation,
    has_selection: bool,
    operation: &mut SelectionOperation,
    events: &mut Vec<SelectEvent>,
) {
    match event {
        Some(DraftEvent::Started { intent }) => *operation = SelectionOperation::resolve(bar, intent, has_selection),
        Some(DraftEvent::Closed { points, .. }) => events.push(SelectEvent::Commit { shape: SelectionShape::Polygon(points), operation: *operation }),
        _ => {}
    }
}

/// What a lasso is doing: a selection drag, an outline with its point count, or nothing.
pub(super) fn phase(draft: &OutlineDraft, inside: Option<&InsideDrag>) -> SelectPhase {
    if let Some(inside) = inside {
        return SelectPhase::Dragging(inside.kind());
    }
    if draft.is_open() { SelectPhase::Outline { points: draft.points().len() } } else { SelectPhase::Idle }
}

/// The crosshair with the operation's badge: the latched one while an outline is open, else the
/// live modifiers'; ⌘ is the Move tool's, so it shows none. Over the selection, the cursors of
/// [`cursor_over`].
pub(super) fn cursor(
    draft: &OutlineDraft,
    operation: SelectionOperation,
    bar: SelectionOperation,
    inside: Option<&InsideDrag>,
    rule: PressRule,
    probe: &dyn SelectionProbe,
    cx: &ToolContext<'_>,
) -> ToolCursor {
    let crosshair = ToolCursor::System(CursorIcon::Crosshair);
    let Some(pointer) = cx.pointer else { return crosshair };
    if !draft.is_open()
        && let Some(cursor) = cursor_over(probe, pointer.raw, cx.modifiers, bar, rule, inside.map(InsideDrag::kind))
    {
        return cursor;
    }
    let badge = if draft.is_open() {
        operation.badge()
    } else if cx.modifiers.command {
        SelectionBadge::New
    } else {
        SelectionOperation::resolve(bar, cx.modifiers, probe.has_selection()).badge()
    };
    selection_cursor(&cx.canvas.ctx, pointer.screen, badge, PointerInk::System);
    crosshair
}

/// The outline as a solid line and, between straight segments, the rubber band.
pub(super) fn paint_outline(draft: &OutlineDraft, painter: &Painter, cx: &ToolContext<'_>) {
    let points = draft.points();
    if points.len() >= 2 {
        Marquee::polyline(points.iter().copied(), "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
    }
    if let Some([a, b]) = draft.rubber_band() {
        Marquee::segments([(a, b)], "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
    }
}

/// The open outline as one AccessKit object; `None` with no outline.
pub(super) fn outline_node(draft: &OutlineDraft, labels: Option<&SelectLabels>, cx: &ToolContext<'_>) -> Option<ToolNode> {
    let points = draft.points();
    let first = *points.first()?;
    let bounds = points.iter().fold(Rect::from_points(first, first), |rect, &p| rect.union_pt(p));
    let (ctx, words) = (&cx.canvas.ctx, Words::of_ctx(&cx.canvas.ctx));
    let value = words.counted(ctx, OUTLINE_POINTS, &words.get(OUTLINE_POINTS.word), points.len() as u64);
    Some(ToolNode::new(labels_in(labels, ctx).outline).value(value).bounds(bounds))
}

/// Starts the selection drag for `kind` at the press.
pub(super) fn begin_inside(
    kind: super::DragKind,
    pointer: &ToolPointer,
    cx: &ToolContext<'_>,
    inside: &mut Option<InsideDrag>,
    events: &mut Vec<SelectEvent>,
) {
    let (drag, event) = InsideDrag::begin(&ConstrainStyle::of(cx.tokens), kind, pointer.doc, pointer.modifiers);
    *inside = Some(drag);
    events.push(event);
}

/// The draft sample for `pointer`.
pub(super) fn sample(pointer: &ToolPointer, cx: &ToolContext<'_>) -> DraftPoint {
    DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom)
}

/// Photoshop's Lasso: a drag draws a freehand outline; the release closes it into a selection.
///
/// The drawing rules are [`OutlineDraft`]'s with [`DraftKind::Mixed`]: ⌥ pressed after the press
/// switches to straight segments (each click adds a vertex, a drag draws freehand again), and
/// releasing ⌥ with the button up closes; ⌥ held at the press subtracts (the operation) until it
/// is released and pressed again, except with nothing selected, where it starts straight
/// segments. Between clicks the outline closes on a click within `outline_draft.close` (8 pt) of
/// the first point (3 or more points), on a click within `outline_draft.same_point` (3 pt) of the last,
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
    labels: Option<SelectLabels>,
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
            labels: None,
            events: Vec::new(),
        }
    }

    /// The options bar's operation for outlines begun from now on.
    pub fn operation(&mut self, bar: SelectionOperation) -> &mut Self {
        self.bar = bar;
        self
    }

    /// The words of the outline's accessible name and value. Without them the tool reads the
    /// installed ones.
    pub fn labels(&mut self, labels: SelectLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The tool as the `CanvasTool` for one pass, asking `probe` about the document.
    pub fn pass<'a>(&'a mut self, probe: &'a dyn SelectionProbe) -> SelectToolPass<'a, Self> {
        SelectToolPass::new(self, probe)
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectEvent> {
        std::mem::take(&mut self.events)
    }

    /// Drop the outline and any selection drag without an event: the app switched tools (an
    /// outline open between clicks has no host gesture for `ToolHost::cancel` to end).
    pub fn reset(&mut self) {
        self.draft.cancel();
        self.inside = None;
    }

    /// What the tool is doing: an outline with its point count, or a selection drag.
    pub fn phase(&self) -> SelectPhase {
        phase(&self.draft, self.inside.as_ref())
    }
}

impl CanvasTool for SelectToolPass<'_, LassoTool> {
    /// Takes the reach from `DraftReach::of(cx.tokens)`, checks the document
    /// (`OutlineDraft::for_document`), and passes a change of `cx.modifiers` to
    /// `OutlineDraft::modifiers_changed` (⌥ released with the button up closes: `Commit`). Keeps
    /// the keys while the draft takes retract.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:95-100 `cancel_stale`, :108-125 `modifiers`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        tool.draft.set_reach(DraftReach::of(cx.tokens));
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

    /// The crosshair with the operation's badge (PhotoCraft draws the selection tools' pointer the
    /// same way); over the selection `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408, tool_feedback.rs:158-169 `draw_badge`.
        let tool = &*self.tool;
        cursor(&tool.draft, tool.operation, tool.bar, tool.inside.as_ref(), PressRule::Drags, self.probe, cx)
    }

    /// With no outline open: a selection drag inside the selection, `Ignored` for ⌘ outside,
    /// else the outline begins (`OutlineDraft::press`; the operation resolves from the
    /// `Started` intent). With one open: the press goes to the draft (a vertex, or a close). A
    /// second click of a run (`pointer.clicks` 2) closes (`OutlineDraft::close`).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:136-207 `pointer` (Down), :284-286 (double-click
        // commits).
        let (tool, probe) = (&mut *self.tool, self.probe);
        let open = tool.draft.is_open();
        if !open {
            if let Some(kind) = drag_kind(probe, pointer.raw, pointer.modifiers, tool.bar, PressRule::Drags) {
                begin_inside(kind, pointer, cx, &mut tool.inside, &mut tool.events);
                return ToolAction::Claimed;
            }
            if pointer.modifiers.command {
                return ToolAction::Ignored;
            }
        }
        let event = if open && pointer.clicks >= 2 { tool.draft.close() } else { tool.draft.press(sample(pointer, cx), probe.has_selection()) };
        apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
        ToolAction::Claimed
    }

    /// Freehand points, or the selection drag.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:208-240 (Move).
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

    /// The release closes a freehand outline (`Commit`) unless straight segments are on; a
    /// freehand outline of fewer than 3 points (a click) is [`SelectEvent::Deselect`], as in
    /// PhotoCraft. Ends a selection drag.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:237-239.
        let (tool, probe) = (&mut *self.tool, self.probe);
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.end(pointer.doc, pointer.modifiers, cx.view.zoom));
            return;
        }
        let event = tool.draft.release(sample(pointer, cx));
        if event == Some(DraftEvent::Cancelled) {
            // pc: crates/ui-egui/src/canvas/gesture.rs:118-131 (fewer than 3 points deselects).
            tool.events.push(SelectEvent::Deselect);
            return;
        }
        apply(event, tool.bar, probe.has_selection(), &mut tool.operation, &mut tool.events);
    }

    /// Between clicks the rubber band follows the pointer (`OutlineDraft::hover`); never adds a
    /// vertex.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:245-246 (hover moves only move the rubber band).
        if self.tool.draft.hover(sample(pointer, cx)).is_some() {
            cx.request_repaint();
        }
    }

    /// Esc during a drag, a lost pointer or `ToolHost::cancel`, whatever the reason: drops the
    /// outline (no event); a selection drag reports `Cancel`.
    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:272-276 (window focus lost drops it).
        let tool = &mut *self.tool;
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.cancel());
        }
        tool.draft.cancel();
    }

    /// Between clicks: Enter closes, Esc cancels, Backspace and Delete (no modifier) retract.
    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/lasso_ui.rs:60-75 `undo_last_vertex`.
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

    /// The outline (`Marquee::polyline`, `MarqueeLook::Drawing`) and its rubber band.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/tool_feedback.rs:178-182 (the solid path's 3 pt light at
        // 0.55 under 1 pt dark, `MarqueeLook::Drawing`).
        paint_outline(&self.tool.draft, painter, cx);
    }

    /// The outline being drawn as one AccessKit object: named `SelectLabels::outline`, valued
    /// `OUTLINE_POINTS` of its points ("5 points"), over the points' bounds; `None` while no
    /// outline is open.
    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: SE deviation 3 (no `Ui` on the host); `ToolNode::new(..).value(..).bounds(..)`.
        outline_node(&self.tool.draft, self.tool.labels.as_ref(), cx)
    }
}
