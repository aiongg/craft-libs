//! The channel-mixing adjustments: Black & White, Channel Mixer and Selective Color (task AD-5).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, Context, Id, IdSalt, Ui};

use craft_ui::dialog::ColorRequest;
use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;

use super::{AdjustContext, AdjustLabels};

// ---------------------------------------------------------------------------------------------
// Black & White

/// A Black & White value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlackWhite {
    /// Reds, Yellows, Greens, Cyans, Blues, Magentas: −200..=300 %, whole numbers.
    pub weights: [f64; 6],
    pub tint: bool,
    /// The tint colour; (0.882, 0.827, 0.702) by default.
    pub tint_color: Srgba,
}

impl BlackWhite {
    /// Photoshop's default weights: 40, 60, 40, 60, 20, 80 (PhotoCraft's engine `BW_DEFAULTS`).
    pub const DEFAULT_WEIGHTS: [f64; 6] = [40.0, 60.0, 40.0, 60.0, 20.0, 80.0];
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self { weights: Self::DEFAULT_WEIGHTS, tint: false, tint_color: Srgba::rgb(0.882, 0.827, 0.702) }
    }
}

/// What [`BlackWhiteEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BlackWhiteOutput {
    pub edit: EditResponse<BlackWhite>,
    /// The user clicked the tint swatch: open the colour picker with this request (key
    /// `"blackWhite.tint"`, title "Tint Color") and write the answer into `tint_color`.
    pub pick_color: Option<ColorRequest>,
}

/// Black & White: six colour rows, Tint with its swatch, and Default.
///
/// - Rows Reds … Magentas, −200..300 %, each over a track from black to its colour while dynamic
///   tracks are on: (230, 50, 50), (235, 215, 40), (50, 200, 70), (40, 200, 220), (50, 90, 235),
///   (220, 50, 210). A double-click returns a row to its default weight.
/// - Tint (a `Set`) enables the swatch, which asks for a colour through
///   [`BlackWhiteOutput::pick_color`].
/// - Default restores the six [`BlackWhite::DEFAULT_WEIGHTS`] in one `Set`; tint unchanged.
#[must_use = "show the editor with `show`"]
pub struct BlackWhiteEditor<'a> {
    id_salt: IdSalt,
    value: &'a BlackWhite,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> BlackWhiteEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a BlackWhite, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> BlackWhiteOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:158-192 `black_white` (colours :160,
        // tint colour :178, Default :184-189).
        // new: the tint colour through `ColorRequest`.
        todo!()
    }
}

// ---------------------------------------------------------------------------------------------
// Channel Mixer

/// A Channel Mixer value: per output channel the Red, Green, Blue source weights and the
/// Constant, each −200..=200 %, whole numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelMixer {
    pub red: [f64; 4],
    pub green: [f64; 4],
    pub blue: [f64; 4],
    /// Monochrome's Gray row; `Some` while Monochrome is on.
    pub gray: Option<[f64; 4]>,
}

impl ChannelMixer {
    /// The Gray row Monochrome starts with: [40, 40, 20, 0] (Photoshop).
    pub const MONOCHROME_START: [f64; 4] = [40.0, 40.0, 20.0, 0.0];
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self { red: [100.0, 0.0, 0.0, 0.0], green: [0.0, 100.0, 0.0, 0.0], blue: [0.0, 0.0, 100.0, 0.0], gray: None }
    }
}

/// Channel Mixer: the Output Channel select, three source rows, the Total readout, Constant and
/// Monochrome.
///
/// - Output Channel (Red, Green, Blue; only Gray with Monochrome) is view state under the
///   editor's id, Red at first.
/// - Rows Red, Green, Blue −200..200 % over a track from black to (225, 50, 50), (50, 190, 70),
///   (60, 100, 235) while dynamic tracks are on; Constant −200..200 % with the plain track.
/// - Total: the sum of the three source weights, "+N%" in `type.mono`, `status.warn.fg` above
///   100 % (AccessKit value adds "over 100 percent"). Not editable.
/// - Monochrome on (a `Set`) seeds `gray` with [`ChannelMixer::MONOCHROME_START`] and shows Gray;
///   off removes it.
#[must_use = "show the editor with `show`"]
pub struct ChannelMixerEditor<'a> {
    id_salt: IdSalt,
    value: &'a ChannelMixer,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> ChannelMixerEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a ChannelMixer, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<ChannelMixer> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:243-290 `channel_mixer` (output
        // select :245-255, rows :262-266, Total :267-272, Monochrome :280-288).
        todo!()
    }
}

// ---------------------------------------------------------------------------------------------
// Selective Color

/// How Selective Color applies its percentages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectiveMethod {
    /// Relative to the ink already there; the default.
    #[default]
    Relative,
    Absolute,
}

/// A Selective Color value: for each of Reds, Yellows, Greens, Cyans, Blues, Magentas, Whites,
/// Neutrals, Blacks the Cyan, Magenta, Yellow, Black shifts, −100..=100 %, whole numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectiveColor {
    pub ranges: [[f64; 4]; 9],
    pub method: SelectiveMethod,
}

/// Selective Color: the Colors select (nine ranges), four rows for the shown range, and the
/// Relative / Absolute radios.
///
/// - Colors is view state under the editor's id, Reds at first; rows edit only the shown range.
/// - Rows Cyan, Magenta, Yellow, Black −100..100 % with plain tracks; double-click to 0.
/// - Relative / Absolute (a `RadioSet`) is one `Set`.
#[must_use = "show the editor with `show`"]
pub struct SelectiveColorEditor<'a> {
    id_salt: IdSalt,
    value: &'a SelectiveColor,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> SelectiveColorEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a SelectiveColor, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The editor's id in `ui`, for [`shown_range`](Self::shown_range).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The range the editor with `id` shows, 0 Reds … 8 Blacks.
    pub fn shown_range(ctx: &Context, id: Id) -> usize {
        // pc: crates/ui-egui/src/panels/adjust_ui.rs:38-39 (`selc-range` in memory).
        todo!()
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<SelectiveColor> {
        // pc: crates/ui-egui/src/panels/adjust_ui.rs:17 (the nine ranges), :31-81
        // `selective_color_editor` (select :40-46, rows :50-59, method :61-71).
        todo!()
    }
}
