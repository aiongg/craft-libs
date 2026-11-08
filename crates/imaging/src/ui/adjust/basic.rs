//! Photoshop's four small adjustments: Brightness/Contrast, Threshold, Color Balance and Photo Filter.

#[cfg(feature = "screen-testing")]
mod stories;

use craft_theme::Srgba;
use craft_ui::choice::{Checkbox, Radio, RadioGroup, Swatch};
use craft_ui::contract::{Bind, EditResponse, Entry, Outcome, Pattern, Props as _, Remember as _, Show as _, ShowContainer as _};
use craft_ui::form::{Field, FieldGroup};
use craft_ui::layout_components::Row;
use craft_ui::pickers::Select;
use craft_ui::runtime::env::{Label, UnitKind};
use craft_ui::runtime::requests::ColorRequest;
use egui::Ui;

use super::sliders::{Run, lifted, read, words};
use super::{AdjustContext, AdjustField, AdjustWords};
use crate::ui::histogram::{ChannelColor, Histogram, HistogramData};

/// An opaque colour from 8-bit sRGB channels.
fn srgb8(r: u8, g: u8, b: u8) -> Srgba {
    Srgba::rgb(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0)
}

// ---------------------------------------------------------------------------------------------
// Brightness/Contrast

/// Contrast's lowest value, and with Use Legacy.
const CONTRAST_MIN: f64 = -50.0;
const LEGACY_CONTRAST_MIN: f64 = -100.0;

/// A Brightness/Contrast value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct BrightnessContrast {
    /// −150..=150, whole numbers.
    pub brightness: f64,
    /// −50..=100, or −100..=100 with `legacy`; whole numbers.
    pub contrast: f64,
    /// Use Legacy.
    pub legacy: bool,
}

impl BrightnessContrast {
    /// A value of `brightness`, `contrast` and Use Legacy.
    pub fn new(brightness: f64, contrast: f64, legacy: bool) -> Self {
        Self { brightness, contrast, legacy }
    }
}

/// What a Brightness/Contrast editor keeps between passes: the value its running gesture began with.
#[derive(Clone, Default)]
struct BrightnessMemory {
    began: Option<BrightnessContrast>,
}

/// Brightness/Contrast: two slider rows and Use Legacy.
///
/// - Brightness −150..150, Contrast −50..100 (−100..100 with Use Legacy), both default 0 on a reset, whole numbers.
/// - Turning Use Legacy off clamps Contrast to at least −50 in the same `Set`.
/// - Keyboard and AccessKit as [`AdjustSliders`](super::AdjustSliders); the check box is a Tab stop.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct BrightnessContrastEditor<'a> {
    label: Label,
    value: Bind<'a, BrightnessContrast>,
    context: AdjustContext,
}

impl<'a> BrightnessContrastEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, BrightnessContrast>) -> Self {
        Self { label: label.into(), value, context: AdjustContext::dialog(true) }
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit of the `BrightnessContrast`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<BrightnessContrast> {
        let BrightnessContrastEditor { label, mut value, context } = self;
        let words = words(ui);
        let mut edit = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let lowest = if now.legacy { LEGACY_CONTRAST_MIN } else { CONTRAST_MIN };
            let brightness = AdjustField::new("brightness", words.brightness.clone(), -150.0..=150.0, 0.0);
            let contrast = AdjustField::new("contrast", words.contrast.clone(), lowest..=100.0, 0.0);
            let mut run = Run::start(&mut value, context, now, cx.memory::<BrightnessMemory>().began);
            run.slider_row(ui, cx, &brightness, now.brightness, |now, x| BrightnessContrast { brightness: x, ..*now });
            run.slider_row(ui, cx, &contrast, now.contrast, |now, x| BrightnessContrast { contrast: x, ..*now });
            let legacy = Checkbox::new(Bind::given(now.legacy)).label(words.use_legacy.clone()).show(ui);
            // Turning Use Legacy off puts a Contrast below the plain range back in it in the same `Set`.
            run.push(cx, legacy, |now, legacy| BrightnessContrast {
                legacy: *legacy,
                contrast: if *legacy { now.contrast } else { now.contrast.max(CONTRAST_MIN) },
                ..*now
            });
            cx.memory::<BrightnessMemory>().began = run.began();
            edit = Some(run.finish());
        });
        lifted(edit, shown.output)
    }
}

// ---------------------------------------------------------------------------------------------
// Threshold

/// Threshold Level's range and default.
const THRESHOLD_MIN: f64 = 1.0;
const THRESHOLD_MAX: f64 = 255.0;
const THRESHOLD_DEFAULT: f64 = 128.0;
/// The bins of the plot while the host gives no histogram.
const THRESHOLD_BINS: usize = 256;

/// What a Threshold editor keeps between passes: the level its running gesture began with.
#[derive(Clone, Default)]
struct ThresholdMemory {
    began: Option<f64>,
}

/// Threshold: the image's histogram with a marker at the level, then the Threshold Level row (1..=255, default 128,
/// whole numbers).
///
/// The histogram is the bordered plot of [`Histogram`] with its marker at the level; AccessKit names it "Histogram".
/// The row behaves as an [`AdjustSliders`](super::AdjustSliders) row.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct ThresholdEditor<'a> {
    label: Label,
    level: Bind<'a, f64>,
    histogram: Option<&'a HistogramData>,
    context: AdjustContext,
}

impl<'a> ThresholdEditor<'a> {
    /// The editor named `label` of `level` (1..=255).
    pub fn new(label: impl Into<Label>, level: Bind<'a, f64>) -> Self {
        Self { label: label.into(), level, histogram: None, context: AdjustContext::dialog(true) }
    }

    /// The image's histogram; an empty plot without it.
    #[must_use]
    pub fn histogram(mut self, data: &'a HistogramData) -> Self {
        self.histogram = Some(data);
        self
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit of the level.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        let ThresholdEditor { label, mut level, histogram, context } = self;
        let words = words(ui);
        let mut edit = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut level, cx).unwrap_or(THRESHOLD_DEFAULT).clamp(THRESHOLD_MIN, THRESHOLD_MAX);
            let empty = HistogramData::tone(&[0u32; THRESHOLD_BINS], ChannelColor::Neutral);
            let data = histogram.unwrap_or(&empty);
            let _ = Histogram::new(words.histogram.clone(), data).bordered().marker(now as f32).show(ui);
            let row = AdjustField::new("level", words.threshold_level.clone(), THRESHOLD_MIN..=THRESHOLD_MAX, THRESHOLD_DEFAULT);
            let mut run = Run::start(&mut level, context, now, cx.memory::<ThresholdMemory>().began);
            run.slider_row(ui, cx, &row, now, |_, x| x);
            cx.memory::<ThresholdMemory>().began = run.began();
            edit = Some(run.finish());
        });
        lifted(edit, shown.output)
    }
}

// ---------------------------------------------------------------------------------------------
// Color Balance

/// The tone Color Balance shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorBalanceTone {
    /// The shadows.
    Shadows,
    /// The default, and where a reset returns.
    #[default]
    Midtones,
    /// The highlights.
    Highlights,
}

/// A Color Balance value: per tone the Cyan–Red, Magenta–Green and Yellow–Blue shifts, −100..=100, whole numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorBalance {
    /// Shifts for the shadows: Cyan–Red, Magenta–Green, Yellow–Blue.
    pub shadows: [f64; 3],
    /// Shifts for the midtones, in the same order.
    pub midtones: [f64; 3],
    /// Shifts for the highlights, in the same order.
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
    /// A value of the shifts of each tone and Preserve Luminosity.
    pub fn new(shadows: [f64; 3], midtones: [f64; 3], highlights: [f64; 3], preserve_luminosity: bool) -> Self {
        Self { shadows, midtones, highlights, preserve_luminosity }
    }

    /// Photoshop's reset to defaults (Alt+Cancel; the Properties panel's Reset once the layer is as the panel found
    /// it): every tone back to 0, Preserve Luminosity kept. The editor shows Midtones again when the host gives
    /// [`ColorBalanceEditor::tone`] `Midtones`.
    pub fn reset(&mut self) {
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

/// What a Color Balance editor keeps between passes: the tone it shows and the value its running gesture began with.
#[derive(Clone, Default)]
struct BalanceMemory {
    tone: Option<ColorBalanceTone>,
    began: Option<ColorBalance>,
}

/// What a [`ColorBalanceEditor`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ColorBalanceOutput {
    /// The edit of the value.
    pub edit: EditResponse<ColorBalance>,
    /// The tone shown after the pass; a host that keeps the tone stores it.
    pub tone: ColorBalanceTone,
}

impl Outcome for ColorBalanceOutput {
    type Value = ColorBalance;

    fn response(&self) -> &egui::Response {
        &self.edit.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.edit.story_facts()
    }
}

impl std::ops::Deref for ColorBalanceOutput {
    type Target = EditResponse<ColorBalance>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

/// Color Balance: the Tone radios, three two-colour slider rows for the shown tone, and Preserve Luminosity.
///
/// - The Tone radios (Shadows, Midtones, Highlights) pick the tone shown. The choice is view state: the editor keeps
///   it, Midtones at first, and it survives value changes. A host that keeps it sets [`tone`](Self::tone) and stores
///   the output's.
/// - Rows "Cyan · Red", "Magenta · Green", "Yellow · Blue", −100..100, with two-colour tracks while dynamic tracks are
///   on. A reset zeroes only that row of the shown tone.
/// - Preserve Luminosity is one `Set`.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct ColorBalanceEditor<'a> {
    label: Label,
    value: Bind<'a, ColorBalance>,
    tone: Option<ColorBalanceTone>,
    context: AdjustContext,
}

impl<'a> ColorBalanceEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, ColorBalance>) -> Self {
        Self { label: label.into(), value, tone: None, context: AdjustContext::dialog(true) }
    }

    /// The tone shown, kept by the host.
    #[must_use]
    pub fn tone(mut self, tone: ColorBalanceTone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit of the `ColorBalance` and the tone shown.
    pub fn show(self, ui: &mut Ui) -> ColorBalanceOutput {
        let ColorBalanceEditor { label, mut value, tone: host_tone, context } = self;
        let words = words(ui);
        let mut out = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let tone = host_tone.or(cx.memory::<BalanceMemory>().tone).unwrap_or_default();
            let mut picked = None;
            Field::new(words.tone.clone()).show(ui, |ui, _| {
                let group = RadioGroup::new(words.tone.clone(), Bind::given(tone)).show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        for (member, name) in [
                            (ColorBalanceTone::Shadows, &words.shadows),
                            (ColorBalanceTone::Midtones, &words.midtones),
                            (ColorBalanceTone::Highlights, &words.highlights),
                        ] {
                            Radio::new(member).label(name.clone()).show(ui);
                        }
                    });
                });
                picked = group.output.value().copied();
            });
            let rows = [
                ("cyanRed", &words.cyan_red, super::style::balance::CYAN_RED),
                ("magentaGreen", &words.magenta_green, super::style::balance::MAGENTA_GREEN),
                ("yellowBlue", &words.yellow_blue, super::style::balance::YELLOW_BLUE),
            ];
            let mut run = Run::start(&mut value, context, now, cx.memory::<BalanceMemory>().began);
            let shifts = now.of_tone(tone);
            for (i, (key, name, stops)) in rows.into_iter().enumerate() {
                let field = AdjustField::new(key, name.clone(), -100.0..=100.0, 0.0).track(super::AdjustTrack::Colors(stops.to_vec()));
                run.slider_row(ui, cx, &field, shifts[i], move |now: &ColorBalance, x| {
                    let mut next = now.of_tone(tone);
                    next[i] = x;
                    now.with_tone(tone, next)
                });
            }
            let preserve = Checkbox::new(Bind::given(now.preserve_luminosity)).label(words.preserve_luminosity.clone()).show(ui);
            run.push(cx, preserve, |now, on| ColorBalance { preserve_luminosity: *on, ..*now });
            let memory = cx.memory::<BalanceMemory>();
            memory.began = run.began();
            memory.tone = Some(picked.unwrap_or(tone));
            out = Some((run.finish(), picked.unwrap_or(tone)));
        });
        let (edit, tone) = out.expect("an editor shows its body");
        ColorBalanceOutput { edit: lifted(Some(edit), shown.output), tone }
    }
}

// ---------------------------------------------------------------------------------------------
// Photo Filter

/// The colour request's key.
const COLOR_KEY: &str = "photoFilter.color";

/// Density's default.
const DENSITY_DEFAULT: f64 = 25.0;

/// Photo Filter's presets, in Photoshop's menu order, with their colours.
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
        srgb8(r, g, b)
    }

    /// The preset's menu name from `words` (the channel words for Red, Yellow, Green, Cyan, Blue and Magenta).
    pub fn label(self, words: &AdjustWords) -> Label {
        match self {
            PhotoFilterPreset::Warming85 => &words.warming_85,
            PhotoFilterPreset::WarmingLba => &words.warming_lba,
            PhotoFilterPreset::Warming81 => &words.warming_81,
            PhotoFilterPreset::Cooling80 => &words.cooling_80,
            PhotoFilterPreset::CoolingLbb => &words.cooling_lbb,
            PhotoFilterPreset::Cooling82 => &words.cooling_82,
            PhotoFilterPreset::Red => &words.red,
            PhotoFilterPreset::Orange => &words.orange,
            PhotoFilterPreset::Yellow => &words.yellow,
            PhotoFilterPreset::Green => &words.green,
            PhotoFilterPreset::Cyan => &words.cyan,
            PhotoFilterPreset::Blue => &words.blue,
            PhotoFilterPreset::Violet => &words.violet,
            PhotoFilterPreset::Magenta => &words.magenta,
            PhotoFilterPreset::Sepia => &words.sepia,
            PhotoFilterPreset::DeepRed => &words.deep_red,
            PhotoFilterPreset::DeepBlue => &words.deep_blue,
            PhotoFilterPreset::DeepEmerald => &words.deep_emerald,
            PhotoFilterPreset::DeepYellow => &words.deep_yellow,
            PhotoFilterPreset::Underwater => &words.underwater,
        }
        .clone()
    }

    /// The preset whose colour is `color` in 8-bit sRGB, if any (how the editor tells a preset from a custom colour).
    pub fn of(color: Srgba) -> Option<PhotoFilterPreset> {
        PhotoFilterPreset::ALL.into_iter().find(|preset| preset.color().same_rgb8(color))
    }
}

/// A Photo Filter value.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PhotoFilter {
    /// The filter colour (a preset's or the user's).
    pub color: Srgba,
    /// Density 0..=100 %, default 25.
    pub density: f64,
    /// Preserve Luminosity; on by default.
    pub preserve_luminosity: bool,
}

impl PhotoFilter {
    /// A value of the filter `color`, `density` and Preserve Luminosity.
    pub fn new(color: Srgba, density: f64, preserve_luminosity: bool) -> Self {
        Self { color, density, preserve_luminosity }
    }
}

impl Default for PhotoFilter {
    fn default() -> Self {
        Self { color: PhotoFilterPreset::Warming85.color(), density: DENSITY_DEFAULT, preserve_luminosity: true }
    }
}

/// What a Photo Filter editor keeps between passes: Filter or Color, and the value its running gesture began with.
#[derive(Clone, Default)]
struct FilterMemory {
    use_color: Option<bool>,
    began: Option<PhotoFilter>,
}

/// What [`PhotoFilterEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct PhotoFilterOutput {
    /// The edit of the value, as every editor reports it.
    pub edit: EditResponse<PhotoFilter>,
    /// The user clicked the Color swatch: open the colour picker with this request (key `"photoFilter.color"`, title
    /// "Filter Color") and write the answer into the value's `color`.
    pub pick_color: Option<ColorRequest>,
}

impl Outcome for PhotoFilterOutput {
    type Value = PhotoFilter;

    fn response(&self) -> &egui::Response {
        &self.edit.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.edit.story_facts()
    }
}

impl std::ops::Deref for PhotoFilterOutput {
    type Target = EditResponse<PhotoFilter>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

/// Photo Filter: the Filter radio with the preset select, the Color radio with a swatch, Density and Preserve
/// Luminosity.
///
/// - Filter or Color is view state of the editor, Color at first when the value's colour is no preset
///   ([`PhotoFilterPreset::of`]). The other row's control is disabled.
/// - The preset select lists [`PhotoFilterPreset::ALL`], with "Custom" first while the colour is no preset; picking
///   one is a `Set` of its colour.
/// - The swatch asks for a colour through [`PhotoFilterOutput::pick_color`].
/// - Density 0..100 %, default 25 on a reset; Preserve Luminosity a `Set`.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct PhotoFilterEditor<'a> {
    label: Label,
    value: Bind<'a, PhotoFilter>,
    context: AdjustContext,
}

impl<'a> PhotoFilterEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, PhotoFilter>) -> Self {
        Self { label: label.into(), value, context: AdjustContext::dialog(true) }
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit and any colour-picker request.
    pub fn show(self, ui: &mut Ui) -> PhotoFilterOutput {
        let PhotoFilterEditor { label, mut value, context } = self;
        let words = words(ui);
        let mut out = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let preset = PhotoFilterPreset::of(now.color);
            let use_color = cx.memory::<FilterMemory>().use_color.unwrap_or(preset.is_none());
            let mut preset_edit = None;
            let mut pick_color = None;
            let group = RadioGroup::new(words.filter.clone(), Bind::given(use_color)).show(ui, |ui, _| {
                let mut entries: Vec<Entry<Option<PhotoFilterPreset>>> = Vec::new();
                if preset.is_none() {
                    entries.push(Entry::new(None, words.custom.clone()));
                }
                entries.extend(PhotoFilterPreset::ALL.into_iter().map(|each| Entry::new(Some(each), each.label(&words))));
                Field::new(words.filter.clone()).id_salt("filter").show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        Radio::new(false).label(words.filter.clone()).show(ui);
                        preset_edit =
                            Some(Select::new(words.filter.clone(), Bind::given(preset), &entries).enabled(!use_color).id_salt("preset").show(ui));
                    });
                });
                Field::new(words.color.clone()).id_salt("color").show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        Radio::new(true).label(words.color.clone()).show(ui);
                        let swatch =
                            Swatch::new(words.filter_color.clone(), now.color).picker(COLOR_KEY).enabled(use_color).id_salt("swatch").show(ui);
                        pick_color = swatch.pick_color;
                    });
                });
            });
            let mode = group.output.value().copied();
            let mut run = Run::start(&mut value, context, now, cx.memory::<FilterMemory>().began);
            if let Some(edit) = preset_edit {
                run.push(cx, edit, |now, entry: &Option<PhotoFilterPreset>| PhotoFilter {
                    color: entry.map_or(now.color, PhotoFilterPreset::color),
                    ..*now
                });
            }
            let density = AdjustField::new("density", words.density.clone(), 0.0..=100.0, DENSITY_DEFAULT).unit_kind(UnitKind::Percent);
            run.slider_row(ui, cx, &density, now.density, |now, density| PhotoFilter { density, ..*now });
            let preserve = Checkbox::new(Bind::given(now.preserve_luminosity)).label(words.preserve_luminosity.clone()).show(ui);
            run.push(cx, preserve, |now, on| PhotoFilter { preserve_luminosity: *on, ..*now });
            let memory = cx.memory::<FilterMemory>();
            memory.began = run.began();
            memory.use_color = Some(mode.unwrap_or(use_color));
            out = Some((run.finish(), pick_color));
        });
        let (edit, pick_color) = out.expect("an editor shows its body");
        PhotoFilterOutput { edit: lifted(Some(edit), shown.output), pick_color }
    }
}
