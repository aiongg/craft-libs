//! [`RulerTool`]: the Ruler tool on the canvas tool host, with its overlay and keyboard parts.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-R2)")]

use egui::Painter;

use super::model::{RulerLine, RulerPart};
use craft_ui::canvas::{CanvasTool, RulerUnits, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::{EditEvent, GestureId};

craft_ui::words! {
    /// The Ruler tool's AccessKit names, English by default.
    #[non_exhaustive]
    pub struct RulerToolLabels {
        group: "rulerTool.group" = "Ruler";
        start: "rulerTool.start" = "Start point";
        end: "rulerTool.end" = "End point";
        protractor: "rulerTool.protractor" = "Protractor point";
        /// The group's value; `{length}` and `{angle}` are formatted numbers.
        value: "rulerTool.value" = "Length {length}, angle {angle}°";
    }
}

/// One pass's input: the line as the app stores it, and how its values read.
pub struct RulerInput<'a> {
    line: Option<RulerLine>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    labels: Option<RulerToolLabels>,
}

impl<'a> RulerInput<'a> {
    /// The pass over `line`, the document's Ruler line (`None`: no line yet).
    pub fn new(line: Option<RulerLine>) -> Self {
        Self { line, units: None, labels: None }
    }

    /// The ruler units of the parts' values ("x, y") and of the group's length; default: document
    /// pixels.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The AccessKit names; default: [`RulerToolLabels`] of the installed words.
    pub fn labels(mut self, labels: RulerToolLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

/// What a pass of the Ruler tool reports, from [`RulerPass::finish`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RulerOutput {
    /// The line's changes, in order: a drag (Begin, Update, Commit; Cancel on Esc, which puts the
    /// line back), an arrow press or Delete (`Set`; `None` clears). The app stores the value; it
    /// is not an undo step.
    pub events: Vec<EditEvent<Option<RulerLine>>>,
}

/// The Ruler tool, kept by the app while the tool is picked.
///
/// - Pointer: a press within `measure.hit` screen points of an end grabs it (the protractor's end
///   first, then the end, then the start); ⌥-press at an end starts the protractor arm at that
///   vertex (when the far end was hit, start and end swap first); elsewhere a new line starts.
///   ⇧ keeps the moving point on 45° steps about the fixed end (about the vertex for the
///   protractor arm, `snap_ray` with π/4). A new line shorter than [`MIN_LINE`](super::MIN_LINE)
///   document units is cleared on release. The reach is in screen points at every zoom.
/// - Keyboard: the line is one Tab stop through the host with the parts Start point, End point
///   and, with a protractor, Protractor point (values "x, y" in the ruler unit; the group's value
///   "Length L, angle A°"); Page Up and Down pick a part, the arrows move it 1 unit and ⇧ 10
///   ([`NudgeKeys::from_key`](craft_ui::behavior::nudge::NudgeKeys::from_key)); Delete or
///   Backspace clears the line.
/// - Look: each arm as the line guide (halo, then line) and a cross at each end
///   (`measure.handle_arm`), the picked part ringed while the keyboard has it.
/// - Cursor: the crosshair.
#[derive(Clone, Debug, Default)]
pub struct RulerTool {
    /// The part the drag moves, and whether it started a new line.
    drag: Option<(RulerPart, bool)>,
    gesture: Option<GestureId>,
    /// The line at the press, for Esc.
    before: Option<RulerLine>,
    picked: Option<RulerPart>,
}

impl RulerTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// This pass's tool for the host.
    pub fn pass<'a>(&'a mut self, input: RulerInput<'a>) -> RulerPass<'a> {
        RulerPass { tool: self, input, output: RulerOutput::default() }
    }
}

/// The Ruler tool for one pass of the host ([`RulerTool::pass`]).
pub struct RulerPass<'a> {
    tool: &'a mut RulerTool,
    input: RulerInput<'a>,
    output: RulerOutput,
}

impl RulerPass<'_> {
    /// The pass's events.
    pub fn finish(self) -> RulerOutput {
        self.output
    }
}

impl CanvasTool for RulerPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the precise crosshair (`ToolCursor::Crosshair` with the context's cursor style), as
        // the other measuring tools of the host.
        todo!()
    }

    /// Grabs an end, starts the protractor arm, or starts a new line (Begin).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:250 (Down), :210 `tolerance` (7 / zoom).
        todo!()
    }

    /// Moves the grabbed point, ⇧ on 45° rays about the fixed end (Update).
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:273 (Move), :219 `constrain`; here
        // `craft_ui::behavior::move_gesture::snap_ray` with π/4 (task K2).
        todo!()
    }

    /// Commit; a new line under [`MIN_LINE`](super::MIN_LINE) commits `None`.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:283 (Up).
        todo!()
    }

    /// Esc: Cancel; the app puts the line of the press back.
    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: the edit lifecycle (decisions §2.5).
        todo!()
    }

    /// Arrows move the picked part (no pick: the whole line); Delete and Backspace clear.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: lead call 25 (PhotoCraft has no keyboard path).
        todo!()
    }

    /// The arms and the end crosses in `MeasureStyle`.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:416 `draw_overlay` (the Ruler half), :408
        // `handle`.
        todo!()
    }

    /// The line's parts; `None` without a line.
    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: lead call 33; values through the units, the group's value from `RulerLine::reading`.
        todo!()
    }

    fn picked(&self) -> Option<u64> {
        self.tool.picked.map(RulerPart::key)
    }

    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        self.tool.picked = key.and_then(RulerPart::of_key);
    }
}
