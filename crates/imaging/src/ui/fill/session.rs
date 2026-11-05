//! Content-Aware Fill's session (what OK and Apply fill with, with undo), its limits and its
//! remembered settings.

use serde::{Deserialize, Serialize};

use craft_ui::localized::Words;

use super::labels::{LASSO_EDIT_LIMIT, POINT_LIMIT, STROKE_LIMIT};
use super::lasso::FillAreaEdit;
use crate::ui::select_mask::EditMode;

/// Sampling Area Options: how the sampling area starts before the brush strokes. Auto and
/// Rectangular start full (the brush subtracts), Custom starts empty (the brush adds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SamplingMode {
    #[default]
    #[allow(missing_docs)]
    Auto,
    #[allow(missing_docs)]
    Rectangular,
    #[allow(missing_docs)]
    Custom,
}

impl SamplingMode {
    /// The three modes, in menu order.
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
    #[allow(missing_docs)]
    None,
    #[default]
    #[allow(missing_docs)]
    Default,
    #[allow(missing_docs)]
    High,
    #[allow(missing_docs)]
    VeryHigh,
}

/// Fill Settings › Rotation Adaptation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RotationAdaptation {
    #[default]
    #[allow(missing_docs)]
    None,
    #[allow(missing_docs)]
    Low,
    #[allow(missing_docs)]
    Medium,
    #[allow(missing_docs)]
    High,
    #[allow(missing_docs)]
    Full,
}

/// Output Settings › Output To. Photoshop opens on New Layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FillOutputTo {
    #[serde(rename = "current")]
    #[allow(missing_docs)]
    CurrentLayer,
    #[default]
    #[serde(rename = "new")]
    #[allow(missing_docs)]
    NewLayer,
    #[serde(rename = "duplicate")]
    #[allow(missing_docs)]
    DuplicateLayer,
}

/// A Sampling Brush stroke: a hard round brush that adds to or subtracts from the sampling area.
/// Serialised as PhotoCraft's `samplingStrokes` entries (`{"mode": "add", "size": …, "points":
/// …}`, `caf_ui.rs:216` `stroke_json`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingStroke {
    /// Whether the stroke adds to or subtracts from the sampling area.
    pub mode: EditMode,
    /// Diameter in document pixels.
    pub size: f64,
    /// The brush path in document pixels.
    pub points: Vec<[f64; 2]>,
}

/// The engine's limits, given by the app: PhotoCraft's `MAX_CAF_EDITS` (1000) strokes, and as many
/// lasso edits, and `MAX_CAF_POINTS` (200 000) points in all
/// (`engine/src/cmd/edit_menu_cmds/content_aware.rs:105`, `:107`). The brush stops there, so
/// Apply and the preview never fail on what was painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillLimits {
    /// Most strokes, and separately most lasso edits.
    pub edits: usize,
    /// Most points over all strokes.
    pub points: usize,
}

/// The limit a stroke or edit ran into, with the number it is. The workspace words it
/// ([`FillLimit::message`]) under the panel; the app gets it in [`FillEvent::Refused`].
///
/// [`FillEvent::Refused`]: super::FillEvent::Refused
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FillLimit {
    /// At most this many sampling strokes.
    Strokes(usize),
    /// At most this many lasso edits.
    LassoEdits(usize),
    /// At most this many points in all strokes and lasso edits.
    Points(usize),
}

impl FillLimit {
    /// The limit as a sentence in the installed language ("at most 2 sampling strokes: undo some,
    /// or reset the sampling area"), the count in the locale's digits and plural form.
    pub fn message(&self, ctx: &egui::Context) -> String {
        // pc: crates/ui-egui/src/dialogs/caf_ui/preview.rs:84 `points_limit`, caf_ui.rs:353
        // `begin_stroke`, caf_ui/lifecycle.rs:196 `fill_edit` (the texts).
        let (counted, n) = match *self {
            FillLimit::Strokes(n) => (STROKE_LIMIT, n),
            FillLimit::LassoEdits(n) => (LASSO_EDIT_LIMIT, n),
            FillLimit::Points(n) => (POINT_LIMIT, n),
        };
        let words = Words::of_ctx(ctx);
        words.counted(ctx, counted, &words.get(counted.word), n as u64)
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
    /// How the sampling area starts.
    pub sampling: SamplingMode,
    /// The Sampling Brush strokes, in order.
    pub strokes: Vec<SamplingStroke>,
    /// The fill-area edits, in order.
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

    /// Whether a new stroke may begin: `Err` with the stroke limit or the points limit it ran into.
    pub fn may_begin_stroke(&self, limits: &FillLimits) -> Result<(), FillLimit> {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:353 `begin_stroke` (the checks), :371
        // `extend_stroke`; caf_ui/preview.rs:84 `points_limit`.
        if self.strokes.len() >= limits.edits {
            return Err(FillLimit::Strokes(limits.edits));
        }
        if self.points_used(None) >= limits.points {
            return Err(FillLimit::Points(limits.points));
        }
        Ok(())
    }

    /// Records a finished stroke.
    pub fn add_stroke(&mut self, stroke: SamplingStroke) {
        // pc: crates/ui-egui/src/dialogs/caf_ui.rs:389 `end_stroke`.
        self.strokes.push(stroke);
    }

    /// Records a fill-area edit within `limits`, or returns the limit it ran into. An edit the app
    /// then finds empties the fill area is taken back by the workspace
    /// (`ContentAwareFillWorkspace::refuse_fill_edit`).
    pub fn add_fill_edit(&mut self, edit: FillAreaEdit, limits: &FillLimits) -> Result<(), FillLimit> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:196 `fill_edit` (the count first,
        // then the points; a total equal to the limit passes).
        if self.fill_edits.len() >= limits.edits {
            return Err(FillLimit::LassoEdits(limits.edits));
        }
        if self.points_used(None) + edit.points_len() > limits.points {
            return Err(FillLimit::Points(limits.points));
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
    /// Whether the overlay shows.
    pub show_overlay: bool,
    /// 0..=100.
    pub opacity: f64,
    /// The overlay tint as sRGB bytes.
    pub color: [u8; 3],
    /// The tint marks what is left out (Indicates › Excluded Area), not the sampling area.
    pub excluded: bool,
    /// How the sampling area starts.
    pub sampling: SamplingMode,
    #[allow(missing_docs)]
    pub sample_all_layers: bool,
    #[allow(missing_docs)]
    pub color_adaptation: ColorAdaptation,
    #[allow(missing_docs)]
    pub rotation: RotationAdaptation,
    #[allow(missing_docs)]
    pub scale: bool,
    #[allow(missing_docs)]
    pub mirror: bool,
    #[allow(missing_docs)]
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
    /// How the sampling area starts.
    pub sampling: SamplingMode,
    #[allow(missing_docs)]
    pub sample_all_layers: bool,
    #[allow(missing_docs)]
    pub color_adaptation: ColorAdaptation,
    #[allow(missing_docs)]
    pub rotation_adaptation: RotationAdaptation,
    #[allow(missing_docs)]
    pub scale: bool,
    #[allow(missing_docs)]
    pub mirror: bool,
    #[allow(missing_docs)]
    pub output: FillOutputTo,
    /// The fill-area edits, in order.
    pub fill_edits: Vec<FillAreaEdit>,
    /// The Sampling Brush strokes, in order.
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
