//! Select and Mask's tools, their options and the canvas tool (task WS-SM2).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task WS-SM2")]

use egui::{Modifiers, Painter, PointerButton};

use craft_ui::canvas::brush::BrushResize;
use craft_ui::canvas::draft::OutlineDraft;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::edit::GestureId;
use craft_ui::patterns::ToolSlot;

use super::SelectMaskEvent;
use super::SelectMaskLabels;

/// The tools of Select and Mask's strip, with Photoshop's letters. Quick Selection and Object
/// Selection share the W slot, Lasso and Polygonal Lasso the L slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectMaskTool {
    /// W.
    #[default]
    Quick,
    /// R.
    RefineEdge,
    /// B.
    Brush,
    /// W (Shift+W cycles with Quick Selection).
    Object,
    /// L.
    Lasso,
    /// L (Shift+L cycles with Lasso).
    PolygonalLasso,
    /// H.
    Hand,
    /// Z.
    Zoom,
}

impl SelectMaskTool {
    pub const ALL: [SelectMaskTool; 8] = [
        SelectMaskTool::Quick,
        SelectMaskTool::RefineEdge,
        SelectMaskTool::Brush,
        SelectMaskTool::Object,
        SelectMaskTool::Lasso,
        SelectMaskTool::PolygonalLasso,
        SelectMaskTool::Hand,
        SelectMaskTool::Zoom,
    ];

    /// W R B W L L H Z.
    pub fn shortcut(self) -> char {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:114 `tip`.
        todo!()
    }

    /// Paints with a round brush whose size `[` and `]` change: Quick Selection, Refine Edge
    /// Brush, Brush.
    pub fn brushes(self) -> bool {
        matches!(self, SelectMaskTool::Quick | SelectMaskTool::RefineEdge | SelectMaskTool::Brush)
    }

    /// Adds to or subtracts from the selection (the bar's + and −): all but Hand and Zoom.
    pub fn edits(self) -> bool {
        !matches!(self, SelectMaskTool::Hand | SelectMaskTool::Zoom)
    }

    /// The strip: slot W (Quick Selection, Object Selection), R, B, slot L (Lasso, Polygonal
    /// Lasso; a right-click opens the flyout), `group_end` after it, H, Z. Labels from `labels`,
    /// letters as each tool's `shortcut`; `ToolRail::pick_by_key` gives W, L and their Shift
    /// cycles.
    pub fn slots(labels: &SelectMaskLabels) -> Vec<ToolSlot<SelectMaskTool>> {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1845-1868 (the strip, the shared L
        // slot with its corner triangle and menu), :101 `icon`.
        todo!()
    }
}

/// The diameters the three brush tools remember apart (1..=5000; 30 each when the workspace
/// opens). Not in the history, not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSizes {
    pub quick: f64,
    pub refine: f64,
    pub brush: f64,
}

impl Default for BrushSizes {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:749-751.
        Self { quick: 30.0, refine: 30.0, brush: 30.0 }
    }
}

impl BrushSizes {
    /// The sizes' range.
    pub const RANGE: std::ops::RangeInclusive<f64> = 1.0..=5000.0;

    /// The size of `tool` (the Brush's for a tool without its own).
    pub fn get(&self, tool: SelectMaskTool) -> f64 {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:467 `size`.
        todo!()
    }

    /// Sets the size of a brush tool, rounded and clamped to [`Self::RANGE`]; a non-finite value
    /// or another tool changes nothing.
    pub fn set(&mut self, tool: SelectMaskTool, size: f64) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:475 `set_size`.
        todo!()
    }

    /// `[` (`larger` false) and `]`: × 0.9 or × 1.1, at least one pixel of change.
    pub fn step(&mut self, tool: SelectMaskTool, larger: bool) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1405-1412.
        todo!()
    }
}

/// The tools' options: the bar's values, outside the history and not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectMaskToolOptions {
    /// + (true) or − in the bar.
    pub add: bool,
    pub sizes: BrushSizes,
    /// The Brush's hardness, 0..=100 (100).
    pub hardness: f64,
    /// Sample All Layers (Quick Selection, Refine Edge Brush, Object Selection, Select Subject).
    pub sample_all_layers: bool,
}

impl Default for SelectMaskToolOptions {
    fn default() -> Self {
        Self { add: true, sizes: BrushSizes::default(), hardness: 100.0, sample_all_layers: false }
    }
}

/// The state of Select and Mask's canvas between passes: the gesture being drawn, the Polygonal
/// Lasso's polygon, the brush resize, the events not yet taken.
#[derive(Debug, Default)]
pub struct SelectMaskCanvas {
    gesture: Option<(GestureId, SelectMaskTool, bool, Vec<Point>)>,
    polygon: Option<OutlineDraft>,
    resize: BrushResize,
    events: Vec<SelectMaskEvent>,
}

impl SelectMaskCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass: `tool` with `options` (the resize writes the brush size).
    /// `real_time` reports `Update`s while a stroke is drawn.
    pub fn tool<'a>(&'a mut self, tool: SelectMaskTool, options: &'a mut SelectMaskToolOptions, real_time: bool) -> SelectMaskCanvasTool<'a> {
        SelectMaskCanvasTool { canvas: self, tool, options, real_time }
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectMaskEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether a Polygonal Lasso polygon is open: Enter closes it and Esc drops it before the
    /// dialog sees them.
    pub fn polygon_open(&self) -> bool {
        self.polygon.is_some()
    }

    /// Enter: closes an open polygon (3 points or more) into a Lasso edit. Returns whether there
    /// was a polygon.
    pub fn close_polygon(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:870 `enter`, :590 `close_polygon`.
        todo!()
    }

    /// Esc: drops an open polygon. Returns whether there was one.
    pub fn drop_polygon(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:881 `escape`.
        todo!()
    }

    /// A gesture, a polygon or a resize is in progress: the history waits.
    pub fn busy(&self) -> bool {
        self.gesture.is_some() || self.polygon.is_some() || self.resize.active()
    }
}

/// [`SelectMaskCanvas`] as the `CanvasTool` of one pass.
///
/// - **Quick Selection, Brush, Refine Edge Brush**: press, drag, release is one stroke; a point
///   joins when it is at least `size / 8` (at least 1) pixels from the last. Reported as
///   [`SelectMaskEvent::Edit`] (or [`SelectMaskEvent::Refine`] for the Refine Edge Brush) Begin,
///   Update with the points so far (only with Real-Time Refinement on), Commit with the finished
///   `SelectMaskEdit` or `RefineStroke` (the bar's mode, reversed by Alt at the press; the
///   tool's size; the Brush's hardness; Sample All Layers). The workspace records the commit
///   within the limits.
/// - **Lasso**: a freehand outline (points 0.5 px apart), committed as `SelectMaskEdit::Lasso`
///   when it has 3 points or more.
/// - **Polygonal Lasso**: an [`OutlineDraft`] of `DraftKind::Polygonal` with a close reach of 6
///   points: a click adds a vertex, a click on the first vertex or a double click closes;
///   Backspace and Delete remove the last vertex (`ToolContext::keep_keys` while it is open).
/// - **Object Selection**: a click, or a drag shorter than 4 screen points, picks the object under
///   the press (`ObjectTarget::Point`); a longer drag gives `ObjectTarget::Rect` from the press
///   to the release.
/// - **Brush resize**: Alt + right-drag or Control + Alt + drag ([`BrushResize`] with
///   `BrushLevel::Hardness` for the Brush; the size only for the others, `max_diameter` 5000).
/// - Paint: the trail of the gesture (its newest part over the preview until a preview with it
///   arrives), the polygon with its rubber band, the brush outline (`BrushCursor`).
pub struct SelectMaskCanvasTool<'a> {
    canvas: &'a mut SelectMaskCanvas,
    tool: SelectMaskTool,
    options: &'a mut SelectMaskToolOptions,
    real_time: bool,
}

impl CanvasTool for SelectMaskCanvasTool<'_> {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1767 `canvas_input`.
        todo!()
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1730 `paint_cursor`.
        todo!()
    }

    fn button_for(&self, modifiers: Modifiers) -> PointerButton {
        // new: the brush resize chord (`BrushResize::chord`), as Liquify's.
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:599 `begin`.
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:620 `extend`.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1367-1370 (Up), :554 `finish`, :579
        // `end_gesture`.
        todo!()
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1363-1366 (Move: the hover point, the
        // polygon's rubber band).
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the gesture without an edit.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1413-1419 (Backspace, Delete).
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1658 `paint_canvas` (trail, polygon).
        todo!()
    }
}
