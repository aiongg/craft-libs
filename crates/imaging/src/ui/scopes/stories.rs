//! Stories of the scope panel.
//!
//! Every story uses fakes: histograms generated from a seed (RGB and CMYK modes), a vectorscope
//! counted from generated pixels, a generated preview image for the clipping overlay, and story
//! words (the gallery's pseudo-locale test). Nothing runs in the background: every count is made
//! in the frame that shows it.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/scope-panel/compact` | Compact View; arg: RGB or CMYK mode |
//! | `patterns/scope-panel/expanded` | Expanded View with the Channel and Source rows and statistics; arg: mode |
//! | `patterns/scope-panel/all-channels` | All Channels View; arg: mode |
//! | `patterns/scope-panel/colors-overlay` | the Colors channel, light and ink |
//! | `patterns/scope-panel/adjusted-over-original` | the outlined original and the stale mark |
//! | `patterns/scope-panel/tone-zones` | five zones, the hover band, a drag |
//! | `patterns/scope-panel/vectorscope` | the cloud, the hover target; args: red position, skin line |
//! | `patterns/scope-panel/clipping` | the warnings on and off, the overlay over a generated image |

use craft_ui::canvas::textures::{Revision, TextureCache, TextureCacheOptions};
use craft_ui::composites::ChannelColor;
use craft_ui::controls::slider::with_values;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryChoice};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Rect, Sense, TextureOptions, Ui, pos2, vec2};

use super::{
    COLORS_CHANNEL, ChannelOverlay, ClippingFrame, ClippingOverlay, HistogramSet, ScopeChannel, ScopeFrame, ScopePanel, ScopePanelLabels,
    ScopePanelState, ScopeSettings, ScopeSource, ScopeView, ToneZone, VectorscopeFrame, ZoneChange,
};
use crate::scopes::{ClippingMask, HUES, SATURATIONS, Vectorscope, hue_saturation};
use crate::ui::ChannelsLabels;

craft_ui::words! {
    /// The channel and source names the scope stories show (the host's words, not the panel's); the
    /// colour channels' names are [`ChannelsLabels`]'.
    #[non_exhaustive]
    pub struct ScopeStoryLabels {
        rgb: "scopesStory.rgb" = "RGB";
        cmyk: "scopesStory.cmyk" = "CMYK";
        black: "scopesStory.black" = "Black";
        luminosity: "scopesStory.luminosity" = "Luminosity";
        selected_layer: "scopesStory.selectedLayer" = "Selected Layer";
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ScopePanel"];

/// The eight stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/scope-panel/compact", Level::Patterns, "ScopePanel", "Compact", ModeArgs::default(), show_compact),
        typed_story!("patterns/scope-panel/expanded", Level::Patterns, "ScopePanel", "Expanded", ModeArgs::default(), show_expanded),
        typed_story!("patterns/scope-panel/all-channels", Level::Patterns, "ScopePanel", "All channels", ModeArgs::default(), show_all_channels),
        typed_story!("patterns/scope-panel/colors-overlay", Level::Patterns, "ScopePanel", "Colors overlay", NoArgs {}, show_colors_overlay),
        typed_story!(
            "patterns/scope-panel/adjusted-over-original",
            Level::Patterns,
            "ScopePanel",
            "Adjusted over original",
            ModeArgs::default(),
            show_adjusted
        ),
        typed_story!("patterns/scope-panel/tone-zones", Level::Patterns, "ScopePanel", "Tone zones", NoArgs {}, show_tone_zones),
        typed_story!("patterns/scope-panel/vectorscope", Level::Patterns, "ScopePanel", "Vectorscope", VectorscopeArgs::default(), show_vectorscope),
        typed_story!("patterns/scope-panel/clipping", Level::Patterns, "ScopePanel", "Clipping", ClippingArgs::default(), show_clipping),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    #[choice(name = "RGB")]
    Rgb,
    #[choice(name = "CMYK")]
    Cmyk,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ModeArgs {
    mode: ModeArg,
}

impl Default for ModeArgs {
    fn default() -> Self {
        Self { mode: ModeArg::Rgb }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct VectorscopeArgs {
    /// Red at 3 o'clock instead of at -100 degrees.
    #[arg(name = "red right")]
    red_right: bool,
    #[arg(name = "skin line")]
    skin_line: bool,
}

impl Default for VectorscopeArgs {
    fn default() -> Self {
        Self { red_right: false, skin_line: true }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ClippingArgs {
    shadows: bool,
    highlights: bool,
}

impl Default for ClippingArgs {
    fn default() -> Self {
        Self { shadows: true, highlights: true }
    }
}

/// A generator of repeatable noise in 0..1 (a linear congruential step).
fn noise(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 8) as f32 / 16_777_216.0
}

/// A hill of counts around `centre` with a smaller one 60 levels up, and noise.
fn hill(seed: u32, centre: f32, width: f32) -> Vec<u32> {
    let mut state = seed;
    (0..256)
        .map(|level| {
            let x = level as f32;
            let main = (-((x - centre) / width).powi(2) / 2.0).exp();
            let side = 0.5 * (-((x - centre - 60.0) / (width * 1.6)).powi(2) / 2.0).exp();
            ((main + side) * 3000.0 * (0.85 + 0.3 * noise(&mut state))) as u32 + 8
        })
        .collect()
}

/// One generated channel, owned.
struct Curve {
    key: &'static str,
    label: String,
    ink: ChannelColor,
    bins: Vec<u32>,
    color_channel: bool,
}

/// The mode's channels in list order, the composite first and Luminosity last. `shift` moves
/// every hill, which is how the adjusted histogram differs from the original.
fn curves(mode: ModeArg, words: &ScopeStoryLabels, names: &ChannelsLabels, shift: f32) -> Vec<Curve> {
    let colours: Vec<(&'static str, &String, ChannelColor, f32)> = match mode {
        ModeArg::Rgb => vec![
            ("red", &names.red, ChannelColor::Red, 150.0),
            ("green", &names.green, ChannelColor::Green, 110.0),
            ("blue", &names.blue, ChannelColor::Blue, 70.0),
        ],
        ModeArg::Cmyk => vec![
            ("cyan", &names.cyan, ChannelColor::Cyan, 70.0),
            ("magenta", &names.magenta, ChannelColor::Magenta, 100.0),
            ("yellow", &names.yellow, ChannelColor::Yellow, 130.0),
            ("black", &words.black, ChannelColor::Neutral, 40.0),
        ],
    };
    let mut channels: Vec<Curve> = colours
        .iter()
        .zip(1u32..)
        .map(|((key, label, ink, centre), seed)| Curve {
            key,
            label: (*label).clone(),
            ink: *ink,
            bins: hill(seed, (centre + shift).clamp(0.0, 255.0), 34.0),
            color_channel: true,
        })
        .collect();
    let count = channels.len() as u32;
    let composite: Vec<u32> = (0..256).map(|level| channels.iter().map(|c| c.bins[level]).sum::<u32>() / count).collect();
    let (key, label) = match mode {
        ModeArg::Rgb => ("rgb", words.rgb.clone()),
        ModeArg::Cmyk => ("cmyk", words.cmyk.clone()),
    };
    channels.insert(0, Curve { key, label, ink: ChannelColor::Neutral, bins: composite, color_channel: false });
    let luminosity = Curve {
        key: "luminosity",
        label: words.luminosity.clone(),
        ink: ChannelColor::Neutral,
        bins: hill(9, (105.0 + shift).clamp(0.0, 255.0), 46.0),
        color_channel: false,
    };
    channels.push(luminosity);
    channels
}

fn scope_channels(curves: &[Curve]) -> Vec<ScopeChannel<'_>> {
    curves.iter().map(|c| ScopeChannel { key: c.key, label: &c.label, ink: c.ink, bins: &c.bins, color_channel: c.color_channel }).collect()
}

fn overlay_of(mode: ModeArg) -> ChannelOverlay {
    match mode {
        ModeArg::Rgb => ChannelOverlay::Light,
        ModeArg::Cmyk => ChannelOverlay::Ink,
    }
}

/// What a histogram story adds to the plain panel.
#[derive(Clone, Copy, Default)]
struct Look {
    colors: bool,
    original: bool,
    stale: bool,
    sources: bool,
}

/// A histogram panel over generated counts. Its state lives in egui memory under `key`.
fn histogram(ui: &mut Ui, key: &str, mode: ModeArg, settings: ScopeSettings, look: Look) {
    with_values(ui, key, [ScopePanelState { settings, range: None }], |ui, state| {
        let words = ScopeStoryLabels::of(ui);
        let names = ChannelsLabels::of(ui);
        let entire_image = ScopePanelLabels::of(ui).entire_image;
        let adjusted = curves(mode, &words, &names, if look.original { 25.0 } else { 0.0 });
        let before = curves(mode, &words, &names, 0.0);
        let channels = scope_channels(&adjusted);
        let originals = scope_channels(&before);
        let mut set = HistogramSet::new(&channels).stale(look.stale);
        if look.original {
            set = set.original(Some(&originals));
        }
        if look.colors {
            set = set.colors(overlay_of(mode));
        }
        let sources = [ScopeSource { key: "image", label: &entire_image }, ScopeSource { key: "layer", label: &words.selected_layer }];
        let sources: &[ScopeSource<'_>] = if look.sources { &sources } else { &[] };
        let frame = ScopeFrame::histogram(set);
        ScopePanel::new(key, &mut state[0]).frame(&frame).sources(sources).show(ui);
    });
}

fn view(view: ScopeView) -> ScopeSettings {
    ScopeSettings { view, ..ScopeSettings::default() }
}

fn show_compact(ui: &mut Ui, args: &mut ModeArgs) {
    histogram(ui, "scopes-compact", args.mode, view(ScopeView::Compact), Look::default());
}

fn show_expanded(ui: &mut Ui, args: &mut ModeArgs) {
    histogram(ui, "scopes-expanded", args.mode, view(ScopeView::Expanded), Look { sources: true, ..Look::default() });
}

fn show_all_channels(ui: &mut Ui, args: &mut ModeArgs) {
    histogram(ui, "scopes-all-channels", args.mode, view(ScopeView::AllChannels), Look::default());
}

fn show_colors_overlay(ui: &mut Ui, _args: &mut NoArgs) {
    let colors = ScopeSettings { channel: COLORS_CHANNEL.to_owned(), ..view(ScopeView::Expanded) };
    histogram(ui, "scopes-colors-light", ModeArg::Rgb, colors.clone(), Look { colors: true, ..Look::default() });
    ui.separator();
    histogram(ui, "scopes-colors-ink", ModeArg::Cmyk, colors, Look { colors: true, ..Look::default() });
}

fn show_adjusted(ui: &mut Ui, args: &mut ModeArgs) {
    let settings = ScopeSettings { show_channels_in_color: true, ..view(ScopeView::Expanded) };
    histogram(ui, "scopes-adjusted", args.mode, settings, Look { original: true, stale: true, ..Look::default() });
}

/// The zone controls the host maps edits to, in the order of [`ToneZone::CAMERA_RAW`].
#[derive(Clone)]
struct ZoneHost {
    state: ScopePanelState,
    values: [f32; 5],
    /// The control's value at the start of a drag, which a cancel restores.
    before: Option<f32>,
}

/// The range of a zone's control: Exposure is in stops, the others in percent.
fn limit(key: &str) -> f32 {
    if key == "exposure" { 5.0 } else { 100.0 }
}

fn zone_text(key: &str, value: f32) -> String {
    if key == "exposure" { format!("{value:+.2}") } else { format!("{value:+.0}") }
}

fn show_tone_zones(ui: &mut Ui, _args: &mut NoArgs) {
    let start = ZoneHost { state: ScopePanelState { settings: view(ScopeView::Expanded), range: None }, values: [0.0; 5], before: None };
    with_values(ui, "scopes-tone-zones", [start], |ui, hosts| {
        let host = &mut hosts[0];
        let words = ScopeStoryLabels::of(ui);
        let names = ChannelsLabels::of(ui);
        let curves = curves(ModeArg::Rgb, &words, &names, 0.0);
        let channels = scope_channels(&curves);
        let frame = ScopeFrame::histogram(HistogramSet::new(&channels));
        let zones = ToneZone::CAMERA_RAW;
        let texts: Vec<String> = zones.iter().zip(host.values).map(|(zone, value)| zone_text(zone.key, value)).collect();
        let out = ScopePanel::new("scopes-tone-zones", &mut host.state).frame(&frame).zones(&zones, &texts).show(ui);
        let Some(edit) = out.zone else { return };
        let max = limit(edit.key);
        let value = &mut host.values[edit.zone];
        match edit.change {
            ZoneChange::Drag(EditEvent::Begin { .. }) => host.before = Some(*value),
            ZoneChange::Drag(EditEvent::Update { value: share, .. }) => {
                if let Some(before) = host.before {
                    *value = (before + share * max * 2.0).clamp(-max, max);
                }
            }
            ZoneChange::Drag(EditEvent::Commit { value: share, .. }) => {
                if let Some(before) = host.before.take() {
                    *value = (before + share * max * 2.0).clamp(-max, max);
                }
            }
            ZoneChange::Drag(EditEvent::Cancel { .. }) => {
                if let Some(before) = host.before.take() {
                    *value = before;
                }
            }
            ZoneChange::Drag(_) => {}
            ZoneChange::Step(steps) => *value = (*value + steps as f32 * max / 100.0).clamp(-max, max),
            ZoneChange::Reset => *value = 0.0,
            ZoneChange::SetValue(set) => *value = (set as f32).clamp(-max, max),
        }
    });
}

/// Generated pixels in a skin cluster, a sky and some foliage, with a few saturated ones.
fn scope_pixels() -> Vec<[f32; 4]> {
    let mut state = 5;
    let mut around = |centre: [f32; 3], spread: f32| {
        let mut next = |c: f32| (c + (noise(&mut state) - 0.5) * spread).clamp(0.0, 1.0);
        [next(centre[0]), next(centre[1]), next(centre[2]), 1.0]
    };
    let mut pixels = Vec::new();
    for _ in 0..1500 {
        pixels.push(around([0.88, 0.66, 0.54], 0.12));
    }
    for _ in 0..800 {
        pixels.push(around([0.35, 0.55, 0.9], 0.2));
    }
    for _ in 0..600 {
        pixels.push(around([0.3, 0.65, 0.25], 0.25));
    }
    for _ in 0..100 {
        pixels.push(around([0.95, 0.1, 0.1], 0.1));
    }
    pixels
}

/// What a story with two args and a panel keeps: the state, and the args last copied into it, so
/// a change on either side wins once.
#[derive(Clone)]
struct Toggles {
    state: ScopePanelState,
    applied: Option<(bool, bool)>,
}

fn show_vectorscope(ui: &mut Ui, args: &mut VectorscopeArgs) {
    let settings = ScopeSettings { vectorscope: true, ..view(ScopeView::Compact) };
    let start = Toggles { state: ScopePanelState { settings, range: None }, applied: None };
    with_values(ui, "scopes-vectorscope", [start], |ui, hosts| {
        let host = &mut hosts[0];
        let now = (args.red_right, args.skin_line);
        if host.applied == Some(now) {
            args.red_right = host.state.settings.red_right;
            args.skin_line = host.state.settings.skin_tone;
        } else {
            host.state.settings.red_right = args.red_right;
            host.state.settings.skin_tone = args.skin_line;
        }
        host.applied = Some((args.red_right, args.skin_line));

        let words = ScopeStoryLabels::of(ui);
        let names = ChannelsLabels::of(ui);
        let curves = curves(ModeArg::Rgb, &words, &names, 0.0);
        let channels = scope_channels(&curves);
        let counts = Vectorscope::from_rgba(&scope_pixels());
        let (hue, saturation) = hue_saturation([0.88, 0.66, 0.54]);
        let scope = VectorscopeFrame { counts: &counts.counts, hues: HUES, saturations: SATURATIONS, hover: Some([hue, saturation]) };
        let frame = ScopeFrame::histogram(HistogramSet::new(&channels)).vectorscope(scope);
        ScopePanel::new("scopes-vectorscope", &mut host.state).frame(&frame).show(ui);
    });
}

const IMAGE_SIZE: [usize; 2] = [96, 64];

/// A generated picture in straight RGBA: a ramp whose red clips at both ends, a green ramp that
/// does not, and a blue that fades to black, so shadows clip in blue and highlights in red.
fn image_pixels() -> Vec<[f32; 4]> {
    let [width, height] = IMAGE_SIZE;
    (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as f32 / (width - 1) as f32, (i / width) as f32 / (height - 1) as f32);
            [x * 1.5 - 0.25, 0.1 + 0.8 * y, (1.0 - x) * 1.5 - 0.25 + 0.2 * y, 1.0]
        })
        .map(|[r, g, b, a]: [f32; 4]| [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), a])
        .collect()
}

/// The counts of one channel of `pixels`.
fn counted(pixels: &[[f32; 4]], channel: usize) -> Vec<u32> {
    let mut bins = vec![0u32; 256];
    for pixel in pixels {
        bins[(pixel[channel] * 255.0).round() as usize] += 1;
    }
    bins
}

fn show_clipping(ui: &mut Ui, args: &mut ClippingArgs) {
    let start = Toggles { state: ScopePanelState { settings: view(ScopeView::Compact), range: None }, applied: None };
    with_values(ui, "scopes-clipping", [start], |ui, hosts| {
        let host = &mut hosts[0];
        let now = (args.shadows, args.highlights);
        if host.applied == Some(now) {
            args.shadows = host.state.settings.shadow_warning;
            args.highlights = host.state.settings.highlight_warning;
        } else {
            host.state.settings.shadow_warning = args.shadows;
            host.state.settings.highlight_warning = args.highlights;
        }
        host.applied = Some((args.shadows, args.highlights));

        let pixels = image_pixels();
        let Some(mask) = ClippingMask::from_rgba(IMAGE_SIZE, &pixels) else { return };
        let words = ScopeStoryLabels::of(ui);
        let names = ChannelsLabels::of(ui);
        let curves: Vec<Curve> = [
            ("rgb", &words.rgb, ChannelColor::Neutral),
            ("red", &names.red, ChannelColor::Red),
            ("green", &names.green, ChannelColor::Green),
            ("blue", &names.blue, ChannelColor::Blue),
        ]
        .into_iter()
        .enumerate()
        .map(|(channel, (key, label, ink))| {
            let bins = if channel == 0 {
                let parts: Vec<Vec<u32>> = (0..3).map(|c| counted(&pixels, c)).collect();
                (0..256).map(|level| parts.iter().map(|p| p[level]).sum::<u32>() / 3).collect()
            } else {
                counted(&pixels, channel - 1)
            };
            Curve { key, label: label.clone(), ink, bins, color_channel: channel > 0 }
        })
        .collect();
        let channels = scope_channels(&curves);
        let any = mask.any();
        let frame = ScopeFrame::histogram(HistogramSet::new(&channels)).clipping(ClippingFrame { shadows: any.shadows, highlights: any.highlights });
        ScopePanel::new("scopes-clipping", &mut host.state).frame(&frame).show(ui);

        let [width, height] = IMAGE_SIZE;
        let size = vec2(ui.available_width(), ui.available_width() * height as f32 / width as f32);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let picture = ColorImage::new(
            IMAGE_SIZE,
            pixels.iter().map(|p| Color32::from_rgb((p[0] * 255.0) as u8, (p[1] * 255.0) as u8, (p[2] * 255.0) as u8)).collect(),
        );
        let ctx = ui.ctx().clone();
        let cache = TextureCache::<()>::in_context(&ctx, "scopes-clipping-picture", TextureCacheOptions::frames().filter(TextureOptions::NEAREST));
        let texture = cache.get_or_put(&ctx, (), Revision::of(&0u8), || picture);
        let painter = ui.painter();
        painter.image(texture.id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE); // design: allow(identity tint)
        ClippingOverlay::new("scopes-clipping-overlay").show(&ctx, painter, rect, &mask, 0, host.state.settings.clipping_mode(None));
    });
}
