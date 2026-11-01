//! [`MarqueeTool`]: the Rectangular, Elliptical, Single Row and Single Column Marquees on the tool
//! host (task SE-F8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task SE-F8")]

use craft_ui::canvas::kurbo::Rect;
use craft_ui::canvas::{
    CanvasTool, Constraint, MarqueeGesture, SelectionOperation, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode, ToolPointer,
};
use egui::Painter;

use super::drag::InsideDrag;
use super::{SelectEvent, SelectPhase, SelectToolPass};

/// Which marquee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarqueeKind {
    /// A rectangle on whole pixels.
    #[default]
    Rectangle,
    /// The ellipse inscribed in a rectangle on whole pixels.
    Ellipse,
    /// The one-pixel row under the pointer, across the whole document.
    SingleRow,
    /// The one-pixel column under the pointer, down the whole document.
    SingleColumn,
}

/// The press of a marquee drag, kept until the release.
#[derive(Clone, Copy, Debug)]
struct MarqueePress {
    /// The kernel's marquee rules: the click slop, the fresh ⇧ and ⌥, Space moving the marquee
    /// (the same rules `MarqueeDrag` runs on a `Ui`).
    gesture: MarqueeGesture,
    /// The operation the press latched.
    operation: SelectionOperation,
    /// Space is held: the drag moves the marquee instead of sizing it (set by `key`).
    space: bool,
}

/// The marquee selection tools (Photoshop's M group). The app owns one value per canvas, sets the
/// options bar's operation and style each pass, and wraps it in a [`SelectToolPass`] for
/// `ToolHost::show`.
///
/// Pointer, in document units through the host:
/// - A drag sizes the marquee from the press: [`Constraint`] (Normal, Fixed Ratio, Fixed Size) from
///   [`set_constraint`](Self::set_constraint), ⇧ square (circle), ⌥ from the centre, each only when
///   pressed after the press (or released and pressed again: ⇧ and ⌥ held at the press are the
///   operation). Space held moves the whole marquee. The rectangle is on whole pixels:
///   `marquee_pixels` of the corners, `square_pixels` when it is square (⇧, or a Fixed Ratio of
///   1 : 1), `fixed_size_pixels` for Fixed Size, exactly as the release commits it; the readout
///   beside the pointer shows W and H ([`SelectLabels`](super::SelectLabels)).
/// - A release within `marquee.click_slop` (3 screen points) of the press, or with no area, is a
///   click: [`SelectEvent::Deselect`]. With Fixed Size a click places the box there.
/// - [`MarqueeKind::SingleRow`] and [`MarqueeKind::SingleColumn`] select the pixel row (column)
///   under the pointer across the whole document ([`SelectionProbe::size`](super::SelectionProbe::size)),
///   kept on the canvas, following the pointer until the release; a click selects too.
/// - A press inside the selection drags it ([`SelectEvent::DragSelection`], module docs); a ⌘
///   press outside it is [`ToolAction::Ignored`].
/// - Esc cancels the drag (the host calls `cancel`): no event.
///
/// The operation is latched at the press ([`SelectionOperation::resolve`]) and the cursor shows
/// its badge before the press, following the live modifiers. The ants of the drag are the
/// rectangle (or the inscribed ellipse) built in document space and mapped point by point through
/// the view, so a rotated view draws them where the release selects.
#[derive(Clone, Debug, Default)]
pub struct MarqueeTool {
    kind: MarqueeKind,
    bar: SelectionOperation,
    constraint: Constraint,
    press: Option<MarqueePress>,
    inside: Option<InsideDrag>,
    events: Vec<SelectEvent>,
}

impl MarqueeTool {
    /// A `kind` marquee with the operation New and the Normal style.
    pub fn new(kind: MarqueeKind) -> Self {
        Self { kind, ..Self::default() }
    }

    /// Which marquee.
    pub fn kind(&self) -> MarqueeKind {
        self.kind
    }

    /// Switch marquee (Shift+M, the tool flyout); a drag in progress is dropped without an event.
    pub fn set_kind(&mut self, kind: MarqueeKind) {
        // new: drop `press` and `inside` when the kind changes.
        todo!()
    }

    /// The options bar's operation (New, Add, Subtract, Intersect) for presses from now on.
    pub fn set_operation(&mut self, bar: SelectionOperation) {
        self.bar = bar;
    }

    /// The options bar's Style for drags from now on (`ConstraintFields` edits it).
    pub fn set_constraint(&mut self, constraint: Constraint) {
        self.constraint = constraint;
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectEvent> {
        std::mem::take(&mut self.events)
    }

    /// Drop a drag in progress without an event (a document or tool switch the host did not see).
    pub fn reset(&mut self) {
        self.press = None;
        self.inside = None;
    }

    /// What the tool is doing: the marquee's pixel rectangle, or a selection drag.
    pub fn phase(&self) -> SelectPhase {
        // new: `Dragging` from `inside`, `Marquee { rect: self.pixels() }` from `press`.
        todo!()
    }

    /// The pixel rectangle the release would select now, `None` while the drag is a click (not
    /// for Fixed Size) or has no area. A one-pixel row or column is a selection. Square is
    /// `MarqueeGesture::square_and_centre`'s ⇧, or a Fixed Ratio of 1 : 1.
    fn pixels(&self, size: [u32; 2]) -> Option<Rect> {
        // pc: crates/ui-egui/src/canvas/drag.rs:82-127 `marquee_mods`, `marquee_corners`,
        // `marquee_preview_px`; :146-153 `marquee_is_square`; :178-202 `single_line_px`
        // (`floor`, clamped to `0..=size − 1`).
        todo!()
    }
}

impl CanvasTool for SelectToolPass<'_, MarqueeTool> {
    /// The crosshair with the operation's badge (`SelectionOperation::badge`, live modifiers);
    /// over the selection the cursors of `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408 (over a selection);
        // crates/ui-egui/src/canvas/tool_feedback.rs:158-169 `draw_badge` (the badge).
        todo!()
    }

    /// A press inside the selection begins a selection drag; a ⌘ press outside is `Ignored`;
    /// otherwise the marquee begins at `pointer.doc` with the operation latched
    /// (`MarqueeGesture::press(pointer.screen, pointer.doc, pointer.modifiers)`).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/tool_input.rs:582-610 `selection_drag_kind`;
        // crates/ui-egui/src/canvas/tool_feedback.rs:88-99 `document_selection_mode`.
        todo!()
    }

    /// Sizes (or with Space moves) the marquee, or moves the selection drag; sets the W and H
    /// readout. The marquee runs `MarqueeGesture::{follow, begin_past, size_to}` with the
    /// `marquee.click_slop` token (`Marquee::style(..).click_slop`), the tool's constraint and its Space flag; no rule is written
    /// here.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:88-127 (corners and the preview rectangle).
        todo!()
    }

    /// Commits: `Commit { shape, operation }` for a rectangle or ellipse with area, `Deselect` for a
    /// click, the drag's `Commit` for a selection drag.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:109-127 (a click deselects; Fixed Size places).
        todo!()
    }

    /// Esc, a lost pointer or a tool switch: the marquee is dropped silently; a selection drag
    /// reports its `Cancel`.
    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: `InsideDrag::cancel` for a drag; nothing for a marquee.
        todo!()
    }

    /// Space down and up during a drag (moves the marquee); taken so the app's Space (the Hand) does
    /// not run.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: sets `MarqueePress::space`, which `drag` passes to `MarqueeGesture::size_to`.
        todo!()
    }

    /// The ants of the marquee being dragged (`Marquee::rect` or `Marquee::ellipse`,
    /// `MarqueeLook::Ants`).
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:213-240 `marquee_ants` (built in document space,
        // mapped point by point, #2512).
        todo!()
    }

    /// Corners snap to the document's snap targets (a point).
    fn snap_gesture(&self) -> Option<SnapGesture> {
        Some(SnapGesture::Point)
    }

    /// The marquee being dragged as one AccessKit object: named `SelectLabels::outline`, valued with
    /// its pixel size (as the W and H readout), over the pixel rectangle; `None` with no marquee.
    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: SE deviation 3 (no `Ui` on the host); `ToolNode::new(..).value(..).bounds(..)`.
        todo!()
    }
}
