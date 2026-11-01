//! Content-Aware Fill's session (what OK and Apply fill with, with undo), its limits and its
//! remembered settings (task WS-CA1).

use serde::{Deserialize, Serialize};

use super::lasso::FillAreaEdit;
use crate::ui::select_mask::EditMode;

/// Sampling Area Options: how the sampling area starts before the brush strokes. Auto and
/// Rectangular start full (the brush subtracts), Custom starts empty (the brush adds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SamplingMode {
    #[default]
    Auto,
    Rectangular,
    Custom,
}

impl SamplingMode {
    pub const ALL: [SamplingMode; 3] = [SamplingMode::Auto, SamplingMode::Rectangular, SamplingMode::Custom];

    /// Whether the Sampling Brush adds in this mode (Custom) or subtracts (Auto, Rectangular).
    pub fn brush_adds(self) -> bool {
        self == SamplingMode::Custom
    }
}

/// Fill Settings › Color Adaptation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorAdaptation {
    None,
    #[default]
    Default,
    High,
    VeryHigh,
}

/// Fill Settings › Rotation Adaptation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RotationAdaptation {
    #[default]
    None,
    Low,
    Medium,
    High,
    Full,
}

/// Output Settings › Output To. Photoshop opens on New Layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FillOutputTo {
    #[serde(rename = "current")]
    CurrentLayer,
    #[default]
    #[serde(rename = "new")]
    NewLayer,
    #[serde(rename = "duplicate")]
    DuplicateLayer,
}

/// A Sampling Brush stroke: a hard round brush that adds to or subtracts from the sampling area.
/// Serialised as PhotoCraft's `samplingStrokes` entries (`{"mode": "add", "size": …, "points":
/// …}`, `caf_ui.rs:216` `stroke_json`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingStroke {
    pub mode: EditMode,
    /// Diameter in document pixels.
    pub size: f64,
    pub points: Vec<[f64; 2]>,
}

/// The engine's limits, given by the app: PhotoCraft's `MAX_CAF_EDITS` (1000) strokes, and as many
/// lasso edits, and `MAX_CAF_POINTS` (200 000) points in all
/// (`engine/src/cmd/edit_menu_cmds/content_aware.rs:105`, `:107`). The brush stops there, so
/// Apply and the preview never fail on what was painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillLimits {
    pub edits: usize,
    pub points: usize,
}

impl FillLimits {
    /// The message for a stroke or edit that would pass `points`.
    pub fn points_message(&self) -> String {
        // pc: crates/ui-egui/src/dialogs/caf_ui/preview.rs:84 `points_limit`.
        format!("at most {} points in all sampling strokes and lasso edits: undo some, or reset", self.points)
    }
}

/// What a Reset button resets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillReset {
    /// Sampling Area Options' Reset: Auto, Sample All Layers off, no strokes.
    Sampling,
    /// Fill Settings' Reset: the adaptation, Scale and Mirror to their defaults (not undoable,
    /// as PhotoCraft).
    Fill,
    /// The Lasso's Reset in the options bar: no fill-area edits.
    FillArea,
    /// The footer's Reset: everything, the settings included.
    All,
}

/// What OK and Apply fill with and the history snapshots: the sampling option, the brush strokes
/// and the lasso edits of the fill area. The tool, the brush size and the overlay are outside it
/// (risk 8).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FillSession {
    pub sampling: SamplingMode,
    pub strokes: Vec<SamplingStroke>,
    pub fill_edits: Vec<FillAreaEdit>,
}

impl FillSession {
    /// Points in the strokes and lasso edits so far, against [`FillLimits::points`]; `extra` is a
    /// stroke in progress.
    pub fn points_used(&self, extra: Option<&SamplingStroke>) -> usize {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:342 `points_used`.
        let strokes: usize = self.strokes.iter().chain(extra).map(|stroke| stroke.points.len()).sum();
        let lassos: usize = self.fill_edits.iter().map(FillAreaEdit::points_len).sum();
        strokes + lassos
    }

    /// Whether a new stroke may begin (or a stroke in progress take another point): `Err` with "at
    /// most N sampling strokes: undo some, or reset the sampling area" or the points message ("at
    /// most N points in all sampling strokes and lasso edits: undo some, or reset").
    pub fn may_begin_stroke(&self, limits: &FillLimits) -> Result<(), String> {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:353 `begin_stroke` (the checks), :371
        // `extend_stroke`; caf_ui/preview.rs:84 `points_limit`.
        if self.strokes.len() >= limits.edits {
            return Err(format!("at most {} sampling strokes: undo some, or reset the sampling area", limits.edits));
        }
        if self.points_used(None) >= limits.points {
            return Err(limits.points_message());
        }
        Ok(())
    }

    /// Records a finished stroke.
    pub fn add_stroke(&mut self, stroke: SamplingStroke) {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:389 `end_stroke`.
        self.strokes.push(stroke);
    }

    /// Records a fill-area edit within `limits`, or returns why not ("at most N lasso edits: undo
    /// some, or reset the fill area"; the points message). An edit the app then finds empties the
    /// fill area is taken back by the workspace (`ContentAwareFillWorkspace::refuse_fill_edit`).
    pub fn add_fill_edit(&mut self, edit: FillAreaEdit, limits: &FillLimits) -> Result<(), String> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:196 `fill_edit` (the count first,
        // then the points; a total equal to the limit passes).
        if self.fill_edits.len() >= limits.edits {
            return Err(format!("at most {} lasso edits: undo some, or reset the fill area", limits.edits));
        }
        if self.points_used(None) + edit.points_len() > limits.points {
            return Err(limits.points_message());
        }
        self.fill_edits.push(edit);
        Ok(())
    }

    /// A new sampling option: the strokes start over. Nothing changes when the option is the same
    /// and there are no strokes.
    pub fn set_sampling(&mut self, sampling: SamplingMode) -> bool {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:224 `set_sampling`.
        if self.sampling == sampling && self.strokes.is_empty() {
            return false;
        }
        self.strokes.clear();
        self.sampling = sampling;
        true
    }

    /// A Reset: [`FillReset::Sampling`] and [`FillReset::All`] change the session and `settings`,
    /// [`FillReset::Fill`] only `settings`, [`FillReset::FillArea`] only the fill edits. Returns
    /// whether the session changed.
    pub fn reset(&mut self, what: FillReset, settings: &mut FillSettings) -> bool {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:238 `reset`.
        let defaults = FillSettings::default();
        match what {
            FillReset::Fill => {
                settings.color_adaptation = defaults.color_adaptation;
                settings.rotation = defaults.rotation;
                settings.scale = defaults.scale;
                settings.mirror = defaults.mirror;
                false
            }
            FillReset::Sampling => {
                settings.sample_all_layers = defaults.sample_all_layers;
                settings.sampling = defaults.sampling;
                self.set_sampling(SamplingMode::Auto)
            }
            FillReset::FillArea => {
                let changed = !self.fill_edits.is_empty();
                self.fill_edits.clear();
                changed
            }
            FillReset::All => {
                let changed = *self != FillSession::default();
                *self = FillSession::default();
                *settings = defaults;
                changed
            }
        }
    }
}

/// Content-Aware Fill's remembered settings (PhotoCraft `CafOpts`,
/// `edit.contentAwareFill.workspace`), persisted by the app on Apply, OK and Cancel (serde, every
/// field defaulted). Defaults: Show Sampling Area on, Opacity 50, Color (142, 208, 106) (the 50 %
/// tint reads as Photoshop's (171, 204, 153)), Indicates Sampling Area, Auto, Sample All Layers
/// off, Color Adaptation Default, Rotation Adaptation None, Scale and Mirror off, Output To New
/// Layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FillSettings {
    pub show_overlay: bool,
    /// 0..=100.
    pub opacity: f64,
    pub color: [u8; 3],
    /// The tint marks what is left out (Indicates › Excluded Area), not the sampling area.
    pub excluded: bool,
    pub sampling: SamplingMode,
    pub sample_all_layers: bool,
    pub color_adaptation: ColorAdaptation,
    pub rotation: RotationAdaptation,
    pub scale: bool,
    pub mirror: bool,
    pub output: FillOutputTo,
}

impl Default for FillSettings {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:121-136 `CafOpts::default`, :62 `OVERLAY_COLOR`.
        Self {
            show_overlay: true,
            opacity: 50.0,
            color: [142, 208, 106],
            excluded: false,
            sampling: SamplingMode::Auto,
            sample_all_layers: false,
            color_adaptation: ColorAdaptation::Default,
            rotation: RotationAdaptation::None,
            scale: false,
            mirror: false,
            output: FillOutputTo::NewLayer,
        }
    }
}

/// `edit.contentAwareFill`'s parameters, serialised as PhotoCraft's `plan_params` without the
/// layer (the app adds `"layer"`): `sampling`, `sampleAllLayers`, `colorAdaptation`,
/// `rotationAdaptation`, `scale`, `mirror`, `output`, `fillEdits`, and `samplingStrokes` when
/// there are strokes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillCommand {
    pub sampling: SamplingMode,
    pub sample_all_layers: bool,
    pub color_adaptation: ColorAdaptation,
    pub rotation_adaptation: RotationAdaptation,
    pub scale: bool,
    pub mirror: bool,
    pub output: FillOutputTo,
    pub fill_edits: Vec<FillAreaEdit>,
    #[serde(rename = "samplingStrokes", skip_serializing_if = "Vec::is_empty")]
    pub strokes: Vec<SamplingStroke>,
}

impl FillCommand {
    /// The command for `session` with `settings`.
    pub fn new(session: &FillSession, settings: &FillSettings) -> FillCommand {
        // pc: crates/ui-egui/src/dialogs/caf_ui/preview.rs:65 `plan_params`.
        FillCommand {
            sampling: session.sampling,
            sample_all_layers: settings.sample_all_layers,
            color_adaptation: settings.color_adaptation,
            rotation_adaptation: settings.rotation,
            scale: settings.scale,
            mirror: settings.mirror,
            output: settings.output,
            fill_edits: session.fill_edits.clone(),
            strokes: session.strokes.clone(),
        }
    }
}
