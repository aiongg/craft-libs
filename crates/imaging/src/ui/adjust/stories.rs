//! Stories of the adjustment editors. `adjust::stories()` returns these and the Levels editor's;
//! `adjust::COMPONENTS` lists both.
//!
//! Histograms and the adjusted hue spectrum are fakes made by the story (`story_bins` hills, a hue
//! rotation), so every state shows without an engine. Each cell keeps its own value and applies
//! the editor's events to it (`Begin`/`Update` as a preview, `Commit`/`Set` as the value, `Cancel`
//! back to the value before the gesture). Every string goes through words ([`AdjustStoryLabels`]
//! and the editors' own [`AdjustLabels`]).
//!
//! | Story id | Component | Shows |
//! |---|---|---|
//! | `composites/levels-panel/states` | `LevelsPanel` | RGB with a fake histogram, Auto pressed, a picker armed, a grey document |
//! | `composites/curves-panel/states` | `CurvesPanel` | Points; pencil with Smooth; the overlays on the composite; an isolated mask channel |
//! | `composites/adjust-sliders/tables` | `AdjustSliders` | Exposure, Vibrance, Posterize, and a VectorCraft-style table with colour tracks |
//! | `composites/brightness-contrast-editor/modes` | `BrightnessContrastEditor` | Normal and Use Legacy |
//! | `composites/threshold-editor/marker` | `ThresholdEditor` | The marker on a fake histogram |
//! | `composites/hue-saturation-editor/ranges` | `HueSaturationEditor` | Master, a range with its thumbs, Colorize, a range wrapping through 0° |
//! | `composites/color-balance-editor/tones` | `ColorBalanceEditor` | The three tones |
//! | `composites/black-white-editor/tint` | `BlackWhiteEditor` | Default and tinted |
//! | `composites/photo-filter-editor/sources` | `PhotoFilterEditor` | A preset and a custom colour |
//! | `composites/channel-mixer-editor/outputs` | `ChannelMixerEditor` | Colour, Monochrome, Total over 100 % |
//! | `composites/selective-color-editor/methods` | `SelectiveColorEditor` | Relative, Absolute, Neutrals |
//! | `composites/adjust-session/properties-panel` | `AdjustSession` | Color Balance with Reset, Hue/Saturation and Brightness/Contrast with `AdjustContext::properties` (no wheel) |
//!
//! The first eleven take the arguments `dialog` (`AdjustContext::dialog` or `properties`) and
//! `dynamic_colors` (Preferences › Interface › Dynamic Color Sliders). The point pickers are
//! inside the Levels and Curves panels, so `TonePickerRow` has no story of its own.

use craft_theme::{Hsb, Srgba};
use craft_ui::composites::curve_editor::story_bins;
use craft_ui::composites::{ChannelColor, Curve};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::{Context, Ui};

use super::sliders::PERCENT;
use super::{
    AdjustContext, AdjustField, AdjustLabels, AdjustSession, AdjustSliders, AdjustTrack, BlackWhite, BlackWhiteEditor, BlackWhiteOutput,
    BrightnessContrast, BrightnessContrastEditor, ChannelMixer, ChannelMixerEditor, ColorBalance, ColorBalanceEditor, ColorBalanceTone, CurvesMode,
    CurvesPanel, HueRangeValue, HueSaturation, HueSaturationEditor, Levels, LevelsPanel, PhotoFilter, PhotoFilterEditor, PhotoFilterOutput,
    PhotoFilterPreset, SelectiveColor, SelectiveColorEditor, SelectiveMethod, ThresholdEditor, ToneChannel, ToneChannels, ToneCurves, TonePicker,
    ToneSpace,
};

craft_ui::words! {
    /// The captions of the adjustment stories, English by default.
    #[non_exhaustive]
    pub(crate) struct AdjustStoryLabels {
        levels_rgb: "adjustStory.levelsRgb" = "RGB with a histogram";
        levels_auto: "adjustStory.levelsAuto" = "After Auto";
        levels_picker: "adjustStory.levelsPicker" = "Black point picker armed";
        levels_gray: "adjustStory.levelsGray" = "Grayscale document";
        curves_points: "adjustStory.curvesPoints" = "Points";
        curves_pencil: "adjustStory.curvesPencil" = "Pencil, with Smooth";
        curves_overlays: "adjustStory.curvesOverlays" = "Composite over the channel curves";
        curves_isolated: "adjustStory.curvesIsolated" = "One isolated mask channel";
        mask_name: "adjustStory.maskName" = "Layer mask";
        posterize: "adjustStory.posterize" = "Posterize";
        table: "adjustStory.table" = "Table with colour tracks";
        cyan_red: "adjustStory.cyanRed" = "Cyan / Red";
        magenta_green: "adjustStory.magentaGreen" = "Magenta / Green";
        yellow_blue: "adjustStory.yellowBlue" = "Yellow / Blue";
        normal: "adjustStory.normal" = "Normal";
        range_thumbs: "adjustStory.rangeThumbs" = "A range with its thumbs";
        range_wrap: "adjustStory.rangeWrap" = "A range through 0°";
        tinted: "adjustStory.tinted" = "Tinted";
        photo_preset: "adjustStory.photoPreset" = "A filter preset";
        photo_custom: "adjustStory.photoCustom" = "A custom colour";
        mixer_over: "adjustStory.mixerOver" = "Total over 100 %";
        properties_balance: "adjustStory.propertiesBalance" = "Color Balance with Reset";
        properties_reset: "adjustStory.propertiesReset" = "Reset";
        properties_brightness: "adjustStory.propertiesBrightness" = "Brightness/Contrast";
        properties_hue: "adjustStory.propertiesHue" = "Hue/Saturation";
    }
}

/// The args of the stories that take the context.
const DEFAULTS: ContextArgs = ContextArgs { dialog: true, dynamic_colors: true };

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/levels-panel/states", Level::Composites, "LevelsPanel", "Levels panel", DEFAULTS, show_levels),
        typed_story!("composites/curves-panel/states", Level::Composites, "CurvesPanel", "Curves panel", DEFAULTS, show_curves),
        typed_story!("composites/adjust-sliders/tables", Level::Composites, "AdjustSliders", "Slider tables", DEFAULTS, show_sliders),
        typed_story!(
            "composites/brightness-contrast-editor/modes",
            Level::Composites,
            "BrightnessContrastEditor",
            "Brightness/Contrast",
            DEFAULTS,
            show_brightness_contrast
        ),
        typed_story!("composites/threshold-editor/marker", Level::Composites, "ThresholdEditor", "Threshold", DEFAULTS, show_threshold),
        typed_story!(
            "composites/hue-saturation-editor/ranges",
            Level::Composites,
            "HueSaturationEditor",
            "Hue/Saturation",
            DEFAULTS,
            show_hue_saturation
        ),
        typed_story!("composites/color-balance-editor/tones", Level::Composites, "ColorBalanceEditor", "Color Balance", DEFAULTS, show_color_balance),
        typed_story!("composites/black-white-editor/tint", Level::Composites, "BlackWhiteEditor", "Black & White", DEFAULTS, show_black_white),
        typed_story!("composites/photo-filter-editor/sources", Level::Composites, "PhotoFilterEditor", "Photo Filter", DEFAULTS, show_photo_filter),
        typed_story!(
            "composites/channel-mixer-editor/outputs",
            Level::Composites,
            "ChannelMixerEditor",
            "Channel Mixer",
            DEFAULTS,
            show_channel_mixer
        ),
        typed_story!(
            "composites/selective-color-editor/methods",
            Level::Composites,
            "SelectiveColorEditor",
            "Selective Color",
            DEFAULTS,
            show_selective_color
        ),
        typed_story!(
            "composites/adjust-session/properties-panel",
            Level::Composites,
            "AdjustSession",
            "Properties panel",
            PropertiesArgs { dynamic_colors: true },
            show_properties
        ),
    ]
}

#[derive(StoryArgs, Clone, Copy, Debug, PartialEq)]
struct ContextArgs {
    /// In an Image › Adjustments dialog (the wheel steps a slider); off is the Properties panel.
    dialog: bool,
    /// Preferences › Interface › Dynamic Color Sliders: colour tracks on the colour sliders.
    dynamic_colors: bool,
}

impl ContextArgs {
    fn context(&self) -> AdjustContext {
        if self.dialog { AdjustContext::dialog(self.dynamic_colors) } else { AdjustContext::properties(self.dynamic_colors) }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PropertiesArgs {
    /// Preferences › Interface › Dynamic Color Sliders.
    dynamic_colors: bool,
}

/// A cell's value and the value before the gesture under way, which `Cancel` restores.
#[derive(Clone)]
struct Doc<D> {
    value: D,
    before: Option<D>,
}

impl<D: Clone> Doc<D> {
    /// Applies an editor's event: `set` puts the event's value into the document.
    fn apply<T: Clone>(&mut self, event: Option<EditEvent<T>>, set: impl Fn(&mut D, T)) {
        let Some(event) = event else { return };
        match &event {
            EditEvent::Begin { .. } => self.before = Some(self.value.clone()),
            EditEvent::Cancel { .. } => {
                if let Some(before) = self.before.take() {
                    self.value = before;
                }
                return;
            }
            EditEvent::Commit { .. } | EditEvent::Set { .. } => self.before = None,
            _ => {}
        }
        if let Some(value) = event.value() {
            set(&mut self.value, value.clone());
        }
    }
}

/// Runs `body` on the cell `salt`'s kept [`Doc`], made by `init` the first time.
fn with_doc<D: Clone + Send + Sync + 'static, R>(
    ui: &mut Ui,
    salt: &str,
    init: impl FnOnce() -> D,
    body: impl FnOnce(&mut Ui, &mut Doc<D>) -> R,
) -> R {
    let id = ui.scope_id().with(("adjust-story", salt));
    let mut doc = ui.data(|d| d.get_temp::<Doc<D>>(id)).unwrap_or_else(|| Doc { value: init(), before: None });
    let out = body(ui, &mut doc);
    ui.data_mut(|d| d.insert_temp(id, doc));
    out
}

/// Runs `f` once for the cell `salt`: it sets the editor's view state (the shown range or tone).
fn once(ui: &Ui, salt: &str, f: impl FnOnce(&Context)) {
    let id = ui.scope_id().with(("adjust-story-seed", salt));
    if !ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false)) {
        f(ui.ctx());
        ui.data_mut(|d| d.insert_temp(id, true));
    }
}

type Cell<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A caption and the editor under it.
fn cell<'a>(caption: &'a str, body: impl FnOnce(&mut Ui) + 'a) -> Cell<'a> {
    Box::new(move |ui| {
        ui.add(Text::new(caption).content(Content::Muted));
        body(ui);
    })
}

/// `cells` in rows of `per_row` equal columns.
fn grid(ui: &mut Ui, per_row: usize, cells: Vec<Cell<'_>>) {
    let mut cells = cells.into_iter().peekable();
    while cells.peek().is_some() {
        let row: Vec<Cell<'_>> = cells.by_ref().take(per_row).collect();
        ui.columns(per_row, |columns| {
            for (cell, column) in row.into_iter().zip(columns.iter_mut()) {
                cell(column);
            }
        });
    }
}

/// A fake channel histogram: two hills, shifted by `shift`.
fn counts(shift: f32) -> [u32; 256] {
    let mut out = [0; 256];
    for (slot, bin) in out.iter_mut().zip(story_bins(shift)) {
        *slot = bin;
    }
    out
}

/// The histograms of an RGB composite and its three channels.
fn rgb_counts() -> Vec<[u32; 256]> {
    [0.0, 0.0, 12.0, -12.0].into_iter().map(counts).collect()
}

fn rgb() -> ToneChannels {
    ToneChannels::of(ToneSpace::Rgb, false)
}

/// A story-local hue at full saturation and brightness.
fn hue_colour(degrees: f64) -> Srgba {
    Srgba::from_hsb(Hsb { h: degrees.rem_euclid(360.0) as f32, s: 1.0, b: 1.0 })
}

// --- Levels ---

#[derive(Clone)]
struct LevelsState {
    levels: Vec<Levels>,
    armed: Option<TonePicker>,
}

fn levels_cell(
    ui: &mut Ui,
    salt: &'static str,
    channels: ToneChannels,
    context: AdjustContext,
    histograms: &[[u32; 256]],
    levels: Vec<Levels>,
    armed: Option<TonePicker>,
) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || LevelsState { levels, armed },
        |ui, doc| {
            let out = LevelsPanel::new(salt, &doc.value.levels, channels, &numbers)
                .histograms(histograms)
                .picker(doc.value.armed)
                .context(context)
                .show(ui);
            let channel = out.channel;
            doc.apply(out.edit.event, |state, levels| {
                if let Some(slot) = state.levels.get_mut(channel) {
                    *slot = levels;
                }
            });
            if let Some(armed) = out.picker {
                doc.value.armed = armed;
            }
        },
    );
}

fn show_levels(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let context = args.context();
    let histograms = rgb_counts();
    let identity = || vec![Levels::identity(0.0..=255.0); 4];
    let mut auto = identity();
    auto[0] = auto[0].auto(&histograms[0]);
    let gray = [counts(0.0)];
    grid(
        ui,
        2,
        vec![
            cell(&labels.levels_rgb, |ui| levels_cell(ui, "levels-rgb", rgb(), context, &histograms, identity(), None)),
            cell(&labels.levels_auto, |ui| levels_cell(ui, "levels-auto", rgb(), context, &histograms, auto, None)),
            cell(&labels.levels_picker, |ui| levels_cell(ui, "levels-picker", rgb(), context, &histograms, identity(), Some(TonePicker::Black))),
            cell(&labels.levels_gray, |ui| {
                let channels = ToneChannels::of(ToneSpace::Rgb, true);
                levels_cell(ui, "levels-gray", channels, context.gray(true), &gray, vec![Levels::identity(0.0..=255.0)], None);
            }),
        ],
    );
}

// --- Curves ---

/// A drawn map: the identity with a bump.
fn bumpy_map() -> Vec<f32> {
    (0..256).map(|k| (k as f32 + 14.0 * (k as f32 / 255.0 * std::f32::consts::TAU * 3.0).sin()).clamp(0.0, 255.0)).collect()
}

/// The drawn curve as points again: every 32nd level and the end.
fn map_points(map: &[f32]) -> Curve {
    let last = map.len().saturating_sub(1);
    let mut points: Vec<[f32; 2]> = (0..last).step_by(32).map(|k| [k as f32, map[k]]).collect();
    points.push([last as f32, map.get(last).copied().unwrap_or(255.0)]);
    Curve::smooth(points)
}

/// The host's conversion when the panel reports a mode switch.
fn convert(curves: &mut ToneCurves, mode: CurvesMode) {
    for curve in &mut curves.curves {
        *curve = match (mode, &*curve) {
            (CurvesMode::Pencil, Curve::Points { points, .. }) => Curve::Map(Curve::monotone(points, 255.0)),
            (CurvesMode::Points, Curve::Map(map)) => map_points(map),
            _ => continue,
        };
    }
    curves.mode = mode;
}

fn curves_cell(ui: &mut Ui, salt: &'static str, channels: ToneChannels, context: AdjustContext, histograms: &[[u32; 256]], curves: ToneCurves) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || curves,
        |ui, doc| {
            let out = CurvesPanel::new(salt, &doc.value, channels, &numbers).histograms(histograms).context(context).show(ui);
            let channel = out.channel;
            doc.apply(out.edit.event, |curves, curve| {
                if let Some(slot) = curves.curves.get_mut(channel) {
                    *slot = curve;
                }
            });
            if let Some(mode) = out.mode {
                convert(&mut doc.value, mode);
            }
        },
    );
}

fn show_curves(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let context = args.context();
    let histograms = rgb_counts();
    let single = [counts(0.0)];
    let channels = rgb();
    let pencil = ToneCurves { curves: vec![Curve::Map(bumpy_map()); channels.len()], mode: CurvesMode::Pencil };
    let mut overlays = ToneCurves::identity(&channels);
    overlays.curves[1] = Curve::smooth(vec![[0.0, 0.0], [96.0, 122.0], [255.0, 255.0]]);
    overlays.curves[2] = Curve::smooth(vec![[0.0, 0.0], [128.0, 110.0], [255.0, 255.0]]);
    overlays.curves[3] = Curve::smooth(vec![[0.0, 12.0], [160.0, 150.0], [255.0, 240.0]]);
    let mask = ToneChannels::isolated(ToneChannel::Named { name: labels.mask_name.clone(), color: ChannelColor::Neutral });
    let isolated = ToneCurves::identity(&mask);
    grid(
        ui,
        2,
        vec![
            cell(&labels.curves_points, |ui| {
                curves_cell(ui, "curves-points", rgb(), context, &histograms, ToneCurves::identity(&rgb()));
            }),
            cell(&labels.curves_pencil, |ui| curves_cell(ui, "curves-pencil", rgb(), context, &histograms, pencil)),
            cell(&labels.curves_overlays, |ui| curves_cell(ui, "curves-overlays", rgb(), context, &histograms, overlays)),
            cell(&labels.curves_isolated, |ui| curves_cell(ui, "curves-isolated", mask, context, &single, isolated)),
        ],
    );
}

// --- Slider tables ---

fn table_cell(ui: &mut Ui, salt: &'static str, fields: Vec<AdjustField>, context: AdjustContext) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        || fields.iter().map(|field| field.default).collect::<Vec<f64>>(),
        |ui, doc| {
            let edit = AdjustSliders::new(salt, &fields, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |values, new| *values = new);
        },
    );
}

/// A VectorCraft-style table: the colour effects' rows, each over a two-colour track. The track
/// ends are the colours the rows name (data, not theme colours).
fn colour_table(labels: &AdjustStoryLabels) -> Vec<AdjustField> {
    let row = |key, label: &str, from: Srgba, to: Srgba| {
        AdjustField::new(key, label, -100.0..=100.0, 0.0).unit(Unit::new(PERCENT)).track(AdjustTrack::Colors(vec![from, to]))
    };
    vec![
        row("cyan-red", &labels.cyan_red, Srgba::rgb(0.0, 1.0, 1.0), Srgba::rgb(1.0, 0.0, 0.0)),
        row("magenta-green", &labels.magenta_green, Srgba::rgb(1.0, 0.0, 1.0), Srgba::rgb(0.0, 1.0, 0.0)),
        row("yellow-blue", &labels.yellow_blue, Srgba::rgb(1.0, 1.0, 0.0), Srgba::rgb(0.0, 0.0, 1.0)),
    ]
}

fn show_sliders(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let names = AdjustLabels::of(ui);
    let context = args.context();
    grid(
        ui,
        2,
        vec![
            cell(&names.exposure, |ui| table_cell(ui, "sliders-exposure", AdjustField::exposure(&names), context)),
            cell(&names.vibrance, |ui| table_cell(ui, "sliders-vibrance", AdjustField::vibrance(&names), context)),
            cell(&labels.posterize, |ui| table_cell(ui, "sliders-posterize", AdjustField::posterize(&names), context)),
            cell(&labels.table, |ui| table_cell(ui, "sliders-table", colour_table(&labels), context)),
        ],
    );
}

// --- Brightness/Contrast and Threshold ---

fn brightness_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, start: BrightnessContrast) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            let edit = BrightnessContrastEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_brightness_contrast(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let normal = BrightnessContrast { brightness: 30.0, contrast: -20.0, legacy: false };
    grid(
        ui,
        2,
        vec![
            cell(&labels.normal, |ui| brightness_cell(ui, "bc-normal", context, normal)),
            cell(&names.use_legacy, |ui| brightness_cell(ui, "bc-legacy", context, BrightnessContrast { contrast: -80.0, legacy: true, ..normal })),
        ],
    );
}

fn show_threshold(ui: &mut Ui, args: &mut ContextArgs) {
    let context = args.context();
    let numbers = LocaleNumbers::of(ui);
    let histogram = counts(0.0);
    with_doc(
        ui,
        "threshold",
        || 128.0,
        |ui, doc| {
            let edit = ThresholdEditor::new("threshold", doc.value, &numbers).histogram(&histogram).context(context).show(ui);
            doc.apply(edit.event, |level, new| *level = new);
        },
    );
}

// --- Hue/Saturation ---

fn hue_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, shown: usize, start: HueSaturation) {
    let numbers = LocaleNumbers::of(ui);
    // The shown range is view state in memory: 0 Master, 1 Reds ... 6 Magentas.
    once(ui, salt, |ctx| {
        let id = HueSaturationEditor::id_in(ui, salt);
        ctx.data_mut(|d| d.insert_temp(id.with("range"), shown));
    });
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            // The adjusted spectrum: the input turned by the Master hue.
            let shift = doc.value.hue;
            let adjusted = move |hue: f64| hue_colour(hue + shift);
            let edit = HueSaturationEditor::new(salt, &doc.value, &numbers).adjusted_spectrum(&adjusted).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_hue_saturation(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let master = HueSaturation { hue: 40.0, saturation: 15.0, ..HueSaturation::default() };
    let mut thumbs = HueSaturation::default();
    thumbs.ranges[2] = HueRangeValue { hue: -30.0, saturation: 20.0, ..HueRangeValue::neutral(2) };
    let colorize = HueSaturation { hue: 200.0, saturation: 40.0, colorize: true, ..HueSaturation::default() };
    let mut wrap = HueSaturation::default();
    wrap.ranges[0] = HueRangeValue { hue: 25.0, saturation: -20.0, ..HueRangeValue::neutral(0) };
    grid(
        ui,
        2,
        vec![
            cell(&names.master, |ui| hue_cell(ui, "hue-master", context, 0, master)),
            cell(&labels.range_thumbs, |ui| hue_cell(ui, "hue-thumbs", context, 3, thumbs)),
            cell(&names.colorize, |ui| hue_cell(ui, "hue-colorize", context, 0, colorize)),
            cell(&labels.range_wrap, |ui| hue_cell(ui, "hue-wrap", context, 1, wrap)),
        ],
    );
}

// --- Color Balance ---

fn balance_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, tone: ColorBalanceTone, start: ColorBalance) {
    let numbers = LocaleNumbers::of(ui);
    once(ui, salt, |ctx| {
        let id = ColorBalanceEditor::id_in(ui, salt);
        ctx.data_mut(|d| d.insert_temp(id.with("tone"), tone));
    });
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            let edit = ColorBalanceEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_color_balance(ui: &mut Ui, args: &mut ContextArgs) {
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let value =
        ColorBalance { shadows: [-20.0, 0.0, 30.0], midtones: [10.0, -15.0, 0.0], highlights: [25.0, 10.0, -20.0], ..ColorBalance::default() };
    grid(
        ui,
        3,
        vec![
            cell(&names.shadows, |ui| balance_cell(ui, "balance-shadows", context, ColorBalanceTone::Shadows, value)),
            cell(&names.midtones, |ui| balance_cell(ui, "balance-midtones", context, ColorBalanceTone::Midtones, value)),
            cell(&names.highlights, |ui| balance_cell(ui, "balance-highlights", context, ColorBalanceTone::Highlights, value)),
        ],
    );
}

// --- Black & White ---

fn black_white_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, start: BlackWhite) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            // The swatch's colour request needs a colour dialog, which a story does not open.
            let BlackWhiteOutput { edit, .. } = BlackWhiteEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_black_white(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let tinted = BlackWhite { weights: [60.0, 80.0, 30.0, 70.0, 10.0, 40.0], tint: true, ..BlackWhite::default() };
    grid(
        ui,
        2,
        vec![
            cell(&names.default_button, |ui| black_white_cell(ui, "bw-default", context, BlackWhite::default())),
            cell(&labels.tinted, |ui| black_white_cell(ui, "bw-tinted", context, tinted)),
        ],
    );
}

// --- Photo Filter ---

fn photo_filter_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, start: PhotoFilter) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            // The swatch's colour request needs a colour dialog, which a story does not open.
            let PhotoFilterOutput { edit, .. } = PhotoFilterEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_photo_filter(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let context = args.context();
    let preset = PhotoFilter { color: PhotoFilterPreset::Sepia.color(), density: 25.0, preserve_luminosity: true };
    // A colour no preset has.
    let custom = PhotoFilter { color: Srgba::rgb(0.2, 0.6, 0.5), ..preset };
    grid(
        ui,
        2,
        vec![
            cell(&labels.photo_preset, |ui| photo_filter_cell(ui, "photo-preset", context, preset)),
            cell(&labels.photo_custom, |ui| photo_filter_cell(ui, "photo-custom", context, custom)),
        ],
    );
}

// --- Channel Mixer ---

fn mixer_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, start: ChannelMixer) {
    let numbers = LocaleNumbers::of(ui);
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            let edit = ChannelMixerEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_channel_mixer(ui: &mut Ui, args: &mut ContextArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let colour = ChannelMixer { red: [80.0, 20.0, 0.0, 0.0], ..ChannelMixer::default() };
    let monochrome = ChannelMixer { gray: Some(ChannelMixer::MONOCHROME_START), ..ChannelMixer::default() };
    let over = ChannelMixer { red: [70.0, 50.0, 0.0, 0.0], ..ChannelMixer::default() };
    grid(
        ui,
        3,
        vec![
            cell(&names.color, |ui| mixer_cell(ui, "mixer-colour", context, colour)),
            cell(&names.monochrome, |ui| mixer_cell(ui, "mixer-monochrome", context, monochrome)),
            cell(&labels.mixer_over, |ui| mixer_cell(ui, "mixer-over", context, over)),
        ],
    );
}

// --- Selective Color ---

fn selective_cell(ui: &mut Ui, salt: &'static str, context: AdjustContext, shown: usize, start: SelectiveColor) {
    let numbers = LocaleNumbers::of(ui);
    // The shown range is view state in memory: 0 Reds ... 8 Blacks.
    once(ui, salt, |ctx| {
        let id = SelectiveColorEditor::id_in(ui, salt);
        ctx.data_mut(|d| d.insert_temp(id.with("range"), shown));
    });
    with_doc(
        ui,
        salt,
        move || start,
        |ui, doc| {
            let edit = SelectiveColorEditor::new(salt, &doc.value, &numbers).context(context).show(ui);
            doc.apply(edit.event, |value, new| *value = new);
        },
    );
}

fn show_selective_color(ui: &mut Ui, args: &mut ContextArgs) {
    let names = AdjustLabels::of(ui);
    let context = args.context();
    let mut relative = SelectiveColor::default();
    relative.ranges[0] = [30.0, -10.0, 20.0, 0.0];
    let mut absolute = SelectiveColor { method: SelectiveMethod::Absolute, ..SelectiveColor::default() };
    absolute.ranges[4] = [-20.0, 25.0, 0.0, 10.0];
    let mut neutrals = SelectiveColor::default();
    neutrals.ranges[7] = [5.0, 0.0, -15.0, 10.0];
    grid(
        ui,
        3,
        vec![
            cell(&names.relative, |ui| selective_cell(ui, "selective-relative", context, 0, relative)),
            cell(&names.absolute, |ui| selective_cell(ui, "selective-absolute", context, 4, absolute)),
            cell(&names.neutrals, |ui| selective_cell(ui, "selective-neutrals", context, 7, neutrals)),
        ],
    );
}

// --- Properties panel ---

/// Color Balance and the session that remembers what the panel found.
#[derive(Clone)]
struct BalanceState {
    value: ColorBalance,
    session: AdjustSession<ColorBalance>,
}

/// The target the panel's session tracks (one fake adjustment layer).
const SESSION_TARGET: u64 = 1;

fn show_properties(ui: &mut Ui, args: &mut PropertiesArgs) {
    let labels = AdjustStoryLabels::of(ui);
    let context = AdjustContext::properties(args.dynamic_colors);
    let found = ColorBalance { midtones: [20.0, -10.0, 15.0], ..ColorBalance::default() };
    grid(
        ui,
        2,
        vec![
            cell(&labels.properties_balance, |ui| {
                let numbers = LocaleNumbers::of(ui);
                let id = ColorBalanceEditor::id_in(ui, "props-balance");
                let state = BalanceState { value: found, session: AdjustSession::new() };
                with_doc(
                    ui,
                    "props-balance",
                    move || state,
                    |ui, doc| {
                        let value = doc.value.value;
                        doc.value.session.track(SESSION_TARGET, ui.ctx().cumulative_pass_nr(), &value);
                        let edit = ColorBalanceEditor::new("props-balance", &doc.value.value, &numbers).context(context).show(ui);
                        doc.apply(edit.event, |state, new| {
                            state.value = new;
                            state.session.edited();
                        });
                        if ui.add(Button::new(labels.properties_reset.as_str())).clicked() {
                            doc.value.value = doc.value.session.reset(ColorBalance::default);
                            ColorBalanceEditor::reset_tone(ui.ctx(), id);
                        }
                    },
                );
            }),
            cell(&labels.properties_hue, |ui| {
                let start = HueSaturation { saturation: 20.0, ..HueSaturation::default() };
                hue_cell(ui, "props-hue", context, 0, start);
            }),
            cell(&labels.properties_brightness, |ui| {
                brightness_cell(ui, "props-bc", context, BrightnessContrast { brightness: 20.0, ..BrightnessContrast::default() })
            }),
        ],
    );
}

#[cfg(test)]
mod tests {
    use craft_ui::theme::{Base, install};
    use egui::{Context, RawInput};

    // Every story draws its default state, twice (the second pass reads the kept cell values).
    #[test]
    fn every_story_shows_with_its_default_args() {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        for story in super::stories() {
            let mut args = story.defaults();
            for _ in 0..2 {
                ctx.run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
            }
        }
    }
}
