//! [`MarqueeTool`]: the Rectangular, Elliptical, Single Row and Single Column Marquees on the tool
//! host.

use craft_ui::behavior::move_gesture::ConstrainStyle;
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{
    CanvasTool, Constraint, Marquee, MarqueeGesture, MarqueeLook, MarqueeRect, PointerInk, Readout, SelectionBadge, SelectionOperation, SnapGesture,
    ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode, ToolPointer, fixed_size_pixels, marquee_pixels, selection_cursor, square_pixels,
};
use craft_ui::edit::CancelReason;
use craft_ui::localized::Units;
use egui::{CursorIcon, Key, Painter};

use super::drag::{InsideDrag, PressRule, cursor_over, drag_kind};
use super::{SelectEvent, SelectLabels, SelectPhase, SelectToolPass, SelectionProbe, SelectionShape, labels_in};

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
    /// The document's size at the press, for the Single Row and Single Column Marquees.
    size: [u32; 2],
    /// The pointer's document point as last followed.
    last: Point,
    /// The corners the gesture last gave, once it has begun.
    corners: Option<MarqueeRect>,
}

/// The marquee selection tools (Photoshop's M group). The app owns one value per canvas, sets the
/// options bar's operation and style each pass, and wraps it in a [`SelectToolPass`] for
/// `ToolHost::show`.
///
/// Pointer, in document units through the host:
/// - A drag sizes the marquee from the press: [`Constraint`] (Normal, Fixed Ratio, Fixed Size) from
///   [`constraint`](Self::constraint), ⇧ square (circle), ⌥ from the centre, each only when
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
///   press outside it is [`ToolAction::Ignored`]. The Single Row and Single Column Marquees never
///   drag the selection: a press inside it draws the line (PhotoCraft).
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
    labels: Option<SelectLabels>,
    events: Vec<SelectEvent>,
}

impl MarqueeTool {
    /// A `kind` marquee with the operation New and the Normal style.
    pub fn new(kind: MarqueeKind) -> Self {
        Self { kind, ..Self::default() }
    }

    /// Which marquee is set.
    pub fn marquee_kind(&self) -> MarqueeKind {
        self.kind
    }

    /// Switch marquee (Shift+M, the tool flyout); a drag in progress is dropped without an event.
    pub fn kind(&mut self, kind: MarqueeKind) -> &mut Self {
        // new: drop `press` and `inside` when the kind changes.
        if kind != self.kind {
            self.reset();
        }
        self.kind = kind;
        self
    }

    /// The options bar's operation (New, Add, Subtract, Intersect) for presses from now on.
    pub fn operation(&mut self, bar: SelectionOperation) -> &mut Self {
        self.bar = bar;
        self
    }

    /// The options bar's Style for drags from now on (`ConstraintFields` edits it).
    pub fn constraint(&mut self, constraint: Constraint) -> &mut Self {
        self.constraint = constraint;
        self
    }

    /// The words of the W and H readout and the marquee's accessible name and value. Without them
    /// the tool reads the installed ones.
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

    /// Drop a drag in progress without an event (a document or tool switch the host did not see).
    pub fn reset(&mut self) {
        self.press = None;
        self.inside = None;
    }

    /// What the tool is doing: the marquee's pixel rectangle, or a selection drag.
    pub fn phase(&self) -> SelectPhase {
        // new: `Dragging` from `inside`, `Marquee { rect: self.pixels() }` from `press`.
        if let Some(inside) = &self.inside {
            return SelectPhase::Dragging(inside.kind());
        }
        match &self.press {
            Some(press) => SelectPhase::Marquee { rect: self.pixels(press.size) },
            None => SelectPhase::Idle,
        }
    }

    /// The pixel rectangle the release would select now, `None` while the drag is a click (not
    /// for Fixed Size) or has no area. A one-pixel row or column is a selection. Square is
    /// `MarqueeGesture::square_and_centre`'s ⇧, or a Fixed Ratio of 1 : 1.
    fn pixels(&self, size: [u32; 2]) -> Option<Rect> {
        // pc: crates/ui-egui/src/canvas/drag.rs:82-127 `marquee_mods`, `marquee_corners`,
        // `marquee_preview_px`; :146-153 `marquee_is_square`; :178-202 `single_line_px`
        // (`floor`, clamped to `0..=size − 1`).
        let press = self.press.as_ref()?;
        if matches!(self.kind, MarqueeKind::SingleRow | MarqueeKind::SingleColumn) {
            return single_line(self.kind, size, press.last);
        }
        let fixed = matches!(self.constraint, Constraint::Size(..));
        if !fixed && !press.gesture.has_begun() {
            return None;
        }
        let corners = press.corners?;
        let (a, b) = (corners.a, corners.b);
        let mut rect = match self.constraint {
            Constraint::Size(w, h) => fixed_size_pixels(a, b, (w, h)),
            _ => marquee_pixels(a, b),
        };
        let square = match self.constraint {
            Constraint::Size(..) => false,
            Constraint::Ratio(w, h) if w > 0.0 && h > 0.0 => w == h,
            _ => press.gesture.square_and_centre().0,
        };
        if square {
            // The corners run from the press point towards the pointer, whichever way ⌥ centres them.
            rect = square_pixels(rect, a, b);
        }
        (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
    }

    /// Follows the pointer: the gesture records the keys and the slop, and once begun sizes the
    /// marquee to the pointer.
    fn advance(&mut self, pointer: &ToolPointer, cx: &ToolContext<'_>) {
        let slop = Marquee::style(cx.tokens).click_slop;
        let (constraint, kind) = (self.constraint, self.kind);
        let Some(press) = self.press.as_mut() else { return };
        press.gesture.follow(Some(pointer.screen), pointer.modifiers);
        press.last = pointer.doc;
        press.gesture.begin_past(slop, constraint);
        if press.gesture.has_begun() && matches!(kind, MarqueeKind::Rectangle | MarqueeKind::Ellipse) {
            press.corners = Some(press.gesture.size_to(pointer.doc, press.space, constraint));
        }
    }

    /// The W and H readout beside the pointer.
    fn show_readout(&self, cx: &mut ToolContext<'_>) {
        let Some(rect) = self.press.as_ref().and_then(|press| self.pixels(press.size)) else { return };
        let labels = labels_in(self.labels.as_ref(), &cx.canvas.ctx);
        let units = Units::of_ctx(&cx.canvas.ctx);
        let (width, height) = (units.pixels(rect.width()), units.pixels(rect.height()));
        cx.readout(Readout::new("craft_imaging::select::marquee").row(labels.width, width).row(labels.height, height));
    }
}

/// The one-pixel row (column) under `at`, kept on the canvas, across the whole `size`.
fn single_line(kind: MarqueeKind, size: [u32; 2], at: Point) -> Option<Rect> {
    let [w, h] = size.map(f64::from);
    if w < 1.0 || h < 1.0 || !at.x.is_finite() || !at.y.is_finite() {
        return None;
    }
    match kind {
        MarqueeKind::SingleRow => {
            let y = at.y.floor().clamp(0.0, h - 1.0);
            Some(Rect::new(0.0, y, w, y + 1.0))
        }
        MarqueeKind::SingleColumn => {
            let x = at.x.floor().clamp(0.0, w - 1.0);
            Some(Rect::new(x, 0.0, x + 1.0, h))
        }
        MarqueeKind::Rectangle | MarqueeKind::Ellipse => None,
    }
}

impl CanvasTool for SelectToolPass<'_, MarqueeTool> {
    /// The crosshair with the operation's badge (`SelectionOperation::badge`, live modifiers);
    /// over the selection the cursors of `drag::cursor_over`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1384-1408 (over a selection);
        // crates/ui-egui/src/canvas/tool_feedback.rs:158-169 `draw_badge` (the badge).
        let (tool, probe) = (&*self.tool, self.probe);
        let crosshair = ToolCursor::System(CursorIcon::Crosshair);
        let Some(pointer) = cx.pointer else { return crosshair };
        let dragging = tool.inside.as_ref().map(InsideDrag::kind);
        let draws_a_line = matches!(tool.kind, MarqueeKind::SingleRow | MarqueeKind::SingleColumn);
        if tool.press.is_none()
            && !draws_a_line
            && let Some(cursor) = cursor_over(probe, pointer.raw, cx.modifiers, tool.bar, PressRule::Drags, dragging)
        {
            return cursor;
        }
        // ⌘ is the Move tool's: no badge for the operation.
        let badge = match &tool.press {
            Some(press) => press.operation.badge(),
            None if cx.modifiers.command => SelectionBadge::New,
            None => SelectionOperation::resolve(tool.bar, cx.modifiers, probe.has_selection()).badge(),
        };
        selection_cursor(&cx.canvas.ctx, pointer.screen, badge, PointerInk::System);
        crosshair
    }

    /// A press inside the selection begins a selection drag; a ⌘ press outside is `Ignored`;
    /// otherwise the marquee begins at `pointer.doc` with the operation latched
    /// (`MarqueeGesture::press(pointer.screen, pointer.doc, pointer.modifiers)`).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/tool_input.rs:582-610 `selection_drag_kind`;
        // crates/ui-egui/src/canvas/tool_feedback.rs:88-99 `document_selection_mode`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        tool.reset();
        // pc: crates/ui-egui/src/canvas/tool_input.rs:590 (the single row and column never drag).
        let draws_a_line = matches!(tool.kind, MarqueeKind::SingleRow | MarqueeKind::SingleColumn);
        if !draws_a_line && let Some(kind) = drag_kind(probe, pointer.raw, pointer.modifiers, tool.bar, PressRule::Drags) {
            let style = ConstrainStyle::of(cx.tokens);
            let (drag, event) = InsideDrag::begin(&style, kind, pointer.doc, pointer.modifiers);
            tool.inside = Some(drag);
            tool.events.push(event);
            return ToolAction::Claimed;
        }
        if pointer.modifiers.command {
            return ToolAction::Ignored;
        }
        tool.press = Some(MarqueePress {
            gesture: MarqueeGesture::press(pointer.screen, pointer.doc, pointer.modifiers),
            operation: SelectionOperation::resolve(tool.bar, pointer.modifiers, probe.has_selection()),
            space: false,
            size: probe.size(),
            last: pointer.doc,
            corners: None,
        });
        tool.advance(pointer, cx);
        tool.show_readout(cx);
        ToolAction::Claimed
    }

    /// Sizes (or with Space moves) the marquee, or moves the selection drag; sets the W and H
    /// readout. The marquee runs `MarqueeGesture::{follow, begin_past, size_to}` with the
    /// `marquee.click_slop` token (`Marquee::style(..).click_slop`), the tool's constraint and its Space flag; no rule is written
    /// here.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:88-127 (corners and the preview rectangle).
        let tool = &mut *self.tool;
        if let Some(inside) = tool.inside.as_mut() {
            if let Some(event) = inside.drag(pointer.doc, pointer.modifiers, cx.view.zoom) {
                tool.events.push(event);
            }
            return;
        }
        tool.advance(pointer, cx);
        tool.show_readout(cx);
    }

    /// Commits: `Commit { shape, operation }` for a rectangle or ellipse with area, `Deselect` for a
    /// click, the drag's `Commit` for a selection drag.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:109-127 (a click deselects; Fixed Size places).
        let tool = &mut *self.tool;
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.end(pointer.doc, pointer.modifiers, cx.view.zoom));
            return;
        }
        tool.advance(pointer, cx);
        let Some(press) = tool.press else { return };
        let rect = tool.pixels(press.size);
        tool.press = None;
        tool.events.push(match rect {
            Some(rect) if tool.kind == MarqueeKind::Ellipse => {
                SelectEvent::Commit { shape: SelectionShape::Ellipse(rect), operation: press.operation }
            }
            Some(rect) => SelectEvent::Commit { shape: SelectionShape::Rectangle(rect), operation: press.operation },
            None => SelectEvent::Deselect,
        });
    }

    /// Esc, a lost pointer or a tool switch: the marquee is dropped silently; a selection drag
    /// reports its `Cancel`.
    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: `InsideDrag::cancel` for a drag; nothing for a marquee.
        let tool = &mut *self.tool;
        if let Some(inside) = tool.inside.take() {
            tool.events.push(inside.cancel());
        }
        tool.press = None;
    }

    /// Space down and up during a drag (moves the marquee); taken so the app's Space (the Hand) does
    /// not run.
    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // new: sets `MarqueePress::space`, which `drag` passes to `MarqueeGesture::size_to`.
        if key.key != Key::Space {
            return false;
        }
        let Some(press) = self.tool.press.as_mut() else { return false };
        press.space = key.pressed;
        true
    }

    /// The ants of the marquee being dragged (`Marquee::rect` or `Marquee::ellipse`,
    /// `MarqueeLook::Ants`).
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/canvas/drag.rs:213-240 `marquee_ants` (built in document space,
        // mapped point by point, #2512).
        let Some(rect) = self.tool.press.as_ref().and_then(|press| self.tool.pixels(press.size)) else { return };
        let marquee = if self.tool.kind == MarqueeKind::Ellipse { Marquee::ellipse(rect, "") } else { Marquee::rect(rect, "") };
        marquee.paint(painter, cx.view, MarqueeLook::Ants, cx.time);
    }

    /// Corners snap to the document's snap targets (a point).
    fn snap_gesture(&self) -> Option<SnapGesture> {
        Some(SnapGesture::Point)
    }

    /// The marquee being dragged as one AccessKit object: named `SelectLabels::outline`, valued with
    /// its pixel size (as the W and H readout), over the pixel rectangle; `None` with no marquee.
    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: SE deviation 3 (no `Ui` on the host); `ToolNode::new(..).value(..).bounds(..)`.
        let rect = self.tool.press.as_ref().and_then(|press| self.tool.pixels(press.size))?;
        let labels = labels_in(self.tool.labels.as_ref(), &cx.canvas.ctx);
        let units = Units::of_ctx(&cx.canvas.ctx);
        let value = craft_i18n::fmt(&labels.size, &[("w", &units.pixels(rect.width())), ("h", &units.pixels(rect.height()))]);
        Some(ToolNode::new(labels.outline).value(value).bounds(rect))
    }
}
