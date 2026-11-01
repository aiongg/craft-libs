//! [`CurvesPanel`]: the Curves adjustment around the kernel's [`CurveEditor`]: the Channel select,
//! the mode icons (points, pencil), Smooth, the point pickers, the other channels overlaid on the
//! composite and the hint line; and the ⌘/Ctrl-click-on-the-image helper
//! ([`CurvesPanel::add_point_from_sample`]) (task AD-2).
//!
//! [`CurveEditor`]: craft_ui::composites::CurveEditor

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, Context, Id, IdSalt, Ui};

use craft_ui::composites::{Curve, CurveLabels, Sampler};
use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;

use super::{AdjustContext, AdjustLabels, ToneChannels, TonePicker};

/// How the shown curve is edited: Photoshop's two buttons above the graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CurvesMode {
    /// Edit points to modify the curve (`Icon::Spline`); every channel is a [`Curve::Points`].
    #[default]
    Points,
    /// Draw to modify the curve (`Icon::Pencil`); every channel is a [`Curve::Map`].
    Pencil,
}

/// A Curves value: one curve per channel of the panel's [`ToneChannels`] (same index), all in
/// one [`CurvesMode`].
#[derive(Clone, Debug, PartialEq)]
pub struct ToneCurves {
    pub curves: Vec<Curve>,
    pub mode: CurvesMode,
}

impl ToneCurves {
    /// Identity point curves for `channels`, 0..=255.
    pub fn identity(channels: &ToneChannels) -> Self {
        Self { curves: vec![Curve::identity(255.0); channels.len()], mode: CurvesMode::Points }
    }
}

/// What [`CurvesPanel::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct CurvesPanelOutput {
    /// The shown channel's curve edit: the editor's gestures, and Smooth as one `Set` of the
    /// smoothed map. The response is the panel's group.
    pub edit: EditResponse<Curve>,
    /// The channel the edit is for, and the one shown (an index into the panel's
    /// [`ToneChannels`]); the panel keeps it between passes.
    pub channel: usize,
    /// The user clicked the other mode's icon. The host converts every channel with its engine
    /// (draws the point curves into maps, or fits points to the drawn maps, PhotoCraft
    /// `set_pencil`) and passes the new value back; the panel changes nothing itself.
    pub mode: Option<CurvesMode>,
    /// The point picker the user armed (`Some(Some(p))`) or disarmed (`Some(None)`) this pass.
    pub picker: Option<Option<TonePicker>>,
}

/// The Curves adjustment: Image › Adjustments › Curves and a Curves layer's Properties panel.
///
/// Top to bottom:
/// 1. The Channel select ([`ToneChannels`]); an [isolated](ToneChannels::isolated) channel is
///    the only entry, its name shown exactly as the host gave it.
/// 2. The mode row: two icon buttons (`Icon::Spline` "Edit points to modify the curve",
///    `Icon::Pencil` "Draw to modify the curve", 24 pt, the current one selected) and Smooth
///    (a secondary button, enabled in pencil mode only, tooltip "Smooth the curve values").
/// 3. The [`CurveEditor`](craft_ui::composites::CurveEditor) for the shown channel: in its
///    colour, over its histogram, `.pencil(true)` in pencil mode, and, on a composite only, the
///    other channels' curves as overlays (Photoshop; none for an isolated channel).
/// 4. The hint line (`type.caption`, `text.faint`): points mode "Click to add a point · drag off
///    or {⌘}-click or Delete to remove", pencil mode "Drag to draw the curve · {Shift}-click
///    draws a straight line", the key names from the platform's shortcut words.
/// 5. With [`pickers`](Self::pickers), the point picker row.
///
/// - **Smooth** replaces the shown map with its moving average over 7 levels (radius 3, shrunk
///   at the ends so the first and last levels keep their values: PhotoCraft's
///   `smooth_curve_map`), as one `Set`.
/// - **Mode icons** report [`CurvesPanelOutput::mode`]; the current mode's icon reports nothing.
/// - **Keyboard**: Tab visits the Channel select, the two mode icons, Smooth, the editor and the
///   pickers. The editor's own keys (Insert, ⌘/Ctrl+Enter, arrows) are unchanged.
/// - **AccessKit**: a `Group` named "Curves"; the mode icons are toggle `Button`s.
#[must_use = "show the panel with `show`"]
pub struct CurvesPanel<'a> {
    id_salt: IdSalt,
    curves: &'a ToneCurves,
    channels: ToneChannels,
    numbers: &'a LocaleNumbers,
    histograms: &'a [[u32; 256]],
    sampler: Option<&'a Sampler>,
    picker: Option<TonePicker>,
    pickers: bool,
    picker_enabled: [bool; 3],
    unavailable: bool,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    curve_labels: Option<CurveLabels>,
    enabled: bool,
}

impl<'a> CurvesPanel<'a> {
    /// The panel over `curves`, one curve per channel of `channels`.
    pub fn new(id_salt: impl AsIdSalt, curves: &'a ToneCurves, channels: ToneChannels, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            curves,
            channels,
            numbers,
            histograms: &[],
            sampler: None,
            picker: None,
            pickers: true,
            picker_enabled: [true; 3],
            unavailable: false,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            curve_labels: None,
            enabled: true,
        }
    }

    /// The host's histograms, one per channel (same index), 256 bins each.
    pub fn histograms(mut self, histograms: &'a [[u32; 256]]) -> Self {
        self.histograms = histograms;
        self
    }

    /// The host's interpolation of point curves ([`CurveEditor::sampler`](craft_ui::composites::CurveEditor::sampler)).
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The armed point picker, shown selected.
    pub fn picker(mut self, armed: Option<TonePicker>) -> Self {
        self.picker = armed;
        self
    }

    /// Whether the point picker row shows; on by default.
    pub fn pickers(mut self, on: bool) -> Self {
        self.pickers = on;
        self
    }

    /// Enables or disables one picker.
    pub fn picker_enabled(mut self, picker: TonePicker, enabled: bool) -> Self {
        self.picker_enabled[picker as usize] = enabled;
        self
    }

    /// The document's mode has no point pickers: all disabled, with the note.
    pub fn pickers_unavailable(mut self, unavailable: bool) -> Self {
        self.unavailable = unavailable;
        self
    }

    /// Dialog or Properties panel; a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// The editor's edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The panel's words; the installed ones by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The curve editor's own words.
    pub fn curve_labels(mut self, labels: CurveLabels) -> Self {
        self.curve_labels = Some(labels);
        self
    }

    /// Enables or disables the whole panel.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The panel's id in `ui`, for [`shown_channel`](Self::shown_channel) and
    /// [`select_point`](Self::select_point).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The channel the panel with `id` shows (0 before it was first drawn).
    pub fn shown_channel(ctx: &Context, id: Id) -> usize {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:50-55 (the choice in egui memory).
        todo!()
    }

    /// Selects point `index` of the shown curve in the panel with `id`, as a click on it would
    /// (after [`add_point_from_sample`](Self::add_point_from_sample)); through
    /// [`CurveEditor::select`](craft_ui::composites::CurveEditor::select) with the inner editor's id.
    pub fn select_point(ctx: &Context, id: Id, index: usize) {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:153-161 (the new point selected).
        todo!()
    }

    /// ⌘/Ctrl-click on the image while Curves is open: adds a point to the shown channel's curve
    /// at the sampled tone, on the curve (its output there), and returns its index for
    /// [`select_point`](Self::select_point). With `every_channel` (Shift) it adds one to each of
    /// Red, Green and Blue instead, at that channel's value, and returns the shown channel's
    /// point if the shown channel is one of them.
    ///
    /// - The tone, 0..=255 rounded to a whole level: Red, Green, Blue read their channel; the
    ///   composite reads Rec. 709 luma (0.2126 R + 0.7152 G + 0.0722 B); a grey document the mean
    ///   of R, G, B.
    /// - The output is the curve's at that input (through `sampler`, the host's interpolation),
    ///   rounded; the point goes in through [`CurveRules::insert`](craft_ui::composites::CurveRules::insert),
    ///   so a point already at that input is reused and the 16-point limit holds.
    /// - Does nothing and returns `None` in pencil mode, for Lab and CMYK channels, for an
    ///   isolated channel (a mask holds its own grey levels) and when `sample` is off the image
    ///   (the host doesn't call it then). `every_channel` is ignored for a grey document.
    pub fn add_point_from_sample(
        curves: &mut ToneCurves,
        channels: &ToneChannels,
        shown: usize,
        sample: Srgba,
        every_channel: bool,
        sampler: &Sampler,
    ) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:95-175 `add_point_at` (pencil, mode, isolated
        // checks; the tone per channel; the output from the curve's table, rounded; insert).
        todo!()
    }

    /// Draws the panel and reports the shown channel's edit.
    pub fn show(self, ui: &mut Ui) -> CurvesPanelOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:225-249 `curves_mode_row` (icons 24,
        // Smooth enabled in pencil mode), :251-272 (Channel select, isolated channel), :343-350
        // (overlays on the composite only), :397-402 (hint, 11 pt `text_faint`); the smoothing is
        // engine `adjust_params.rs:548-560` `smooth_curve_map` (radius 3).
        // lib: `CurveEditor::new(..).channels(..).channel(..).pencil(..).overlays(..).histogram(..)`.
        todo!()
    }
}
