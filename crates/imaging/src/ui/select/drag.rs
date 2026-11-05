//! A press inside the selection: which drag it starts, the cursor over the selection, and the
//! drag itself (the lasso tools use it unchanged). Public for the app's own
//! click tools: PhotoCraft's Magic Wand takes the same rule as the Polygonal Lasso
//! ([`PressRule::Clicks`]; PhotoCraft `tool_input.rs:587-610`, `clicky`).

use craft_ui::behavior::move_gesture::{ConstrainStyle, Directions, MoveGesture};
use craft_ui::canvas::kurbo::{Point, Vec2};
use craft_ui::canvas::{CursorBadge, SelectionOperation, ToolCursor};
use craft_ui::edit::EditEvent;
use egui::{CursorIcon, Modifiers};

use super::{DragKind, SelectEvent, SelectionDrag, SelectionProbe};

/// How a tool takes a press inside the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressRule {
    /// A drag tool (the marquees, the Lasso): a plain press with the operation New drags the
    /// outline, ⌘ the pixels.
    Drags,
    /// A click-driven tool (the Polygonal Lasso, the app's Magic Wand): only ⌘ drags, and never
    /// while `outline_open`.
    Clicks {
        /// The tool has an outline open (the Polygonal Lasso between clicks).
        outline_open: bool,
    },
}

/// What a press at `at` with `modifiers` drags, or `None` when it draws (or is not the tool's).
///
/// With a floating piece: a press on it (`probe.on_floating`) drags it as `Pixels` unless ⇧ is
/// held, or ⌥ without ⌘; a click-driven tool needs ⌘; elsewhere the press draws a new selection.
/// Without one, a press outside the selection draws; inside, ⌘ without ⇧ is `Pixels { copy: ⌥ }`
/// (⌘⌥ copies), and a drag tool with no ⌘ whose resolved operation
/// (`SelectionOperation::resolve(bar, modifiers, has_selection)`) is New drags the `Outline`.
/// A click-driven tool returns `None` for everything while its outline is open.
pub fn drag_kind(probe: &dyn SelectionProbe, at: Point, modifiers: Modifiers, bar: SelectionOperation, rule: PressRule) -> Option<DragKind> {
    // pc: crates/ui-egui/src/canvas/tool_input.rs:587-610 `selection_drag_kind` (`cut = command &&
    // !shift`; the floating rule at :596-602, #7538; `copy = alt && command`,
    // crates/ui-egui/src/canvas/selection.rs:45).
    let clicky = match rule {
        PressRule::Drags => false,
        PressRule::Clicks { outline_open } if outline_open => return None,
        PressRule::Clicks { .. } => true,
    };
    let cut = modifiers.command && !modifiers.shift;
    let copy = modifiers.alt && modifiers.command;
    if probe.floating() {
        let on = probe.on_floating(at) && !modifiers.shift && (!modifiers.alt || modifiers.command) && (cut || !clicky);
        return on.then_some(DragKind::Pixels { copy });
    }
    if !probe.inside_selection(at) {
        return None;
    }
    if cut {
        return Some(DragKind::Pixels { copy });
    }
    let plain = !clicky && !modifiers.command && SelectionOperation::resolve(bar, modifiers, probe.has_selection()) == SelectionOperation::New;
    plain.then_some(DragKind::Outline)
}

/// The cursor over the selection for a press with `modifiers`, or `None` for the tool's own:
/// `Outline` shows the move cursor (`ToolCursor::System(CursorIcon::Move)`, #1428), `Pixels`
/// the arrow with the scissors badge (`ToolCursor::Arrow { badge: Some(CursorBadge::Scissors) }`)
/// or with ⌥ the copy badge, a floating piece the plain arrow. While `dragging`, `Outline` keeps
/// the move cursor and `Pixels` shows the plain arrow.
pub fn cursor_over(
    probe: &dyn SelectionProbe,
    at: Point,
    modifiers: Modifiers,
    bar: SelectionOperation,
    rule: PressRule,
    dragging: Option<DragKind>,
) -> Option<ToolCursor> {
    // pc: crates/ui-egui/src/canvas/selection.rs:24-51 `selection_cursor`;
    // crates/ui-egui/src/canvas.rs:1384-1408 (the icons: Move, arrow, scissors badge, copy badge).
    match dragging {
        Some(DragKind::Outline) => return Some(ToolCursor::System(CursorIcon::Move)),
        Some(DragKind::Pixels { .. }) => return Some(ToolCursor::Arrow { badge: None }),
        None => {}
    }
    Some(match drag_kind(probe, at, modifiers, bar, rule)? {
        DragKind::Outline => ToolCursor::System(CursorIcon::Move),
        DragKind::Pixels { copy: true } => ToolCursor::Arrow { badge: Some(CursorBadge::Copy) },
        DragKind::Pixels { copy: false } if probe.floating() => ToolCursor::Arrow { badge: None },
        DragKind::Pixels { copy: false } => ToolCursor::Arrow { badge: Some(CursorBadge::Scissors) },
    })
}

/// A selection drag in progress: a [`MoveGesture`] with eight ⇧ directions (selected pixels and
/// outlines lock to the axes and the diagonals), reported as whole-pixel
/// [`SelectionDrag`](super::SelectionDrag) values. A tool keeps one from the press
/// [`drag_kind`] answered until the release or Esc.
#[derive(Clone, Debug)]
pub struct InsideDrag {
    kind: DragKind,
    gesture: MoveGesture,
    /// The whole-pixel delta of the last event.
    last: [i32; 2],
}

impl InsideDrag {
    /// Starts a `kind` drag at `at`: the gesture and the `DragSelection(Begin)` event.
    pub fn begin(style: &ConstrainStyle, kind: DragKind, at: Point, modifiers: Modifiers) -> (InsideDrag, SelectEvent) {
        // new: `MoveGesture::begin(style, at, modifiers, Directions::Eight)`; the Begin event's
        // value is `None`.
        let (gesture, event) = MoveGesture::begin(style, at, modifiers, Directions::Eight);
        let event = event.map(|v| SelectionDrag { kind, delta: whole_pixels(v.delta) });
        (InsideDrag { kind, gesture, last: [0, 0] }, SelectEvent::DragSelection(event))
    }

    /// The kind of drag.
    pub fn kind(&self) -> DragKind {
        self.kind
    }

    /// The pointer moved: `DragSelection(Update)` when the whole-pixel delta changed. The delta is
    /// `MoveGesture`'s offset rounded per axis and clamped to ±10⁷; `MoveGesture`'s own ⌥ copy
    /// is ignored (the copy is latched in `kind`).
    pub fn drag(&mut self, at: Point, modifiers: Modifiers, zoom: f64) -> Option<SelectEvent> {
        // pc: crates/ui-egui/src/canvas/selection.rs:54-58 `selection_drag_delta` (round, clamp
        // ±1e7).
        let kind = self.kind;
        let event = self.gesture.drag(at, modifiers, zoom)?.map(|v| SelectionDrag { kind, delta: whole_pixels(v.delta) });
        let delta = event.value()?.delta;
        if delta == self.last {
            return None;
        }
        self.last = delta;
        Some(SelectEvent::DragSelection(event))
    }

    /// The release: `DragSelection(Commit)`, `changed` false when the delta is 0.
    pub fn end(self, at: Point, modifiers: Modifiers, zoom: f64) -> SelectEvent {
        // pc: crates/ui-egui/src/canvas/selection.rs:90-105 `finish_selection_drag` (the app's
        // deselect, drop, float and transformSelection follow from kind and delta).
        let kind = self.kind;
        let event = self.gesture.end(at, modifiers, zoom).map(|v| SelectionDrag { kind, delta: whole_pixels(v.delta) });
        // The gesture's `changed` counts a copy; here only the whole-pixel delta does.
        SelectEvent::DragSelection(match event {
            EditEvent::Commit { id, value, modifiers, .. } => EditEvent::commit(id, value, value.delta != [0, 0], modifiers),
            other => other,
        })
    }

    /// Esc or a lost pointer: `DragSelection(Cancel)`.
    pub fn cancel(self) -> SelectEvent {
        // new: `MoveGesture::cancel`'s id and reason.
        let kind = self.kind;
        SelectEvent::DragSelection(self.gesture.cancel().map(|v| SelectionDrag { kind, delta: whole_pixels(v.delta) }))
    }
}

/// The offset rounded per axis and clamped to ±10⁷ (PhotoCraft's `selection_drag_delta`).
fn whole_pixels(delta: Vec2) -> [i32; 2] {
    let axis = |v: f64| v.round().clamp(-1e7, 1e7) as i32;
    [axis(delta.x), axis(delta.y)]
}
