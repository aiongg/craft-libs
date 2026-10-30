//! The colour picker, [`ColorPicker`], and its parts: [`ColorArea`], the 2D field of two
//! components of a colour, and [`ColorStrip`], the strip of the third, as PhotoCraft's Color
//! Picker field and component strip.
//!
//! Both edit a colour as one [`ColorModel`]'s values ([`ModelValues`]), the model and components
//! an [`AreaMode`] chooses, and report through the edit lifecycle ([`crate::edit`]): a press sets
//! the value under the pointer and a drag is one gesture, each key is a
//! [`Set`](crate::edit::EditEvent::Set). Working in model values keeps what sRGB can't say: a
//! drag to grey keeps the hue, a drag to black keeps hue and saturation.

mod area;
mod disc;
mod disc_stories;
#[cfg(test)]
mod disc_tests;
mod full_stories;
#[cfg(test)]
mod full_tests;
mod hex;
#[cfg(test)]
mod hex_tests;
mod pair;
mod pair_stories;
#[cfg(test)]
mod pair_tests;
mod palette;
mod picker;
#[cfg(test)]
mod picker_tests;
pub(crate) mod popover;
#[cfg(test)]
mod popover_tests;
#[cfg(test)]
mod quantize_tests;
mod ramp;
mod ramp_stories;
#[cfg(test)]
mod ramp_tests;
mod strip;
#[cfg(test)]
mod tests;
mod wheel;
mod wheel_stories;
#[cfg(test)]
mod wheel_tests;

use craft_theme::{ColorModel, HsbModel, LabModel, ModelValues, OklchModel, RgbModel};
use egui::{Color32, Stroke, Ui, Vec2, vec2};

pub use area::ColorArea;
pub use disc::{ColorDisc, ColorDiscResponse, ColorDiscStyle, DiscMove};
pub use hex::HexField;
pub use pair::{PairWell, SwatchPair, SwatchPairResponse, SwatchPairStyle, Well};
pub use palette::SwatchPalette;
pub(crate) use picker::WORDS as PICKER_WORDS;
pub use picker::{ColorPicker, ColorPickerLayout, ColorPickerResponse, ColorWarning, ColorWarningChip, PickedColor};
pub use popover::{ColorPopover, ColorPopoverResponse};
pub use ramp::{ColorRamp, ColorRampResponse, RampKind, RampPick};
pub use strip::ColorStrip;
pub use wheel::{ColorWheel, ColorWheelResponse, ColorWheelStyle, WheelGeometry, WheelPart};

use crate::controls::slider::{self, Scale};
use crate::edit::{self, EditEvent};
use crate::localized::LocaleNumbers;
use crate::primitives::{Content, Gap, Row, Stack, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// What the area and the strip show: the strip's component and the area's two, across (growing to
/// the right) and down (growing upwards). The nine modes of PhotoCraft's Color Picker, with
/// Photoshop's axes, and three for OKLCH laid out as the HSB ones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AreaMode {
    /// Hue in the strip; saturation across, brightness down (HSB).
    #[default]
    Hue,
    /// Saturation in the strip; hue across, brightness down (HSB).
    Saturation,
    /// Brightness in the strip; hue across, saturation down (HSB).
    Brightness,
    /// Red in the strip; blue across, green down (RGB).
    Red,
    /// Green in the strip; blue across, red down (RGB).
    Green,
    /// Blue in the strip; red across, green down (RGB).
    Blue,
    /// Lab lightness in the strip; a across, b down.
    LabL,
    /// Lab a in the strip; b across, lightness down.
    LabA,
    /// Lab b in the strip; a across, lightness down.
    LabB,
    /// OKLCH lightness in the strip; hue across, chroma down.
    OklchL,
    /// OKLCH chroma in the strip; hue across, lightness down.
    OklchC,
    /// OKLCH hue in the strip; chroma across, lightness down.
    OklchH,
}

impl AreaMode {
    /// PhotoCraft's nine modes, in its radio order: H, S, B, R, G, B, L, a, b.
    pub const PHOTOSHOP: [AreaMode; 9] = [
        AreaMode::Hue,
        AreaMode::Saturation,
        AreaMode::Brightness,
        AreaMode::Red,
        AreaMode::Green,
        AreaMode::Blue,
        AreaMode::LabL,
        AreaMode::LabA,
        AreaMode::LabB,
    ];

    /// The OKLCH modes: L, C, H.
    pub const OKLCH: [AreaMode; 3] = [AreaMode::OklchL, AreaMode::OklchC, AreaMode::OklchH];

    /// Every mode: PhotoCraft's nine, then OKLCH's three.
    pub const ALL: [AreaMode; 12] = [
        AreaMode::Hue,
        AreaMode::Saturation,
        AreaMode::Brightness,
        AreaMode::Red,
        AreaMode::Green,
        AreaMode::Blue,
        AreaMode::LabL,
        AreaMode::LabA,
        AreaMode::LabB,
        AreaMode::OklchL,
        AreaMode::OklchC,
        AreaMode::OklchH,
    ];

    /// The area's accessible name, English, its two components across and down: the key a host
    /// translates ("Saturation and brightness").
    pub fn name(self) -> &'static str {
        match self {
            AreaMode::Hue => "Saturation and brightness",
            AreaMode::Saturation => "Hue and brightness",
            AreaMode::Brightness => "Hue and saturation",
            AreaMode::Red => "Blue and green",
            AreaMode::Green => "Blue and red",
            AreaMode::Blue => "Red and green",
            AreaMode::LabL => "Green-red and blue-yellow",
            AreaMode::LabA => "Blue-yellow and lightness",
            AreaMode::LabB => "Green-red and lightness",
            AreaMode::OklchL => "Hue and chroma",
            AreaMode::OklchC => "Hue and lightness",
            AreaMode::OklchH => "Chroma and lightness",
        }
    }

    /// The model whose values the area and strip edit.
    pub fn model(self) -> &'static dyn ColorModel {
        match self {
            AreaMode::Hue | AreaMode::Saturation | AreaMode::Brightness => &HsbModel,
            AreaMode::Red | AreaMode::Green | AreaMode::Blue => &RgbModel,
            AreaMode::LabL | AreaMode::LabA | AreaMode::LabB => &LabModel,
            AreaMode::OklchL | AreaMode::OklchC | AreaMode::OklchH => &OklchModel,
        }
    }

    /// The strip's channel, an index into the model's channels.
    pub fn strip(self) -> usize {
        match self {
            AreaMode::Hue | AreaMode::Red | AreaMode::LabL | AreaMode::OklchL => 0,
            AreaMode::Saturation | AreaMode::Green | AreaMode::LabA | AreaMode::OklchC => 1,
            AreaMode::Brightness | AreaMode::Blue | AreaMode::LabB | AreaMode::OklchH => 2,
        }
    }

    /// The area's channels, across and down.
    pub fn area(self) -> [usize; 2] {
        match self {
            AreaMode::Hue => [1, 2],
            AreaMode::Saturation => [0, 2],
            AreaMode::Brightness => [0, 1],
            AreaMode::Red => [2, 1],
            AreaMode::Green => [2, 0],
            AreaMode::Blue => [0, 1],
            AreaMode::LabL => [1, 2],
            AreaMode::LabA => [2, 0],
            AreaMode::LabB => [1, 0],
            AreaMode::OklchL => [2, 1],
            AreaMode::OklchC => [2, 0],
            AreaMode::OklchH => [1, 0],
        }
    }

    /// The values the strip's colours are drawn with: the colour's own, except that the HSB hue
    /// strip shows pure hues (full saturation and brightness), as Photoshop's.
    fn strip_base(self, values: &ModelValues) -> ModelValues {
        let mut base = *values;
        if self == AreaMode::Hue {
            base.values[1] = 100.0;
            base.values[2] = 100.0;
        }
        base
    }
}

/// The area's and the strip's sizes and colours, from [`ColorArea::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorPartsStyle {
    /// Side of the Full picker's area, and the strip's default height: `color_picker.area`.
    pub area: f32,
    /// Height of the Compact picker's area: `color_picker.area_height`.
    pub area_height: f32,
    /// Width of the strip in the Full picker, its default: `color_picker.strip_full`.
    pub strip_full: f32,
    /// Width of the strip in the Compact picker: `color_picker.strip`.
    pub strip: f32,
    /// The edge round the area and the strip, outside them: one point of `line.control_edge`.
    pub edge: Stroke,
    /// The area marker's inner circle: `color_picker.marker_inner_radius`, a
    /// `color_picker.marker_inner_width` stroke of `color_picker.marker` (white).
    pub marker_inner_radius: f32,
    pub marker_inner: Stroke,
    /// The marker's outer ring: `color_picker.marker_outer_radius`, a
    /// `color_picker.marker_outer_width` stroke of `color_picker.marker_halo` (black at alpha
    /// 160). One of the two rings reads on any colour, and neither tints it.
    pub marker_outer_radius: f32,
    pub marker_outer: Stroke,
    /// A strip pointer triangle, its depth across and its height: `color_picker.pointer_width`
    /// and `color_picker.pointer_height`.
    pub pointer: Vec2,
    /// The pointers' fill: `text.normal`, outlined in the strip's edge.
    pub pointer_color: Color32,
}

fn parts_style(tokens: &UiTokens) -> ColorPartsStyle {
    let t = &tokens.component.color_picker;
    let c = &tokens.color;
    ColorPartsStyle {
        area: t.area,
        area_height: t.area_height,
        strip_full: t.strip_full,
        strip: t.strip,
        edge: Stroke::new(1.0, c.line.control_edge.to_egui()),
        marker_inner_radius: t.marker_inner_radius,
        marker_inner: Stroke::new(t.marker_inner_width, t.marker.to_egui()),
        marker_outer_radius: t.marker_outer_radius,
        marker_outer: Stroke::new(t.marker_outer_width, t.marker_halo.to_egui()),
        pointer: vec2(t.pointer_width, t.pointer_height),
        pointer_color: c.text.normal.to_egui(),
    }
}

/// A disabled part's edge, marker and pointers are faded; its colours are not.
fn faded(color: Color32, enabled: bool) -> Color32 {
    if enabled { color } else { color.gamma_multiply(0.5) }
}

fn faded_stroke(stroke: Stroke, enabled: bool) -> Stroke {
    Stroke::new(stroke.width, faded(stroke.color, enabled))
}

/// The scale of a model channel: its range, snapping to its step.
fn scale_of(model: &dyn ColorModel, channel: usize) -> Scale {
    let ch = model.channels()[channel];
    Scale::new(&(f64::from(ch.min)..=f64::from(ch.max)), Some(f64::from(ch.step)))
}

/// Where `value` lies in `scale`, from its start (0) to its end (1).
fn fraction(scale: &Scale, value: f32) -> f32 {
    let span = scale.max - scale.min;
    if span > 0.0 { ((scale.place(f64::from(value)) - scale.min) / span) as f32 } else { 0.0 }
}

/// The value at `t` (0 at the start, 1 at the end) of `scale`, on its nearest step.
fn value_at(scale: &Scale, t: f32) -> f32 {
    scale.snap(scale.min + f64::from(t.clamp(0.0, 1.0)) * (scale.max - scale.min)) as f32
}

/// Cells per side of a quantised field, and rows of a quantised strip: fine enough that the
/// level edges follow the colour model, coarse enough to draw every frame.
const QUANTIZE_CELLS: usize = 64;

/// `values` with each red, green and blue channel of their colour on the nearest of `levels`
/// evenly spaced levels (at least two) from 0 to 1, in the model's values again (a web-safe
/// colour has 6). The other components are what the snapped colour gives.
fn quantized(model: &dyn ColorModel, values: &ModelValues, levels: usize) -> ModelValues {
    model.from_srgb(snapped(model.to_srgb(&values.values), levels))
}

/// `color` with each channel on the nearest of `levels` levels.
fn snapped(color: craft_theme::Srgba, levels: usize) -> craft_theme::Srgba {
    let top = (levels.max(2) - 1) as f32;
    let snap = |v: f32| (v.clamp(0.0, 1.0) * top).round() / top;
    craft_theme::Srgba::new(snap(color.r), snap(color.g), snap(color.b), color.a)
}

/// A component as a screen reader reads it: "Saturation 40 %", "Hue 120°", "Lightness 50".
fn spoken(name: &str, model: &dyn ColorModel, channel: usize, value: f32) -> String {
    let ch = model.channels()[channel];
    let factor = 10f32.powi(i32::from(ch.decimals));
    // Adding zero turns a rounded -0 into 0.
    let value = (value * factor).round() / factor + 0.0;
    let number = format!("{value:.*}", usize::from(ch.decimals));
    match ch.unit {
        "" => format!("{name} {number}"),
        "°" => format!("{name} {number}°"),
        unit => format!("{name} {number} {unit}"),
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] =
    &["ColorArea", "ColorDisc", "ColorPicker", "ColorPopover", "ColorRamp", "ColorWheel", "HexField", "SwatchPair", "SwatchPalette"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    #[choice(name = "H")]
    H,
    #[choice(name = "S")]
    S,
    #[choice(name = "B")]
    B,
    #[choice(name = "R")]
    R,
    #[choice(name = "G")]
    G,
    #[choice(name = "B (blue)")]
    BBlue,
    #[choice(name = "L")]
    L,
    #[choice(name = "a")]
    A,
    #[choice(name = "b")]
    Bb,
    #[choice(name = "OKLCH L")]
    OklchL,
    #[choice(name = "OKLCH C")]
    OklchC,
    #[choice(name = "OKLCH H")]
    OklchH,
}

impl ModeArg {
    /// The mode at this option's position in [`AreaMode::PHOTOSHOP`] then [`AreaMode::OKLCH`].
    fn mode(self) -> AreaMode {
        AreaMode::PHOTOSHOP.into_iter().chain(AreaMode::OKLCH).nth(self as usize).unwrap_or_default()
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PartsArgs {
    mode: ModeArg,
    compact: bool,
    disabled: bool,
    #[arg(name = "web colours only")]
    web_colours_only: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CompactArgs {
    alpha: bool,
    eyedropper: bool,
    #[arg(name = "dynamic tracks")]
    dynamic_tracks: bool,
    #[arg(name = "web colours only")]
    web_colours_only: bool,
    disabled: bool,
    #[arg(range = 160.0..=480.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PopoverArgs {
    size: SizeArg,
    alpha: bool,
    eyedropper: bool,
    disabled: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum DigitsArg {
    #[choice(name = "2")]
    Two,
    #[choice(name = "1")]
    One,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct HexFieldArgs {
    digits: DigitsArg,
    alpha: bool,
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PaletteArgs {
    size: SizeArg,
    #[arg(range = 60.0..=400.0)]
    width: f32,
    disabled: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    let mut stories = vec![
        typed_story!(
            "composites/color-area/parts",
            Level::Composites,
            "ColorArea",
            "Area and strip",
            PartsArgs { mode: ModeArg::H, compact: false, disabled: false, web_colours_only: false },
            show_parts
        ),
        typed_story!(
            "composites/color/compact",
            Level::Composites,
            "ColorPicker",
            "Compact",
            CompactArgs { alpha: false, eyedropper: true, dynamic_tracks: true, web_colours_only: false, disabled: false, width: 260.0 },
            show_compact
        ),
        typed_story!(
            "composites/color/popover",
            Level::Composites,
            "ColorPopover",
            "Popover",
            PopoverArgs { size: SizeArg::Md, alpha: true, eyedropper: true, disabled: false },
            show_popover
        ),
        typed_story!(
            "composites/color/hex-field",
            Level::Composites,
            "HexField",
            "Hex field",
            HexFieldArgs { digits: DigitsArg::Two, alpha: false, disabled: false },
            show_hex_field
        ),
        typed_story!(
            "composites/color/palette",
            Level::Composites,
            "SwatchPalette",
            "Palette",
            PaletteArgs { size: SizeArg::Md, width: 200.0, disabled: false },
            show_palette
        ),
    ];
    stories.extend(full_stories::stories());
    stories.extend(pair_stories::stories());
    stories.extend(wheel_stories::stories());
    stories.extend(ramp_stories::stories());
    stories.extend(disc_stories::stories());
    stories
}

/// A Compact picker editing one colour as a panel does: every event applied, a cancel puts the
/// colour from before the gesture back; the eyedropper "samples" orange at once.
fn show_compact(ui: &mut Ui, args: &mut CompactArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let start = craft_theme::Srgba::new(0.2, 0.55, 0.8, 1.0);
    slider::with_values(ui, "color-compact", [(start, start, String::new())], |ui, state| {
        let (color, before, last) = &mut state[0];
        ui.set_width(args.width.min(ui.available_width()));
        let mut web = args.web_colours_only;
        let mut picker = ColorPicker::new("color-compact", "Foreground colour", *color, &numbers)
            .web_colors(&mut web)
            .alpha(args.alpha)
            .dynamic_tracks(args.dynamic_tracks)
            .enabled(!args.disabled);
        if args.eyedropper {
            picker = picker.eyedropper("Pick screen color");
        }
        let picked = picker.show(ui);
        if picked.eyedropper_requested {
            ColorPicker::sampled(ui.ctx(), picked.edit.response.id, craft_theme::Srgba::rgb(1.0, 0.6, 0.0));
        }
        if let Some(event) = &picked.edit.event {
            match event {
                EditEvent::Begin { .. } => *before = *color,
                EditEvent::Cancel { .. } => *color = *before,
                _ => {}
            }
            if let Some(p) = event.value() {
                *color = p.color;
            }
            *last = edit::describe(&event.clone().map(|p| (p.color.to_rgba().to_hex(), p.model.map(|m| m.model))));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}

/// The levels of a web colour's channels: 0x00, 0x33 ... 0xFF.
const WEB_LEVELS: usize = 6;

/// The nearest web colour.
fn web_color(c: craft_theme::Srgba) -> craft_theme::Srgba {
    snapped(c, WEB_LEVELS)
}

/// An area and a strip in a mode, editing one colour that keeps the values last edited, as a
/// picker keeps them.
fn show_parts(ui: &mut Ui, args: &mut PartsArgs) {
    let mode = args.mode.mode();
    let (compact, enabled) = (args.compact, !args.disabled);
    let web = args.web_colours_only;
    let model = mode.model();
    let start = craft_theme::Srgba::rgb(0.2, 0.55, 0.8);
    slider::with_values(ui, "color-parts", [(start, None::<ModelValues>)], |ui, state| {
        let (color, kept) = &mut state[0];
        let values = craft_theme::kept_or_converted(model, *color, kept.as_ref());
        let style = ColorArea::style(&Tokens::of(ui));
        let height = if compact { style.area_height } else { style.area };
        let strip_width = if compact { style.strip } else { style.strip_full };
        let names = |i: usize| model.channels()[i].name;
        Stack::new(Gap::Stack).show(ui, |ui| {
            Row::new(Gap::Inline).show(ui, |ui| {
                let mut area = ColorArea::new("color-parts-area", mode.name(), mode, values).size(Vec2::new(style.area, height)).enabled(enabled);
                let mut strip =
                    ColorStrip::new("color-parts-strip", names(mode.strip()), mode, values).size(Vec2::new(strip_width, height)).enabled(enabled);
                if web {
                    (area, strip) = (area.quantize(WEB_LEVELS), strip.quantize(WEB_LEVELS));
                }
                let edits = [area.show(ui), strip.show(ui)];
                for edit in edits {
                    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                        *color = model.to_srgb(&v.values);
                        *kept = Some(*v);
                    }
                }
            });
            let text = model.channels().iter().enumerate().map(|(i, ch)| spoken(ch.label, model, i, values.values[i])).collect::<Vec<_>>().join(", ");
            ui.add(Text::new(format!("{}  {text}", color.to_rgba().to_hex().to_uppercase())));
        });
    });
}

/// A colour popover on a colour that a palette under it also sets; the eyedropper "samples" orange
/// at once.
fn show_popover(ui: &mut Ui, args: &mut PopoverArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let size = args.size.control();
    let start = craft_theme::Srgba::new(0.2, 0.55, 0.8, 1.0);
    slider::with_values(ui, "color-popover", [(start, String::new())], |ui, state| {
        let (color, last) = &mut state[0];
        let (alpha, eyedropper) = (args.alpha, args.eyedropper);
        let popover = ColorPopover::new("color-popover", "Fill colour", *color, &numbers).size(size).enabled(!args.disabled).picker(|picker| {
            let picker = picker.alpha(alpha);
            if eyedropper { picker.eyedropper("Pick screen color") } else { picker }
        });
        let shown = popover.show(ui);
        if shown.eyedropper_requested
            && let Some(id) = shown.picker_id
        {
            ColorPicker::sampled(ui.ctx(), id, craft_theme::Srgba::rgb(1.0, 0.6, 0.0));
        }
        if let Some(event) = &shown.edit.event {
            if let Some(p) = event.value() {
                *color = p.color;
            }
            *last = edit::describe(&event.clone().map(|p| p.color.to_rgba().to_hex()));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}

/// A hex field on a colour; one digit per channel takes web colours as `RGB`.
fn show_hex_field(ui: &mut Ui, args: &mut HexFieldArgs) {
    let start = craft_theme::Rgba::rgb(0x33, 0x66, 0x99);
    slider::with_values(ui, "color-hex-field", [(start, String::new())], |ui, state| {
        let (color, last) = &mut state[0];
        let digits = if args.digits == DigitsArg::One { 1 } else { 2 };
        ui.set_width(160.0);
        let edit = HexField::new("color-hex-field", "Hex colour", *color).digits(digits).alpha(args.alpha).enabled(!args.disabled).show(ui);
        if let Some(event) = &edit.event {
            if let Some(c) = event.value() {
                *color = *c;
            }
            *last = edit::describe(&event.clone().map(|c| c.to_hex()));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}

const PALETTE: [(&str, [u8; 3]); 12] = [
    ("Red", [0xe5, 0x48, 0x4d]),
    ("Orange", [0xf7, 0x6b, 0x15]),
    ("Amber", [0xf5, 0xa5, 0x24]),
    ("Yellow", [0xf5, 0xd9, 0x0a]),
    ("Lime", [0x7a, 0xc5, 0x5c]),
    ("Green", [0x30, 0xa4, 0x6c]),
    ("Teal", [0x12, 0xa5, 0x94]),
    ("Sky", [0x00, 0x9e, 0xe1]),
    ("Blue", [0x3e, 0x63, 0xdd]),
    ("Violet", [0x8e, 0x4e, 0xc6]),
    ("Pink", [0xd6, 0x40, 0x9f]),
    ("Grey", [0x88, 0x8d, 0x91]),
];

/// Twelve swatches wrapping to the width, choosing one colour.
fn show_palette(ui: &mut Ui, args: &mut PaletteArgs) {
    let size = args.size.control();
    let start = craft_theme::Srgba::from(craft_theme::Rgba::rgb(0x3e, 0x63, 0xdd));
    slider::with_values(ui, "color-palette", [(start, String::new())], |ui, state| {
        let (color, last) = &mut state[0];
        ui.set_width(args.width.min(ui.available_width()));
        let swatches = PALETTE.map(|(name, [r, g, b])| (name, craft_theme::Srgba::from(craft_theme::Rgba::rgb(r, g, b))));
        let picked = SwatchPalette::new("color-palette", "Swatches", *color, swatches).size(size).enabled(!args.disabled).show(ui);
        if let Some(event) = &picked.event {
            if let Some(c) = event.value() {
                *color = *c;
            }
            *last = edit::describe(&event.clone().map(|c| c.to_rgba().to_hex()));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}
