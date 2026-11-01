//! [`ConstraintTool`]: Adaptive Wide Angle's Constraint tool (task WS-WA1).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]

use egui::{Painter, PointerButton};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};

use super::WideAngleLabels;
use super::session::{WideAngleConstraintEdit, WideAngleGeometry, WideAngleSession};
use super::style::WideAngleStyle;

/// The state of the Constraint tool between passes: the drag in progress, the constraint picked
/// for the keyboard, the constraint a right-click opened the menu for, the edits not yet taken.
#[derive(Debug, Default)]
pub struct ConstraintTool {
    drag: Option<(Point, Point)>,
    picked: Option<usize>,
    menu_for: Option<usize>,
    edits: Vec<WideAngleConstraintEdit>,
}

impl ConstraintTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The tool for this pass over `session` (read only: edits are reported) and the app's
    /// geometry.
    pub fn pass<'a>(
        &'a mut self,
        session: &'a WideAngleSession,
        geometry: &'a dyn WideAngleGeometry,
        style: &'a WideAngleStyle,
        labels: &'a WideAngleLabels,
    ) -> ConstraintToolPass<'a> {
        ConstraintToolPass { tool: self, session, geometry, style, labels }
    }

    pub fn take_edits(&mut self) -> Vec<WideAngleConstraintEdit> {
        std::mem::take(&mut self.edits)
    }

    /// A drag is drawing a constraint: the canvas shows the source, not the corrected preview, and
    /// the history waits.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The constraint the context menu is open for (the workspace draws the menu: Free,
    /// Horizontal, Vertical as radio items, then Delete).
    pub fn menu_for(&self) -> Option<usize> {
        self.menu_for
    }

    /// The menu closed (picked or dismissed).
    pub fn close_menu(&mut self) {
        self.menu_for = None;
    }
}

/// [`ConstraintTool`] as the `CanvasTool` of one pass (A).
///
/// - A drag draws a constraint from the press to the pointer (a 2 pt line in the accent while it
///   runs); on release it is added when longer than 6 screen points
///   ([`WideAngleConstraint::from_drag`](super::WideAngleConstraint::from_drag) with Shift at
///   the release).
/// - A secondary click opens the menu for the constraint whose curve passes nearest the click
///   within `wide_angle.hit` (10 points): `menu_for` is set; elsewhere nothing.
/// - Keyboard parts: each constraint is a part ("Constraint 2, Horizontal"); Page picks, arrows
///   move it one document pixel (Shift ten) as `WideAngleConstraintEdit::Move`, Delete and
///   Backspace delete it.
/// - Paint: each constraint as its curve ([`WideAngleGeometry::arc`], 24 segments; through
///   [`WideAngleGeometry::corrected`] over the corrected preview), `wide_angle.line` (2) in the
///   orientation's colour (accent, `status.warn.fg`, `status.bad.fg`) over the `.halo`, with end
///   dots of `wide_angle.end` (3.5).
pub struct ConstraintToolPass<'a> {
    tool: &'a mut ConstraintTool,
    session: &'a WideAngleSession,
    geometry: &'a dyn WideAngleGeometry,
    style: &'a WideAngleStyle,
    labels: &'a WideAngleLabels,
}

impl CanvasTool for ConstraintToolPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the crosshair (PhotoCraft shows the default arrow).
        todo!()
    }

    fn button_for(&self, modifiers: egui::Modifiers) -> PointerButton {
        // new: the primary button draws; the secondary click comes through the canvas response.
        PointerButton::Primary
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:289-290 (drag start).
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:291-295.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:297-308 (over 6 px; Shift orients).
        todo!()
    }

    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:310-322 (a secondary click on
        // `cx.canvas` opens the menu for the constraint whose curve passes within 10 px).
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the drag.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: arrows move and Delete removes the picked constraint (lead call 49).
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:331-356 (curves, colours, end dots).
        todo!()
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: constraints as keyboard parts (lead call 49).
        todo!()
    }

    fn picked(&self) -> Option<u64> {
        self.tool.picked.map(|i| i as u64)
    }

    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        self.tool.picked = key.and_then(|k| usize::try_from(k).ok());
    }
}
