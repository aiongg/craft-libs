//! Liquify's tools, its brush and the stroke recorder (task WS-LQ1).

use egui::Modifiers;
use serde::{Deserialize, Serialize};

use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::patterns::{RailTool, ToolSlot};

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
    pub fn shortcut(self) -> Option<char> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:808 `shortcut`; H and Z are the rail's
        // Hand and Zoom (lead call 8).
        match self {
            LiquifyTool::ForwardWarp => Some('W'),
            LiquifyTool::Reconstruct => Some('R'),
            LiquifyTool::Smooth => Some('E'),
            LiquifyTool::TwirlClockwise => Some('C'),
            LiquifyTool::TwirlCounterclockwise => None,
            LiquifyTool::Pucker => Some('S'),
            LiquifyTool::Bloat => Some('B'),
            LiquifyTool::PushLeft => Some('O'),
            LiquifyTool::Freeze => Some('F'),
            LiquifyTool::Thaw => Some('D'),
            LiquifyTool::FreezeLasso => Some('L'),
            LiquifyTool::Hand => Some('H'),
            LiquifyTool::Zoom => Some('Z'),
        }
    }

    /// The tool a press with Alt held paints with: the twirls reverse, Pucker and Bloat swap; the
    /// others are themselves. (Alt on the Freeze Lasso thaws: [`LiquifyStroke::amount`].)
    pub fn alternate(self) -> LiquifyTool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:658 `alt_tool`.
        match self {
            LiquifyTool::TwirlClockwise => LiquifyTool::TwirlCounterclockwise,
            LiquifyTool::TwirlCounterclockwise => LiquifyTool::TwirlClockwise,
            LiquifyTool::Pucker => LiquifyTool::Bloat,
            LiquifyTool::Bloat => LiquifyTool::Pucker,
            other => other,
        }
    }

    /// The strip icon, from the library's set (PhotoCraft's `tool_icon` names, by meaning).
    fn icon(self) -> Icon {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:791 `tool_icon`.
        match self {
            LiquifyTool::ForwardWarp => Icon::Pointer,
            LiquifyTool::Reconstruct => Icon::BrushCleaning,
            LiquifyTool::Smooth => Icon::Droplet,
            LiquifyTool::TwirlClockwise => Icon::RotateCw,
            LiquifyTool::TwirlCounterclockwise => Icon::Undo,
            LiquifyTool::Pucker => Icon::Scan,
            LiquifyTool::Bloat => Icon::Maximize2,
            LiquifyTool::PushLeft => Icon::ChevronsLeft,
            LiquifyTool::Freeze => Icon::Lock,
            LiquifyTool::Thaw => Icon::LockOpen,
            LiquifyTool::FreezeLasso => Icon::Lasso,
            LiquifyTool::Hand => Icon::Hand,
            LiquifyTool::Zoom => Icon::ZoomIn,
        }
    }

    fn label(self, labels: &LiquifyLabels) -> &str {
        match self {
            LiquifyTool::ForwardWarp => &labels.forward_warp,
            LiquifyTool::Reconstruct => &labels.reconstruct_tool,
            LiquifyTool::Smooth => &labels.smooth,
            LiquifyTool::TwirlClockwise => &labels.twirl_clockwise,
            LiquifyTool::TwirlCounterclockwise => &labels.twirl_counterclockwise,
            LiquifyTool::Pucker => &labels.pucker,
            LiquifyTool::Bloat => &labels.bloat,
            LiquifyTool::PushLeft => &labels.push_left,
            LiquifyTool::Freeze => &labels.freeze,
            LiquifyTool::Thaw => &labels.thaw,
            LiquifyTool::FreezeLasso => &labels.freeze_lasso,
            LiquifyTool::Hand => &labels.hand,
            LiquifyTool::Zoom => &labels.zoom,
        }
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
    pub fn slots(labels: &LiquifyLabels) -> Vec<ToolSlot<LiquifyTool>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:888 `draw_tool_strip` (gaps at :904),
        // :791 `tool_icon`.
        (0_u32..)
            .zip(Self::ALL)
            .map(|(id, tool)| {
                let mut entry = RailTool::new(tool, tool.icon(), tool.label(labels));
                if let Some(letter) = tool.shortcut() {
                    entry = entry.shortcut(letter);
                }
                let group_end = matches!(tool, LiquifyTool::Smooth | LiquifyTool::PushLeft | LiquifyTool::Thaw | LiquifyTool::FreezeLasso);
                ToolSlot::new(id, vec![entry]).group_end(group_end)
            })
            .collect()
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
    pub fn smaller(mut self) -> Self {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:765-768.
        self.size = (self.size * 0.9).round().min(self.size - 1.0).clamp(1.0, Self::SIZE_MAX);
        self
    }

    /// `]`: the size × 1.1 rounded, at least one pixel larger, not above [`Self::SIZE_MAX`].
    pub fn larger(mut self) -> Self {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:769-771.
        self.size = (self.size * 1.1).round().max(self.size + 1.0).min(Self::SIZE_MAX);
        self
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
    /// A stroke already in progress is replaced without an event: the caller ends it first.
    pub fn begin(&mut self, tool: LiquifyTool, brush: &LiquifyBrush, at: [f64; 3], modifiers: Modifiers) -> EditEvent<LiquifyStroke> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:263 `begin`, :240 `template`.
        let tool = if modifiers.alt { tool.alternate() } else { tool };
        let stroke = LiquifyStroke {
            tool,
            size: brush.size,
            density: brush.density,
            pressure: brush.pressure,
            rate: brush.rate,
            points: vec![at],
            amount: None,
        };
        let id = GestureId::next();
        self.stroke = Some((id, stroke.clone()));
        EditEvent::begin_with(id, stroke)
    }

    pub fn extend(&mut self, at: [f64; 3], modifiers: Modifiers) -> Option<EditEvent<LiquifyStroke>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:279 `extend`; :704-713 (an equal point is
        // skipped).
        let (id, stroke) = self.stroke.as_mut()?;
        if stroke.points.last().is_some_and(|last| last[0] == at[0] && last[1] == at[1]) {
            return None;
        }
        stroke.points.push(at);
        Some(EditEvent::update(*id, stroke.clone(), modifiers))
    }

    /// A held dab: adds `at` even when it equals the last point, because a tool that acts while
    /// the brush rests dabs again in place (unlike [`extend`](Self::extend), which skips a
    /// repeated point), and reports `Update` with the whole stroke. `None` with no stroke.
    pub fn hold(&mut self, at: [f64; 3], modifiers: Modifiers) -> Option<EditEvent<LiquifyStroke>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1203-1207 (a stationary tool's `Move` at
        // the same point, 30 a second).
        let (id, stroke) = self.stroke.as_mut()?;
        stroke.points.push(at);
        Some(EditEvent::update(*id, stroke.clone(), modifiers))
    }

    pub fn end(&mut self, at: [f64; 2], modifiers: Modifiers) -> Option<EditEvent<LiquifyStroke>> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:714-727 (`Up` adds the release point with
        // the last pressure, then `end`).
        let (id, mut stroke) = self.stroke.take()?;
        if let Some(&[x, y, pressure]) = stroke.points.last()
            && (x != at[0] || y != at[1])
        {
            stroke.points.push([at[0], at[1], pressure]);
        }
        Some(EditEvent::commit(id, stroke, true, modifiers))
    }

    pub fn cancel(&mut self, reason: CancelReason) -> Option<EditEvent<LiquifyStroke>> {
        // new: PhotoCraft has no stroke cancel; the field already took the dabs, so the workspace
        // follows a Cancel with a Replay.
        let (id, _) = self.stroke.take()?;
        Some(EditEvent::cancel(id, reason))
    }

    /// Whether a stroke is in progress.
    pub fn is_active(&self) -> bool {
        self.stroke.is_some()
    }
}
