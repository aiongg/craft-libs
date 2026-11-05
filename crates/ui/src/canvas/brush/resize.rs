//! Photoshop's on-canvas brush resize.

use egui::{Color32, Modifiers, Painter, PointerButton, Pos2, Stroke};
use kurbo::Point;

use super::{BrushStyle, TipFalloff};
use crate::canvas::{CanvasView, Readout};
use crate::edit::{CancelReason, EditEvent, GestureId};
use crate::localized::{UnitKind, Units};

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
    /// The values for the pointer at `at`: [`track`](Self::track)'s rule.
    fn values(run: &ResizeRun, view: &CanvasView, at: Pos2) -> BrushDimensions {
        let zoom = if view.zoom.is_finite() && view.zoom > 0.0 { view.zoom } else { 1.0 };
        let from = view.to_screen(run.anchor);
        let finite = |v: f32| if v.is_finite() { f64::from(v) } else { 0.0 };
        let (dx, dy) = (finite(at.x - from.x) / zoom, finite(at.y - from.y) / zoom);
        let max = run.options.max_diameter.max(1.0);
        let diameter = (run.start.diameter + 2.0 * dx).round().clamp(1.0, max);
        let travel = f64::from(run.options.travel);
        let shift = if travel.is_finite() && travel > 0.0 { dy * zoom / travel } else { 0.0 };
        let level = ((run.start.level + shift).clamp(0.0, 1.0) * 100.0).round() / 100.0;
        BrushDimensions { diameter, level }
    }

    /// Whether a press of `button` with `modifiers` is a resize: Alt with the secondary button
    /// (Photoshop on Windows), or Ctrl + Alt with the primary (Control + Option on a Mac), on every
    /// platform. Alt with the primary alone is not (the painting tools' eyedropper).
    /// [`takes`](Self::takes) lets the secondary button through.
    pub fn chord(button: PointerButton, modifiers: Modifiers) -> bool {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:47 `is_gesture` (ctrl && alt), :53 `is_right_gesture` (alt).
        let alt_secondary = button == PointerButton::Secondary && modifiers.alt;
        let ctrl_alt_primary = button == PointerButton::Primary && modifiers.alt && modifiers.ctrl;
        alt_secondary || ctrl_alt_primary
    }

    /// Whether the tool host hands a press of `button` with `modifiers` to a brush tool: the
    /// primary button always, the secondary only with Alt (Photoshop's Alt + right-drag resize),
    /// so a plain right-click stays the canvas menu's. Pass it to
    /// [`CanvasTool::takes`](crate::canvas::CanvasTool::takes) beside [`chord`](Self::chord).
    pub fn takes(button: PointerButton, modifiers: Modifiers) -> bool {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:53 `is_right_gesture` (alt).
        button == PointerButton::Primary || (button == PointerButton::Secondary && modifiers.alt)
    }

    /// Starts a resize at the screen point `at` from the brush's `start` values. A resize whose
    /// release was lost is dropped first without an event (on the tool host a lost pointer has
    /// already cancelled it). Returns `Begin`, or `None` when `view` cannot map `at` (a non-finite
    /// point or zoom): the press is then not a resize.
    pub fn begin(&mut self, view: &CanvasView, at: Pos2, start: BrushDimensions, options: &BrushResizeOptions) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:71 `pointer` Down (a new press ends any resize; start
        // (size, hardness or opacity); the anchor in document pixels).
        if !(at.x.is_finite() && at.y.is_finite() && view.zoom.is_finite() && view.zoom > 0.0) {
            self.run = None;
            return None;
        }
        let anchor = view.to_doc(at);
        if !(anchor.x.is_finite() && anchor.y.is_finite()) {
            self.run = None;
            return None;
        }
        let id = GestureId::next();
        self.run = Some(ResizeRun { id, anchor, start, now: start, options: *options });
        Some(EditEvent::begin(id))
    }

    /// The pointer moved to `at` (screen). With `d` the screen offset from the anchor's screen
    /// point over the zoom: `diameter = start + 2 × d.x`, rounded, clamped to 1..=`max_diameter`;
    /// `level = start + d.y × zoom / travel`, clamped to 0..=1 and rounded to a whole percent.
    /// Non-finite offsets count as 0, a non-finite or non-positive zoom as 1. Returns `Update`
    /// when either value changed, else `None`.
    pub fn track(&mut self, view: &CanvasView, at: Pos2, modifiers: Modifiers) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:58 `resized`, :113 `update` (screen-relative delta
        // over the zoom; no event when nothing changed).
        let run = self.run.as_mut()?;
        let now = Self::values(run, view, at);
        if now == run.now {
            return None;
        }
        run.now = now;
        Some(EditEvent::update(run.id, now, modifiers))
    }

    /// The release at `at`: [`track`](Self::track)'s values, then `Commit` with `changed` false
    /// when they equal the start. `None` when no resize runs.
    pub fn finish(&mut self, view: &CanvasView, at: Pos2, modifiers: Modifiers) -> Option<EditEvent<BrushDimensions>> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:95 `pointer` Up.
        let run = self.run.as_mut()?;
        let now = Self::values(run, view, at);
        run.now = now;
        let run = self.run.take()?;
        Some(EditEvent::commit(run.id, now, now != run.start, modifiers))
    }

    /// Esc, a lost pointer or a disabled host: `Cancel` with
    /// [`CancelReason::Escape`](crate::edit::CancelReason::Escape); the app's gesture host puts the
    /// start values back. `None` when no resize runs.
    pub fn cancel(&mut self) -> Option<EditEvent<BrushDimensions>> {
        // new: decisions §2.5 (Photoshop has no cancel; step7-brush lead call 6).
        let run = self.run.take()?;
        Some(EditEvent::cancel(run.id, CancelReason::Escape))
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
        let Some(run) = &self.run else { return };
        let centre = view.to_screen(run.anchor);
        falloff.paint(painter, centre, view.zoom as f32, run.options.tint);
        let radius = ((run.now.diameter / 2.0 * view.zoom) as f32).max(1.0);
        painter.circle_stroke(centre, radius + 0.5, Stroke::new(style.cursor.stroke, style.cursor.halo));
        painter.circle_stroke(centre, radius, Stroke::new(style.cursor.stroke, style.cursor.line));
    }

    /// The readout beside the pointer: `labels.diameter` with the diameter as whole pixels
    /// ([`Units::pixels`]) and the level's label (`hardness`, `opacity` or `density`) with a whole
    /// percentage. `None` when no resize runs. The tool hands it to
    /// [`ToolContext::readout`](crate::canvas::ToolContext::readout).
    pub fn readout(&self, units: &Units, labels: &BrushResizeLabels) -> Option<Readout> {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:153 (`Diameter:` "{size} px", `Hardness:` or `Opacity:` "{n}%").
        let run = self.run.as_ref()?;
        let label = match run.options.level {
            BrushLevel::Hardness => &labels.hardness,
            BrushLevel::Opacity => &labels.opacity,
            BrushLevel::Density => &labels.density,
        };
        let pixels = units.pixels(run.now.diameter.round().max(0.0));
        let level = units.text(UnitKind::Percent, run.now.level * 100.0, 0..=0);
        Some(Readout::new("brush-resize-readout").row(labels.diameter.as_str(), pixels).row(label.as_str(), level))
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
