//! Photoshop's four small adjustments: Brightness/Contrast, Threshold, Color Balance and Photo
//! Filter (task AD-3).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, Context, Id, IdSalt, Ui};

use craft_ui::dialog::ColorRequest;
use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;

use super::{AdjustContext, AdjustLabels};

// ---------------------------------------------------------------------------------------------
// Brightness/Contrast

/// A Brightness/Contrast value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrightnessContrast {
    /// −150..=150, whole numbers.
    pub brightness: f64,
    /// −50..=100, or −100..=100 with `legacy`; whole numbers.
    pub contrast: f64,
    /// Use Legacy.
    pub legacy: bool,
}

/// Brightness/Contrast: two slider rows and Use Legacy.
///
/// - Brightness −150..150, Contrast −50..100 (−100..100 with Use Legacy), both default 0 on a
///   double-click, whole numbers.
/// - Turning Use Legacy off clamps Contrast to at least −50 in the same `Set`.
/// - Keyboard and AccessKit as [`AdjustSliders`](super::AdjustSliders); the check box is a Tab stop.
#[must_use = "show the editor with `show`"]
pub struct BrightnessContrastEditor<'a> {
    id_salt: IdSalt,
    value: &'a BrightnessContrast,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> BrightnessContrastEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a BrightnessContrast, numbers: &'a LocaleNumbers) -> Self {
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

    /// Dialog or Properties panel; a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// The sliders' edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<BrightnessContrast> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:53-66 `brightness_contrast`.
        todo!()
    }
}

// ---------------------------------------------------------------------------------------------
// Threshold

/// Threshold: the image's histogram with a marker at the level, then the Threshold Level row
/// (1..=255, default 128, whole numbers).
///
/// - The histogram is `adjust.threshold_histogram` (90) high and at most
///   `adjust.threshold_width` (300) wide, inset 6 at each side, on `field` with a 1 pt
///   `field.border` frame; the marker is a 1 pt vertical line at `level / 255` in `accent.strong`.
/// - The histogram is the kernel's [`Histogram`](craft_ui::composites::Histogram) (the
///   composite's bins, neutral colour); AccessKit names it "Histogram".
/// - The row behaves as an [`AdjustSliders`](super::AdjustSliders) row.
#[must_use = "show the editor with `show`"]
pub struct ThresholdEditor<'a> {
    id_salt: IdSalt,
    level: f64,
    numbers: &'a LocaleNumbers,
    histogram: Option<&'a [u32; 256]>,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> ThresholdEditor<'a> {
    /// The editor at `level` (1..=255).
    pub fn new(id_salt: impl AsIdSalt, level: f64, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            level,
            numbers,
            histogram: None,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    /// The composite's histogram, 256 bins; an empty plot without it.
    pub fn histogram(mut self, counts: &'a [u32; 256]) -> Self {
        self.histogram = Some(counts);
        self
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

    /// Draws the editor; the edit carries the level.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:71-87 `threshold` (300 × 90, inset 6,
        // marker 1 pt accent, frame 1 pt `field_border`, the row 1..255 default 128).
        todo!()
    }
}

// ---------------------------------------------------------------------------------------------
// Color Balance

/// The tone Color Balance shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorBalanceTone {
    Shadows,
    /// The default, and where a reset returns.
    #[default]
    Midtones,
    Highlights,
}

/// A Color Balance value: per tone the Cyan–Red, Magenta–Green and Yellow–Blue shifts,
/// −100..=100, whole numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorBalance {
    pub shadows: [f64; 3],
    pub midtones: [f64; 3],
    pub highlights: [f64; 3],
    /// Preserve Luminosity; on by default.
    pub preserve_luminosity: bool,
}

impl Default for ColorBalance {
    fn default() -> Self {
        Self { shadows: [0.0; 3], midtones: [0.0; 3], highlights: [0.0; 3], preserve_luminosity: true }
    }
}

impl ColorBalance {
    /// Photoshop's reset to defaults (Alt+Cancel; the Properties panel's Reset once the layer is
    /// as the panel found it): every tone back to 0, Preserve Luminosity kept. The editor's tone
    /// returns to Midtones through [`ColorBalanceEditor::reset_tone`].
    pub fn reset(&mut self) {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:97-105 `reset_color_balance`.
        todo!()
    }
}

/// Color Balance: the Tone radios, three two-colour slider rows for the shown tone, and Preserve
/// Luminosity.
///
/// - The Tone radios (Shadows, Midtones, Highlights; a `RadioSet`) pick the tone shown; the
///   choice is the editor's view state under its id, Midtones at first, and survives value
///   changes. Not an edit.
/// - Rows "Cyan · Red", "Magenta · Green", "Yellow · Blue" (the two channel words joined with
///   " · "), −100..100, with two-colour tracks while dynamic tracks are on: cyan (0, 190, 210) to
///   red (225, 40, 40), magenta (210, 40, 190) to green (40, 190, 60), yellow (230, 210, 30) to
///   blue (40, 80, 230). A double-click zeroes only that row of the shown tone. The wheel steps
///   by 1 in a dialog.
/// - Preserve Luminosity is one `Set`.
/// - AccessKit: the radios a `RadioGroup` named "Tone".
#[must_use = "show the editor with `show`"]
pub struct ColorBalanceEditor<'a> {
    id_salt: IdSalt,
    value: &'a ColorBalance,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> ColorBalanceEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a ColorBalance, numbers: &'a LocaleNumbers) -> Self {
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

    /// The editor's id in `ui`, for [`tone`](Self::tone) and [`reset_tone`](Self::reset_tone).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The tone the editor with `id` shows.
    pub fn tone(ctx: &Context, id: Id) -> ColorBalanceTone {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:92-94 `color_balance_tone_id`.
        todo!()
    }

    /// Shows Midtones again (with [`ColorBalance::reset`]).
    pub fn reset_tone(ctx: &Context, id: Id) {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:97-105 `reset_color_balance`.
        todo!()
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<ColorBalance> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:115-153 `color_balance` (tracks
        // :131-133, reset 0 and wheel step 1 in a dialog :137).
        todo!()
    }
}

// ---------------------------------------------------------------------------------------------
// Photo Filter

/// Photo Filter's presets, in Photoshop's menu order, with their colours (PhotoCraft's engine
/// table `PHOTO_FILTERS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PhotoFilterPreset {
    /// Warming Filter (85): (236, 138, 0), the default.
    Warming85,
    /// Warming Filter (LBA): (250, 150, 0).
    WarmingLba,
    /// Warming Filter (81): (235, 177, 19).
    Warming81,
    /// Cooling Filter (80): (0, 109, 255).
    Cooling80,
    /// Cooling Filter (LBB): (0, 93, 255).
    CoolingLbb,
    /// Cooling Filter (82): (0, 181, 255).
    Cooling82,
    /// Red: (234, 26, 26).
    Red,
    /// Orange: (243, 132, 23).
    Orange,
    /// Yellow: (249, 227, 28).
    Yellow,
    /// Green: (25, 201, 25).
    Green,
    /// Cyan: (29, 203, 234).
    Cyan,
    /// Blue: (29, 53, 234).
    Blue,
    /// Violet: (155, 29, 234).
    Violet,
    /// Magenta: (227, 24, 227).
    Magenta,
    /// Sepia: (172, 122, 51).
    Sepia,
    /// Deep Red: (255, 0, 0).
    DeepRed,
    /// Deep Blue: (0, 34, 205).
    DeepBlue,
    /// Deep Emerald: (0, 140, 0).
    DeepEmerald,
    /// Deep Yellow: (255, 213, 0).
    DeepYellow,
    /// Underwater: (0, 194, 177).
    Underwater,
}

impl PhotoFilterPreset {
    /// Every preset, in menu order.
    pub const ALL: [PhotoFilterPreset; 20] = [
        PhotoFilterPreset::Warming85,
        PhotoFilterPreset::WarmingLba,
        PhotoFilterPreset::Warming81,
        PhotoFilterPreset::Cooling80,
        PhotoFilterPreset::CoolingLbb,
        PhotoFilterPreset::Cooling82,
        PhotoFilterPreset::Red,
        PhotoFilterPreset::Orange,
        PhotoFilterPreset::Yellow,
        PhotoFilterPreset::Green,
        PhotoFilterPreset::Cyan,
        PhotoFilterPreset::Blue,
        PhotoFilterPreset::Violet,
        PhotoFilterPreset::Magenta,
        PhotoFilterPreset::Sepia,
        PhotoFilterPreset::DeepRed,
        PhotoFilterPreset::DeepBlue,
        PhotoFilterPreset::DeepEmerald,
        PhotoFilterPreset::DeepYellow,
        PhotoFilterPreset::Underwater,
    ];

    /// The preset's colour, as in each variant's doc (8-bit sRGB).
    pub fn color(self) -> Srgba {
        // pc: crates/engine/src/adjust_params.rs:26-47 `PHOTO_FILTERS`.
        todo!()
    }

    /// The preset's menu name from `labels` (the channel words for Red, Yellow, Green, Cyan, Blue
    /// and Magenta).
    pub fn label(self, labels: &AdjustLabels) -> &str {
        // pc: crates/engine/src/adjust_params.rs:26-47 `PHOTO_FILTERS` (labels).
        todo!()
    }

    /// The preset whose colour is `color` in 8-bit sRGB, if any (how the editor tells a preset
    /// from a custom colour).
    pub fn of(color: Srgba) -> Option<PhotoFilterPreset> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:201 (found by hex).
        todo!()
    }
}

/// A Photo Filter value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoFilter {
    /// The filter colour (a preset's or the user's).
    pub color: Srgba,
    /// Density 0..=100 %, default 25.
    pub density: f64,
    /// Preserve Luminosity; on by default.
    pub preserve_luminosity: bool,
}

/// What [`PhotoFilterEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct PhotoFilterOutput {
    pub edit: EditResponse<PhotoFilter>,
    /// The user clicked the Color swatch: open the colour picker with this request (key
    /// `"photoFilter.color"`, title "Filter Color") and write the answer into the value's
    /// `color` (an outside change the editor shows next pass).
    pub pick_color: Option<ColorRequest>,
}

/// Photo Filter: the Filter radio with the preset select, the Color radio with a swatch, Density
/// and Preserve Luminosity.
///
/// - Filter or Color: the radio is view state (egui memory), Color at first when the value's
///   colour is no preset ([`PhotoFilterPreset::of`]). The other row's control is disabled.
/// - The preset select lists [`PhotoFilterPreset::ALL`], with "Custom" first while the colour is
///   no preset; picking one is a `Set` of its colour.
/// - The swatch (`Swatch`, a Tab stop) asks for a colour through
///   [`PhotoFilterOutput::pick_color`].
/// - Density 0..100 %, default 25 on a double-click; Preserve Luminosity a `Set`.
#[must_use = "show the editor with `show`"]
pub struct PhotoFilterEditor<'a> {
    id_salt: IdSalt,
    value: &'a PhotoFilter,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> PhotoFilterEditor<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a PhotoFilter, numbers: &'a LocaleNumbers) -> Self {
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

    pub fn show(self, ui: &mut Ui) -> PhotoFilterOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:197-238 `photo_filter` (radio state
        // `pf-mode`, Custom first, density 25, Preserve Luminosity).
        // new: the colour through `ColorRequest` (PhotoCraft opens egui's picker in place).
        todo!()
    }
}
