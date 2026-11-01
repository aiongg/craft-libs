//! Photoshop's on-canvas brush resize.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Modifiers, Painter, PointerButton, Pos2};
use kurbo::Point;

use super::{BrushStyle, TipFalloff};
use crate::canvas::{CanvasView, Readout};
use crate::edit::{EditEvent, GestureId};
use crate::localized::LocaleNumbers;

/// What the vertical travel of a resize changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrushLevel {
    /// The tip's hardness (Photoshop's default, "Vary Round Brush Hardness on HUD" on).
    #[default]
    Hardness,
    /// The brush opacity ("Vary Round Brush Hardness on HUD" off).
    Opacity,
    /// The brush density (Liquify).
    Density,
}

/// The two values a resize sets: the diameter in document units and the level (0..=1) its
/// [`BrushLevel`] names.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrushDimensions {
    pub diameter: f64,
    pub level: f64,
}

/// How a resize behaves, given by the app at the press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushResizeOptions {
    /// What up and down change.
    pub level: BrushLevel,
    /// The largest diameter the app accepts; the smallest is 1.
    pub max_diameter: f64,
    /// Screen points of vertical travel from level 0 to 1 (`brush.resize_travel`).
    pub travel: f32,
    /// The preview's tint: the user's Brush Preview colour, `brush.preview_tint` by default. A
    /// user colour, exempt from contrast.
    pub tint: Color32,
}

impl BrushResizeOptions {
    /// Hardness, `max_diameter`, and the travel and tint of `style`.
    pub fn new(style: &BrushStyle, max_diameter: f64) -> Self {
        Self { level: BrushLevel::Hardness, max_diameter, travel: style.resize_travel, tint: style.preview_tint }
    }

    /// What up and down change.
    pub fn level(mut self, level: BrushLevel) -> Self {
        self.level = level;
        self
    }

    /// The preview's tint (the user's Brush Preview colour).
    pub fn tint(mut self, tint: Color32) -> Self {
        self.tint = tint;
        self
    }
}

/// A resize in progress.
#[derive(Clone, Copy, Debug)]
struct ResizeRun {
    id: GestureId,
    /// Where the press was, in document units: the circle stays there.
    anchor: Point,
    start: BrushDimensions,
    now: BrushDimensions,
    options: BrushResizeOptions,
}

/// Photoshop's brush resize as a value the tool owns: with a chord ([`chord`](Self::chord)) a
/// press on the canvas resizes the brush instead of painting. Moving right makes the diameter
/// bigger and down makes the level harder, in screen directions however the view is turned or
/// flipped and in every text direction. The circle stays where the press was, tinted by the
/// tip's falloff, the pointer is hidden (the tool returns [`ToolCursor::Hidden`](crate::canvas::ToolCursor::Hidden)
/// while [`active`](Self::active)), and the readout follows the pointer.
///
/// The gesture reports [`EditEvent<BrushDimensions>`]: `Begin` at the press, `Update` when a
/// value changes, `Commit` at the release (`changed` false for a click without a change), and
/// `Cancel` on Esc, a lost pointer or a disabled host. Brush size and level are tool options:
/// the app feeds the events to a `GestureHost<QuietMarks<_>>`, one journal entry and no history
/// state. There is no keyboard path for the drag (Photoshop has none); the bracket keys are the
/// app's commands.
#[derive(Clone, Debug, Default)]
pub struct BrushResize {
    run: Option<ResizeRun>,
}

impl BrushResize {
    /// Whether a press of `button` with `modifiers` is a resize: Alt with the secondary button
    /// (Photoshop on Windows), or Ctrl + Alt with the primary (Control + Option on a Mac), on every
    /// platform. Alt with the primary alone is not (the painting tools' eyedropper). The tool
    /// takes the secondary button with Alt through
    /// [`CanvasTool::button_for`](crate::canvas::CanvasTool::button_for).
    pub fn chord(button: PointerButton, modifiers: Modifiers) -> bool {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:47 `is_gesture` (ctrl && alt), :53 `is_right_gesture` (alt).
        todo!()
    }

    /// Starts a resize at the screen point `at` from the brush's `start` values. A resize whose
    /// release was lost is dropped first without an event (on the tool host a lost pointer has
    /// already cancelled it). Returns `Begin`, or `None` when `view` cannot map `at` (a non-finite
    /// point or zoom): the press is then not a resize.
    pub fn begin(&mut self, view: &CanvasView, at: Pos2, start: BrushDimensions, options: &BrushResizeOptions) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:71 `pointer` Down (a new press ends any resize; start
        // (size, hardness or opacity); the anchor in document pixels).
        todo!()
    }

    /// The pointer moved to `at` (screen). With `d` the screen offset from the anchor's screen
    /// point over the zoom: `diameter = start + 2 × d.x`, rounded, clamped to 1..=`max_diameter`;
    /// `level = start + d.y × zoom / travel`, clamped to 0..=1 and rounded to a whole percent.
    /// Non-finite offsets count as 0, a non-finite or non-positive zoom as 1. Returns `Update`
    /// when either value changed, else `None`.
    pub fn track(&mut self, view: &CanvasView, at: Pos2, modifiers: Modifiers) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:58 `resized`, :113 `update` (screen-relative delta
        // over the zoom; no event when nothing changed).
        todo!()
    }

    /// The release at `at`: [`track`](Self::track)'s values, then `Commit` with `changed` false
    /// when they equal the start. `None` when no resize runs.
    pub fn finish(&mut self, view: &CanvasView, at: Pos2, modifiers: Modifiers) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:95 `pointer` Up.
        todo!()
    }

    /// Esc, a lost pointer or a disabled host: `Cancel` with
    /// [`CancelReason::Escape`](crate::edit::CancelReason::Escape); the app's gesture host puts the
    /// start values back. `None` when no resize runs.
    pub fn cancel(&mut self) -> Option<EditEvent<BrushDimensions>> {
        // new: decisions §2.5 (Photoshop has no cancel; step7-brush lead call 6).
        todo!()
    }

    /// Whether a resize runs.
    pub fn active(&self) -> bool {
        self.run.is_some()
    }

    /// The values of the resize in progress.
    pub fn dimensions(&self) -> Option<BrushDimensions> {
        self.run.map(|r| r.now)
    }

    /// Paints the resize at the anchor through `view`: the `falloff` preview (at `view.zoom`) in
    /// the options' tint, then the circle of the current diameter (at least 1 point) as the cursor
    /// halo at radius + 0.5 and the cursor line at the radius, both `style.cursor.stroke` wide.
    /// `falloff` is the tip at the current values. Nothing when no resize runs.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &BrushStyle, falloff: &TipFalloff) {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:142 `draw` (tip preview, then black α 140 at r + 0.5
        // and white α 220 at r: replaced by `CursorStyle`'s halo and line).
        todo!()
    }

    /// The readout beside the pointer: `labels.diameter` with the diameter as whole pixels
    /// ("37 px", the region's digits) and the level's label (`hardness`, `opacity` or `density`)
    /// with a whole percentage (`LocaleNumbers::percent`). `None` when no resize runs. The tool
    /// hands it to [`ToolContext::readout`](crate::canvas::ToolContext::readout).
    pub fn readout(&self, numbers: &LocaleNumbers, labels: &BrushResizeLabels) -> Option<Readout> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:153 (`Diameter:` "{size} px", `Hardness:` or `Opacity:` "{n}%").
        todo!()
    }
}

crate::words! {
    /// The resize readout's row labels, English by default.
    #[non_exhaustive]
    pub struct BrushResizeLabels {
        diameter: "brushResize.diameter" = "Diameter:";
        hardness: "brushResize.hardness" = "Hardness:";
        opacity: "brushResize.opacity" = "Opacity:";
        density: "brushResize.density" = "Density:";
    }
}
