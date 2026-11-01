//! Photoshop's four small adjustments: Brightness/Contrast, Threshold, Color Balance and Photo
//! Filter (task AD-3).

use craft_theme::{ControlSize, Srgba, TypeRole};
use egui::{AsIdSalt, Context, Id, IdSalt, Stroke, Ui, UiBuilder, pos2, vec2};

use craft_ui::composites::{ChannelColor, Histogram, HistogramData};
use craft_ui::controls::{Checkbox, RadioSet, Select, Slider, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::sliders::{PERCENT, is_ending, pick};
use super::{AdjustContext, AdjustField, AdjustLabels, AdjustSliders, AdjustStyle, AdjustTrack};

/// The edit of a table beside the edit of a check box: the first that ends a gesture, as in
/// [`pick`].
fn either<T>(first: Option<EditEvent<T>>, second: Option<EditEvent<T>>) -> Option<EditEvent<T>> {
    pick(first.into_iter().chain(second).collect(), is_ending)
}

// ---------------------------------------------------------------------------------------------
// Brightness/Contrast

/// Contrast's lowest value, and with Use Legacy.
const CONTRAST_MIN: f64 = -50.0;
const LEGACY_CONTRAST_MIN: f64 = -100.0;

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
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let value = *self.value;
        let lowest = if value.legacy { LEGACY_CONTRAST_MIN } else { CONTRAST_MIN };
        let fields = [
            AdjustField::new("brightness", labels.brightness.as_str(), -150.0..=150.0, 0.0),
            AdjustField::new("contrast", labels.contrast.as_str(), lowest..=100.0, 0.0),
        ];
        let group = Stack::new(Gap::Stack).show(ui, |ui| {
            let sliders = AdjustSliders::new((self.id_salt, "rows"), &fields, &[value.brightness, value.contrast], self.numbers)
                .context(self.context)
                .policy(self.policy)
                .enabled(enabled)
                .show(ui);
            let mut legacy = value.legacy;
            let check = Checkbox::bool(labels.use_legacy.as_str(), &mut legacy).id_salt((self.id_salt, "legacy")).enabled(enabled).show(ui);
            (sliders, check)
        });
        let (sliders, check) = group.inner;
        let (submitted, escaped) = (sliders.submitted, sliders.escaped);
        let sliders_event = sliders.event.map(|e| e.map(|row| BrightnessContrast { brightness: row[0], contrast: row[1], legacy: value.legacy }));
        // Turning Use Legacy off puts a Contrast below the plain range back in it in the same `Set`.
        let check_event = check.event.map(|e| {
            e.map(|legacy| BrightnessContrast { legacy, contrast: if legacy { value.contrast } else { value.contrast.max(CONTRAST_MIN) }, ..value })
        });
        let mut edit = EditResponse::new(group.response, either(sliders_event, check_event));
        edit.submitted = submitted;
        edit.escaped = escaped;
        edit
    }
}

// ---------------------------------------------------------------------------------------------
// Threshold

/// Threshold Level's range and default.
const THRESHOLD_MIN: f64 = 1.0;
const THRESHOLD_MAX: f64 = 255.0;
const THRESHOLD_DEFAULT: f64 = 128.0;

/// The x of the level marker over the histogram `plot`: `level / 255` of its width.
pub(super) fn marker_x(plot: egui::Rect, level: f64) -> f32 {
    plot.left() + (level / THRESHOLD_MAX) as f32 * plot.width()
}

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
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let tokens = Tokens::of(ui);
        let style = AdjustStyle::of(&tokens);
        let level_range = THRESHOLD_MIN..=THRESHOLD_MAX;
        let level = self.level.max(*level_range.start()).min(*level_range.end());
        let fields = [AdjustField::new("level", labels.threshold_level.as_str(), level_range, THRESHOLD_DEFAULT)];
        let group = Stack::new(Gap::Stack).show(ui, |ui| {
            // The plot spans the slider's thumb travel, so the marker stands over the thumb.
            let inset = Slider::style(&tokens, ControlSize::Md).thumb_width / 2.0;
            let width = ui.available_width().min(style.threshold_width);
            let (_, outer) = ui.allocate_space(vec2(width, style.threshold_histogram));
            let plot = outer.shrink2(vec2(inset, 0.0));
            let mut histogram =
                Histogram::new((self.id_salt, "histogram"), labels.histogram.as_str()).bordered(true).height(style.threshold_histogram);
            if let Some(counts) = self.histogram {
                histogram = histogram.data(HistogramData::tone(counts, ChannelColor::Neutral));
            }
            let shown = ui.scope_builder(UiBuilder::new().max_rect(plot), |ui| histogram.show(ui)).inner;
            let x = marker_x(shown.response.rect, level);
            let marker = Stroke::new(Histogram::style(&tokens).marker_width, style.threshold_marker);
            ui.painter().line_segment([pos2(x, shown.response.rect.top()), pos2(x, shown.response.rect.bottom())], marker);
            AdjustSliders::new((self.id_salt, "rows"), &fields, &[level], self.numbers)
                .context(self.context)
                .policy(self.policy)
                .enabled(enabled)
                .show(ui)
        });
        let rows = group.inner;
        let (submitted, escaped) = (rows.submitted, rows.escaped);
        let mut edit = EditResponse::new(group.response, rows.event.map(|e| e.map(|row| row[0])));
        edit.submitted = submitted;
        edit.escaped = escaped;
        edit
    }
}

// ---------------------------------------------------------------------------------------------
// Color Balance

/// Where an editor keeps its Tone choice, under its id.
const TONE: &str = "tone";

/// An opaque colour from 8-bit sRGB channels.
fn rgb8(r: u8, g: u8, b: u8) -> Srgba {
    Srgba::rgb(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0)
}

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
        self.shadows = [0.0; 3];
        self.midtones = [0.0; 3];
        self.highlights = [0.0; 3];
    }

    /// The shifts of `tone`.
    fn of_tone(&self, tone: ColorBalanceTone) -> [f64; 3] {
        match tone {
            ColorBalanceTone::Shadows => self.shadows,
            ColorBalanceTone::Midtones => self.midtones,
            ColorBalanceTone::Highlights => self.highlights,
        }
    }

    /// The value with the shifts of `tone` replaced.
    fn with_tone(mut self, tone: ColorBalanceTone, shifts: [f64; 3]) -> Self {
        match tone {
            ColorBalanceTone::Shadows => self.shadows = shifts,
            ColorBalanceTone::Midtones => self.midtones = shifts,
            ColorBalanceTone::Highlights => self.highlights = shifts,
        }
        self
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
        ctx.data(|d| d.get_temp::<ColorBalanceTone>(id.with(TONE))).unwrap_or_default()
    }

    /// Shows Midtones again (with [`ColorBalance::reset`]).
    pub fn reset_tone(ctx: &Context, id: Id) {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:97-105 `reset_color_balance`.
        ctx.data_mut(|d| d.insert_temp(id.with(TONE), ColorBalanceTone::Midtones));
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<ColorBalance> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:115-153 `color_balance` (tracks
        // :131-133, reset 0 and wheel step 1 in a dialog :137).
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        // `id_in` wraps its salt once; `self.id_salt` is wrapped already.
        let id = ui.make_persistent_id(self.id_salt);
        let value = *self.value;
        let mut tone = Self::tone(ui.ctx(), id);
        let cyan_red = [rgb8(0, 190, 210), rgb8(225, 40, 40)];
        let magenta_green = [rgb8(210, 40, 190), rgb8(40, 190, 60)];
        let yellow_blue = [rgb8(230, 210, 30), rgb8(40, 80, 230)];
        let rows = [
            ("cyanRed", format!("{} · {}", labels.cyan, labels.red), cyan_red),
            ("magentaGreen", format!("{} · {}", labels.magenta, labels.green), magenta_green),
            ("yellowBlue", format!("{} · {}", labels.yellow, labels.blue), yellow_blue),
        ];
        let fields: Vec<AdjustField> = rows
            .into_iter()
            .map(|(key, label, stops)| AdjustField::new(key, label, -100.0..=100.0, 0.0).track(AdjustTrack::Colors(stops.to_vec())))
            .collect();
        let group = Stack::new(Gap::Stack).show(ui, |ui| {
            direction::horizontal(ui, |ui| {
                ui.add(Text::new(labels.tone.as_str()).role(TypeRole::Label).content(Content::Muted));
                let mut set = RadioSet::new((self.id_salt, "tone"), labels.tone.as_str(), &mut tone)
                    .item(ColorBalanceTone::Shadows)
                    .item(ColorBalanceTone::Midtones)
                    .item(ColorBalanceTone::Highlights)
                    .enabled(enabled)
                    .begin(ui);
                set.radio(ui, 0, &labels.shadows);
                set.radio(ui, 1, &labels.midtones);
                set.radio(ui, 2, &labels.highlights);
                // The tone is view state: the set's `Set` is not an edit.
                let _ = set.end(ui);
            });
            let sliders = AdjustSliders::new((self.id_salt, "rows", tone), &fields, &value.of_tone(tone), self.numbers)
                .context(self.context)
                .policy(self.policy)
                .enabled(enabled)
                .show(ui);
            let mut preserve = value.preserve_luminosity;
            let check =
                Checkbox::bool(labels.preserve_luminosity.as_str(), &mut preserve).id_salt((self.id_salt, "preserve")).enabled(enabled).show(ui);
            (sliders, check)
        });
        ui.data_mut(|d| d.insert_temp(id.with(TONE), tone));
        let (sliders, check) = group.inner;
        let (submitted, escaped) = (sliders.submitted, sliders.escaped);
        let shifted = sliders.event.map(|e| e.map(|row| value.with_tone(tone, [row[0], row[1], row[2]])));
        let preserved = check.event.map(|e| e.map(|preserve_luminosity| ColorBalance { preserve_luminosity, ..value }));
        let mut edit = EditResponse::new(group.response, either(shifted, preserved));
        edit.submitted = submitted;
        edit.escaped = escaped;
        edit
    }
}

// ---------------------------------------------------------------------------------------------
// Photo Filter

/// Where Photo Filter keeps its Filter or Color choice, under the editor's id.
const MODE: &str = "mode";

/// The colour request's key.
const COLOR_KEY: &str = "photoFilter.color";

/// Density's default.
const DENSITY_DEFAULT: f64 = 25.0;

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
        let (r, g, b) = match self {
            PhotoFilterPreset::Warming85 => (236, 138, 0),
            PhotoFilterPreset::WarmingLba => (250, 150, 0),
            PhotoFilterPreset::Warming81 => (235, 177, 19),
            PhotoFilterPreset::Cooling80 => (0, 109, 255),
            PhotoFilterPreset::CoolingLbb => (0, 93, 255),
            PhotoFilterPreset::Cooling82 => (0, 181, 255),
            PhotoFilterPreset::Red => (234, 26, 26),
            PhotoFilterPreset::Orange => (243, 132, 23),
            PhotoFilterPreset::Yellow => (249, 227, 28),
            PhotoFilterPreset::Green => (25, 201, 25),
            PhotoFilterPreset::Cyan => (29, 203, 234),
            PhotoFilterPreset::Blue => (29, 53, 234),
            PhotoFilterPreset::Violet => (155, 29, 234),
            PhotoFilterPreset::Magenta => (227, 24, 227),
            PhotoFilterPreset::Sepia => (172, 122, 51),
            PhotoFilterPreset::DeepRed => (255, 0, 0),
            PhotoFilterPreset::DeepBlue => (0, 34, 205),
            PhotoFilterPreset::DeepEmerald => (0, 140, 0),
            PhotoFilterPreset::DeepYellow => (255, 213, 0),
            PhotoFilterPreset::Underwater => (0, 194, 177),
        };
        rgb8(r, g, b)
    }

    /// The preset's menu name from `labels` (the channel words for Red, Yellow, Green, Cyan, Blue
    /// and Magenta).
    pub fn label(self, labels: &AdjustLabels) -> &str {
        // pc: crates/engine/src/adjust_params.rs:26-47 `PHOTO_FILTERS` (labels).
        match self {
            PhotoFilterPreset::Warming85 => &labels.warming_85,
            PhotoFilterPreset::WarmingLba => &labels.warming_lba,
            PhotoFilterPreset::Warming81 => &labels.warming_81,
            PhotoFilterPreset::Cooling80 => &labels.cooling_80,
            PhotoFilterPreset::CoolingLbb => &labels.cooling_lbb,
            PhotoFilterPreset::Cooling82 => &labels.cooling_82,
            PhotoFilterPreset::Red => &labels.red,
            PhotoFilterPreset::Orange => &labels.orange,
            PhotoFilterPreset::Yellow => &labels.yellow,
            PhotoFilterPreset::Green => &labels.green,
            PhotoFilterPreset::Cyan => &labels.cyan,
            PhotoFilterPreset::Blue => &labels.blue,
            PhotoFilterPreset::Violet => &labels.violet,
            PhotoFilterPreset::Magenta => &labels.magenta,
            PhotoFilterPreset::Sepia => &labels.sepia,
            PhotoFilterPreset::DeepRed => &labels.deep_red,
            PhotoFilterPreset::DeepBlue => &labels.deep_blue,
            PhotoFilterPreset::DeepEmerald => &labels.deep_emerald,
            PhotoFilterPreset::DeepYellow => &labels.deep_yellow,
            PhotoFilterPreset::Underwater => &labels.underwater,
        }
    }

    /// The preset whose colour is `color` in 8-bit sRGB, if any (how the editor tells a preset
    /// from a custom colour).
    pub fn of(color: Srgba) -> Option<PhotoFilterPreset> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:201 (found by hex).
        PhotoFilterPreset::ALL.into_iter().find(|preset| preset.color().same_rgb8(color))
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
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let value = *self.value;
        let preset = PhotoFilterPreset::of(value.color);
        // Filter (false) or Color (true): view state; the colour of no preset starts on Color.
        let mut use_color = ui.data(|d| d.get_temp::<bool>(id.with(MODE))).unwrap_or(preset.is_none());
        let density = [AdjustField::new("density", labels.density.as_str(), 0.0..=100.0, DENSITY_DEFAULT).unit(Unit::new(PERCENT))];
        let mut pick_color = None;
        let group = Stack::new(Gap::Stack).show(ui, |ui| {
            let mut set =
                RadioSet::new((self.id_salt, "mode"), labels.filter.as_str(), &mut use_color).item(false).item(true).enabled(enabled).begin(ui);
            // The radio's change is already in `use_color` for the rows below.
            let color_mode = use_color;
            let mut chosen = preset;
            let mut picked = None;
            direction::horizontal(ui, |ui| {
                set.radio(ui, 0, &labels.filter);
                let mut select = Select::new((self.id_salt, "preset"), labels.filter.as_str(), &mut chosen).fit(true);
                if preset.is_none() {
                    select = select.option(None, labels.custom.as_str());
                }
                for entry in PhotoFilterPreset::ALL {
                    select = select.option(Some(entry), entry.label(&labels));
                }
                picked = Some(select.enabled(enabled && !color_mode).show(ui));
            });
            direction::horizontal(ui, |ui| {
                set.radio(ui, 1, &labels.color);
                let swatch =
                    Swatch::new((self.id_salt, "swatch"), labels.filter_color.as_str(), value.color.to_rgba()).enabled(enabled && color_mode);
                if ui.add(swatch).clicked() {
                    pick_color = Some(ColorRequest::new(COLOR_KEY, value.color).title(labels.filter_color.as_str()));
                }
            });
            let _ = set.end(ui);
            let sliders = AdjustSliders::new((self.id_salt, "density"), &density, &[value.density], self.numbers)
                .context(self.context)
                .policy(self.policy)
                .enabled(enabled)
                .show(ui);
            let mut preserve = value.preserve_luminosity;
            let check =
                Checkbox::bool(labels.preserve_luminosity.as_str(), &mut preserve).id_salt((self.id_salt, "preserve")).enabled(enabled).show(ui);
            (picked, sliders, check)
        });
        ui.data_mut(|d| d.insert_temp(id.with(MODE), use_color));
        let (picked, sliders, check) = group.inner;
        let (submitted, escaped) = (sliders.submitted, sliders.escaped);
        let preset_event =
            picked.and_then(|p| p.event).map(|e| e.map(|entry| PhotoFilter { color: entry.map_or(value.color, PhotoFilterPreset::color), ..value }));
        let density_event = sliders.event.map(|e| e.map(|row| PhotoFilter { density: row[0], ..value }));
        let preserve_event = check.event.map(|e| e.map(|preserve_luminosity| PhotoFilter { preserve_luminosity, ..value }));
        let event = pick([preset_event, density_event, preserve_event].into_iter().flatten().collect(), is_ending);
        let mut edit = EditResponse::new(group.response, event);
        edit.submitted = submitted;
        edit.escaped = escaped;
        PhotoFilterOutput { edit, pick_color }
    }
}
