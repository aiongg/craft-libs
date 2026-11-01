//! Liquify's tools, its brush and the stroke recorder (task WS-LQ1).

use egui::Modifiers;
use serde::{Deserialize, Serialize};

use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::patterns::ToolSlot;

use super::LiquifyLabels;
use super::session::LiquifyStroke;

/// The tools of Liquify's strip, in strip order, with Photoshop's letters. Serialised with the
/// engine's names (`forwardWarp`, `twirlCw`, `lassoMask`, …) so a [`LiquifyStroke`] passes to
/// PhotoCraft's engine unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiquifyTool {
    /// W.
    #[default]
    ForwardWarp,
    /// R.
    Reconstruct,
    /// E.
    Smooth,
    /// C.
    #[serde(rename = "twirlCw")]
    TwirlClockwise,
    /// No letter (Photoshop's Twirl is one tool; PhotoCraft shows both directions).
    #[serde(rename = "twirlCcw")]
    TwirlCounterclockwise,
    /// S.
    Pucker,
    /// B.
    Bloat,
    /// O.
    PushLeft,
    /// F.
    Freeze,
    /// D.
    Thaw,
    /// L: drag to freeze an area, Alt-drag to thaw it.
    #[serde(rename = "lassoMask")]
    FreezeLasso,
    /// H.
    Hand,
    /// Z.
    Zoom,
}

impl LiquifyTool {
    /// The strip's tools in order.
    pub const ALL: [LiquifyTool; 13] = [
        LiquifyTool::ForwardWarp,
        LiquifyTool::Reconstruct,
        LiquifyTool::Smooth,
        LiquifyTool::TwirlClockwise,
        LiquifyTool::TwirlCounterclockwise,
        LiquifyTool::Pucker,
        LiquifyTool::Bloat,
        LiquifyTool::PushLeft,
        LiquifyTool::Freeze,
        LiquifyTool::Thaw,
        LiquifyTool::FreezeLasso,
        LiquifyTool::Hand,
        LiquifyTool::Zoom,
    ];

    /// The tool's letter: W R E C S B O F D L H Z; `None` for Twirl Counterclockwise.
    #[expect(clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn shortcut(self) -> Option<char> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:808 `shortcut`.
        todo!()
    }

    /// The tool a press with Alt held paints with: the twirls reverse, Pucker and Bloat swap; the
    /// others are themselves. (Alt on the Freeze Lasso thaws: [`LiquifyStroke::amount`].)
    #[expect(clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn alternate(self) -> LiquifyTool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:658 `alt_tool`.
        todo!()
    }

    /// Whether the tool paints with the round brush (all but the Freeze Lasso, Hand and Zoom).
    pub fn is_brush(self) -> bool {
        !matches!(self, LiquifyTool::FreezeLasso | LiquifyTool::Hand | LiquifyTool::Zoom)
    }

    /// Whether the tool's strokes change the freeze mask (Freeze, Thaw, Freeze Lasso).
    pub fn edits_mask(self) -> bool {
        matches!(self, LiquifyTool::Freeze | LiquifyTool::Thaw | LiquifyTool::FreezeLasso)
    }

    /// The strip: one slot per tool, `group_end` after Smooth, Push Left, Thaw and the Freeze
    /// Lasso (PhotoCraft's 6 pt gaps after the first three; the fourth separates Hand and Zoom),
    /// each tool's label from `labels` and its letter as the slot's `shortcut`. The icons are
    /// library `Icon`s until app icons land (gap 27).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn slots(labels: &LiquifyLabels) -> Vec<ToolSlot<LiquifyTool>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:888 `draw_tool_strip` (gaps at :904),
        // :791 `tool_icon`.
        todo!()
    }
}

/// The brush of the painting tools. Serialised in [`LiquifySettings`](super::LiquifySettings).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LiquifyBrush {
    /// Diameter in document pixels, 1..=15000 ([`Self::SIZE_MAX`]); the slider shows 1..=3000
    /// ([`Self::SLIDER_MAX`]), the keys and the resize reach further.
    pub size: f64,
    /// How far the full effect reaches towards the edge, 0..=100.
    pub density: f64,
    /// Strength, 0..=100.
    pub pressure: f64,
    /// Speed of the tools that act while the pointer rests, 0..=100.
    pub rate: f64,
}

impl Default for LiquifyBrush {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:73-76.
        Self { size: 100.0, density: 50.0, pressure: 100.0, rate: 80.0 }
    }
}

impl LiquifyBrush {
    /// The largest size the keys and the resize gesture set.
    pub const SIZE_MAX: f64 = 15000.0;
    /// The Size slider's top.
    pub const SLIDER_MAX: f64 = 3000.0;

    /// `[`: the size × 0.9 rounded, at least one pixel smaller, not below 1.
    #[expect(clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn smaller(self) -> Self {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:765-768.
        todo!()
    }

    /// `]`: the size × 1.1 rounded, at least one pixel larger, not above [`Self::SIZE_MAX`].
    #[expect(clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn larger(self) -> Self {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:769-771.
        todo!()
    }
}

/// Turns a press, drags and a release into one [`LiquifyStroke`] with its [`EditEvent`]s. Ui-free:
/// the canvas tool feeds it document points.
///
/// - [`begin`](Self::begin) starts a stroke of `tool` (its [`alternate`](LiquifyTool::alternate)
///   with Alt) with the brush at the press, and reports `Begin` with the one-point stroke.
/// - [`extend`](Self::extend) adds a point (skipped when equal to the last) and reports `Update`
///   with the whole stroke so far.
/// - [`end`](Self::end) adds the release point when it differs from the last, so a short stroke is
///   recorded whole (PhotoCraft #8055: a stroke shorter than the drag threshold still paints), and
///   reports `Commit` (`changed: true`).
/// - [`cancel`](Self::cancel) drops the stroke and reports `Cancel`.
///
/// The Freeze Lasso is not recorded here: the canvas tool draws it with an `OutlineDraft` and
/// records the closed polygon with `amount` 1 (0 with Alt at the press).
#[derive(Clone, Debug, Default)]
pub struct StrokeRecorder {
    stroke: Option<(GestureId, LiquifyStroke)>,
}

impl StrokeRecorder {
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn begin(&mut self, tool: LiquifyTool, brush: &LiquifyBrush, at: [f64; 3], modifiers: Modifiers) -> EditEvent<LiquifyStroke> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:263 `begin`, :240 `template`.
        todo!()
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn extend(&mut self, at: [f64; 3], modifiers: Modifiers) -> Option<EditEvent<LiquifyStroke>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:279 `extend`; :704-713 (an equal point is
        // skipped).
        todo!()
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn end(&mut self, at: [f64; 2], modifiers: Modifiers) -> Option<EditEvent<LiquifyStroke>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:714-727 (`Up` adds the release point with
        // the last pressure, then `end`).
        todo!()
    }

    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn cancel(&mut self, reason: CancelReason) -> Option<EditEvent<LiquifyStroke>> {
        // new: PhotoCraft has no stroke cancel; the field already took the dabs, so the workspace
        // follows a Cancel with a Replay.
        todo!()
    }

    /// Whether a stroke is in progress.
    pub fn is_active(&self) -> bool {
        self.stroke.is_some()
    }
}
