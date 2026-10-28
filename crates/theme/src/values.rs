//! Every tunable number of the semantic layer in one place: the ramp step each colour token
//! takes, fixed colours, contrast minimums, Size presets, type roles, text-size factors,
//! elevation and motion choices. The gallery tunes these; nothing else should hold such values.

use super::color::Rgba;
use super::ramp::Mode;
use super::scale::{Duration, Easing, Radius};
use super::theme::{Size, TextSize};
use super::tokens::{
    BarSizes, ColorToken, ControlSizes, Edge, FontSlot, Gaps, IconSizes, ListSizes, Pads, RadiusSet, RailSizes, SizeTokens, SpaceTokens, StatusKind,
    ThumbSizes, TypeStyle, TypeTokens,
};

use Bg::{HaloOver, Token};
use ColorToken as T;

/// Where a colour token's default value comes from: a ramp step, a mix of two steps, a fixed
/// colour, or other tokens. [`explain`](crate::explain) reports it per token.
///
/// The background greys (surfaces, state fills, lines) are [`Anchored`](Self::Anchored) on
/// `surface.panel`, so they keep their curve distance from a panel the theme pins, in the
/// pinned panel's hue and chroma. A state variant keeps its distance from its own base instead
/// when the theme pins that base ([`StateVariant`](Self::StateVariant)). Bevel edges are the
/// panel lit and shaded ([`Lighting`](Self::Lighting)). Text greys stay on the curve and the
/// contrast pass keeps them readable, moving derived text first and pulling an anchored grey
/// back toward its anchor for the pairs text can't pass by moving.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ColorSource {
    /// A step (1 to 12) of the neutral ramp.
    Neutral(usize),
    /// A neutral step (1 to 12) at its curve distance from `surface.panel`: the step's lightness
    /// moved by as much as the panel's value lies from the panel's own step, clamped to 0..=1.
    /// Hue and chroma are the step's when the panel is not pinned, and the pinned panel's when it
    /// is: the pin defines the grey family. Equal to the step when the panel is not pinned. The
    /// distance is a target: when a contrast pair on the token fails and its foreground can't
    /// move (pinned) or can't pass by moving, the contrast pass pulls the token's lightness back
    /// toward the panel's, no further than needed and never past it.
    Anchored(usize),
    /// A state variant of `base` (`state.pressed` of `state.hover`, `state.selected_hover` of
    /// `state.selected`): neutral step `step` at its curve distance from `base`'s step when the
    /// theme pins `base`, in the pinned base's hue and chroma, so a pinned base carries its
    /// variants; the contrast pass pulls it back toward the base. When `base` is not pinned, as
    /// [`Anchored`](Self::Anchored)`(step)`.
    StateVariant {
        /// The state the variant derives from.
        base: ColorToken,
        /// The variant's neutral step.
        step: usize,
    },
    /// `surface.panel` lit (a positive amount) or shaded (a negative one), as a bevel's edges:
    /// its OKLCH lightness moved by the amount, clamped to 0..=1, hue and chroma kept.
    Lighting(f64),
    /// A neutral step with an alpha (0..=1).
    NeutralAlpha(usize, f64),
    /// A step of the accent ramp.
    Accent(usize),
    /// The accent step `from` moved `t` (0..=1) of the way toward step `to`, in OKLab.
    AccentMix {
        /// The step mixed from.
        from: usize,
        /// The step mixed toward.
        to: usize,
        /// How far toward `to`, 0..=1.
        t: f64,
    },
    /// A step of a status ramp.
    Status(StatusKind, usize),
    /// The status step `from` moved `t` (0..=1) of the way toward step `to`, in OKLab.
    StatusMix {
        /// The status whose ramp is mixed.
        kind: StatusKind,
        /// The step mixed from.
        from: usize,
        /// The step mixed toward.
        to: usize,
        /// How far toward `to`, 0..=1.
        t: f64,
    },
    /// A fixed colour.
    Fixed(Rgba),
    /// An OKLCH colour (lightness, chroma, hue), gamut-mapped.
    Oklch(f64, f64, f64),
    /// The final value of another token.
    Same(ColorToken),
    /// A solid fill taken from another token: that token's value when the contrast pass reaches
    /// this token's text rule, which then moves the fill's lightness away from its text as far
    /// as the text needs.
    SolidFrom(ColorToken),
    /// White or black, whichever has the higher lowest contrast on all of these tokens.
    OnSolid(&'static [ColorToken]),
}

/// Converts an alpha in 0..=1 to a channel value.
pub(super) fn alpha(a: f64) -> u8 {
    (a.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// OKLCH hues of the categorical data series.
pub const DATA_HUES: [f64; 8] = [25.0, 70.0, 115.0, 160.0, 205.0, 250.0, 295.0, 340.0];
/// OKLCH lightness of the data series, dark and light mode.
pub const DATA_L: (f64, f64) = (0.72, 0.56);
/// OKLCH chroma of the data series.
pub const DATA_C: f64 = 0.15;

/// The status solids that `text.on_status` must read on.
const STATUS_SOLIDS: &[ColorToken] = &[
    T::StatusGoodSolid,
    T::StatusGoodSolidHover,
    T::StatusGoodSolidPressed,
    T::StatusWarnSolid,
    T::StatusWarnSolidHover,
    T::StatusWarnSolidPressed,
    T::StatusBadSolid,
    T::StatusBadSolidHover,
    T::StatusBadSolidPressed,
    T::StatusInfoSolid,
    T::StatusInfoSolidHover,
    T::StatusInfoSolidPressed,
];

/// The ramp step of a solid fill under the pointer (`accent.hover`, `status.<s>.solid_hover`).
const SOLID_HOVER: usize = 10;
/// A pressed solid fill: step `.0` moved `.2` of the way toward step `.1` (`accent.pressed`,
/// `status.<s>.solid_pressed`).
const SOLID_PRESSED: (usize, usize, f64) = (10, 11, 0.5);

/// How far a bevel's lit edge (`edge.light`) lies above `surface.panel` in OKLCH lightness, in
/// either mode. Windows 2000's #d4d0c8 face (L 0.86) has a white lit edge.
pub const BEVEL_LIT: f64 = 0.15;
/// How far a bevel's shaded edge (`edge.dark`) lies below `surface.panel` in OKLCH lightness, in
/// either mode: Windows 2000's #808080 shade of its #d4d0c8 face.
pub const BEVEL_SHADED: f64 = 0.26;

/// The neutral step of `surface.panel` in `mode`: the step every [`ColorSource::Anchored`] token
/// keeps its distance from.
pub(super) fn panel_step(mode: Mode) -> usize {
    match mode {
        Mode::Dark => 3,
        Mode::Light => 2,
    }
}

/// The default source of `token` in `mode`.
pub(super) fn source(token: ColorToken, mode: Mode) -> ColorSource {
    use ColorSource::{
        Accent, AccentMix, Anchored, Fixed, Lighting, Neutral, NeutralAlpha, Oklch, OnSolid, Same, SolidFrom, StateVariant, Status, StatusMix,
    };
    use StatusKind::{Bad, Good, Info, Warn};
    let dark = mode == Mode::Dark;
    let pick = |d: usize, l: usize| if dark { d } else { l };
    let pickf = |d: f64, l: f64| if dark { d } else { l };
    let series = |i: usize| Oklch(pickf(DATA_L.0, DATA_L.1), DATA_C, DATA_HUES[i]);
    let (pressed_from, pressed_to, pressed_t) = SOLID_PRESSED;
    let pressed = |kind| StatusMix { kind, from: pressed_from, to: pressed_to, t: pressed_t };
    match token {
        T::SurfaceBackground => Anchored(pick(2, 3)),
        T::SurfaceCanvas => Same(T::SurfaceBackground),
        T::SurfacePanel => Neutral(panel_step(mode)),
        T::SurfaceRaised => Anchored(pick(4, 1)),
        T::SurfaceSunken => Anchored(1),
        // Near white in both modes (dark: about 236/255), as macOS, GNOME and PhotoCraft knobs. A
        // knob face is not a background grey, so it stays on the curve.
        T::SurfaceKnob => Neutral(pick(12, 1)),
        T::SurfaceScrim => Fixed(Rgba::BLACK.with_alpha(alpha(pickf(0.5, 0.35)))),
        T::TextNormal => Neutral(12),
        T::TextMuted => Neutral(11),
        T::TextFaint => Neutral(10),
        T::TextDisabled => Neutral(9),
        T::TextOnAccent => OnSolid(&[T::AccentSolid]),
        T::TextOnStrong => Fixed(Rgba::WHITE),
        T::TextOnStatus => OnSolid(STATUS_SOLIDS),
        // `text.normal`'s step, so neutral selections look like the rest of the text.
        T::TextOnSelected => Neutral(12),
        T::LineDivider => Anchored(6),
        T::LineBorder => Anchored(7),
        T::LineControlEdge => Anchored(8),
        // A light knob needs an outline on a light panel; on a dark one its face is the edge.
        T::LineKnobEdge => {
            if dark {
                Fixed(Rgba::TRANSPARENT)
            } else {
                Same(T::LineControlEdge)
            }
        }
        T::LineFocus => Accent(9),
        T::AccentSolid => Accent(9),
        T::AccentHover => Accent(SOLID_HOVER),
        T::AccentPressed => AccentMix { from: pressed_from, to: pressed_to, t: pressed_t },
        T::AccentStrong => SolidFrom(T::AccentSolid),
        T::AccentSoft => Accent(pick(4, 3)),
        T::AccentText => Accent(11),
        T::StatusGoodFg => Status(Good, 11),
        T::StatusGoodBg => Status(Good, 3),
        T::StatusGoodBorder => Status(Good, 7),
        T::StatusGoodSolid => Status(Good, 9),
        T::StatusGoodSolidHover => Status(Good, SOLID_HOVER),
        T::StatusGoodSolidPressed => pressed(Good),
        T::StatusWarnFg => Status(Warn, 11),
        T::StatusWarnBg => Status(Warn, 3),
        T::StatusWarnBorder => Status(Warn, 7),
        T::StatusWarnSolid => Status(Warn, 9),
        T::StatusWarnSolidHover => Status(Warn, SOLID_HOVER),
        T::StatusWarnSolidPressed => pressed(Warn),
        T::StatusBadFg => Status(Bad, 11),
        T::StatusBadBg => Status(Bad, 3),
        T::StatusBadBorder => Status(Bad, 7),
        T::StatusBadSolid => Status(Bad, 9),
        T::StatusBadSolidHover => Status(Bad, SOLID_HOVER),
        T::StatusBadSolidPressed => pressed(Bad),
        T::StatusInfoFg => Status(Info, 11),
        T::StatusInfoBg => Status(Info, 3),
        T::StatusInfoBorder => Status(Info, 7),
        T::StatusInfoSolid => Status(Info, 9),
        T::StatusInfoSolidHover => Status(Info, SOLID_HOVER),
        T::StatusInfoSolidPressed => pressed(Info),
        T::StateHover => Anchored(pick(4, 3)),
        T::StatePressed => StateVariant { base: T::StateHover, step: pick(5, 4) },
        // A selection is a state of its own, not a variant of hover.
        T::StateSelected => Anchored(pick(6, 5)),
        T::StateSelectedHover => StateVariant { base: T::StateSelected, step: pick(7, 6) },
        T::StateDisabledFill => Same(T::SurfacePanel),
        T::OverlayStroke => Accent(9),
        T::OverlayHalo => Fixed(Rgba::BLACK.with_alpha(alpha(0.6))),
        T::OverlayHandleFill => Fixed(Rgba::WHITE),
        T::OverlayHandleStroke => Same(T::OverlayStroke),
        T::OverlayMarquee => Fixed(Rgba::WHITE),
        T::OverlayMarqueeDash => Fixed(Rgba::BLACK),
        // Guides sit on the document, not the panel, so they keep one colour in both modes.
        T::OverlayGuide => Fixed(Rgba::rgb(0x4a, 0xff, 0xff)),
        T::OverlayGuideSelected => Fixed(Rgba::rgb(0x00, 0x00, 0xff)),
        T::OverlaySmartGuide => Fixed(Rgba::rgb(0xff, 0x00, 0xff)),
        // Drawn over the document, not the panel, so not anchored.
        T::OverlayGrid => NeutralAlpha(8, 0.5),
        T::DataSeries0 => series(0),
        T::DataSeries1 => series(1),
        T::DataSeries2 => series(2),
        T::DataSeries3 => series(3),
        T::DataSeries4 => series(4),
        T::DataSeries5 => series(5),
        T::DataSeries6 => series(6),
        T::DataSeries7 => series(7),
        T::DataRed => Fixed(if dark { Rgba::rgb(0xff, 0x50, 0x50) } else { Rgba::rgb(0xd0, 0x20, 0x20) }),
        T::DataGreen => Fixed(if dark { Rgba::rgb(0x50, 0xe0, 0x50) } else { Rgba::rgb(0x20, 0xa0, 0x20) }),
        T::DataBlue => Fixed(if dark { Rgba::rgb(0x50, 0x80, 0xff) } else { Rgba::rgb(0x20, 0x50, 0xd0) }),
        T::DataLuma => Neutral(12),
        T::DataCyan => Fixed(if dark { Rgba::rgb(0x28, 0xc8, 0xe6) } else { Rgba::rgb(0x08, 0x73, 0x88) }),
        T::DataMagenta => Fixed(if dark { Rgba::rgb(0xe1, 0x46, 0xc8) } else { Rgba::rgb(0xb0, 0x24, 0x9a) }),
        T::DataYellow => Fixed(if dark { Rgba::rgb(0xeb, 0xd2, 0x28) } else { Rgba::rgb(0x7a, 0x64, 0x00) }),
        T::DataScope => Fixed(Rgba::rgb(0x28, 0x28, 0x28)),
        T::CaptionCloseHover => Fixed(Rgba::rgb(0xc4, 0x2b, 0x1c)),
        T::CaptionCloseText => Fixed(Rgba::WHITE),
        T::CaptionInactive => Same(T::TextFaint),
        T::EdgeLight => Lighting(BEVEL_LIT),
        T::EdgeDark => Lighting(-BEVEL_SHADED),
    }
}

/// A background in a contrast pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Bg {
    /// A colour token.
    Token(ColorToken),
    /// `overlay.halo` composited over a fixed backdrop.
    HaloOver(Rgba),
}

/// A contrast requirement: each fg must reach the minimum on each bg.
#[derive(Clone, Copy, Debug)]
pub(super) struct PairRule {
    /// Foreground tokens.
    pub fgs: &'static [ColorToken],
    /// Backgrounds they appear on.
    pub bgs: &'static [Bg],
    /// Minimum ratio with standard contrast.
    pub standard: f64,
    /// Minimum ratio with high contrast.
    pub high: f64,
    /// The fg is white or black on solid fills; when neither reaches the minimum, the fills move.
    pub on_solid: bool,
}

const fn rule(fgs: &'static [ColorToken], bgs: &'static [Bg], standard: f64, high: f64) -> PairRule {
    PairRule { fgs, bgs, standard, high, on_solid: false }
}

/// WCAG AA text minimum. The large-text minimum (3:1) applies to 18.66 px semibold or 24 px
/// regular text; no type role is that large, so no rule uses it.
pub const TEXT_MIN: (f64, f64) = (4.5, 7.0);
/// WCAG non-text minimum (control boundaries, focus, indicators, overlays).
pub const NON_TEXT_MIN: (f64, f64) = (3.0, 4.5);

const TEXT_BGS: &[Bg] = &[
    Token(T::SurfaceBackground),
    Token(T::SurfacePanel),
    Token(T::SurfaceRaised),
    Token(T::SurfaceSunken),
    Token(T::StateHover),
    Token(T::StatePressed),
];
const SELECTED_BGS: &[Bg] = &[Token(T::StateSelected), Token(T::StateSelectedHover)];
const CONTROL_BGS: &[Bg] = &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::SurfaceSunken)];
const HALO_BGS: &[Bg] = &[HaloOver(Rgba::BLACK), HaloOver(Rgba::WHITE)];

/// Every contrast pair, in the order the contrast pass fixes them: fills that serve as
/// backgrounds later (state fills, `accent.solid`) come before the text drawn on them.
pub(super) const RULES: &[PairRule] = &[
    rule(&[T::LineFocus], CONTROL_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::AccentSolid], CONTROL_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::TextNormal, T::TextMuted, T::TextFaint], TEXT_BGS, TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::TextOnSelected], SELECTED_BGS, TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::AccentText], CONTROL_BGS, TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusGoodFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusGoodBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusWarnFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusWarnBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusBadFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusBadBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusInfoFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusInfoBg)], TEXT_MIN.0, TEXT_MIN.1),
    PairRule {
        on_solid: true,
        ..rule(&[T::TextOnAccent], &[Token(T::AccentSolid), Token(T::AccentHover), Token(T::AccentPressed)], TEXT_MIN.0, TEXT_MIN.1)
    },
    // After every rule that moves `accent.solid`, which `accent.strong` starts from.
    PairRule { on_solid: true, ..rule(&[T::TextOnStrong], &[Token(T::AccentStrong)], TEXT_MIN.0, TEXT_MIN.1) },
    PairRule {
        on_solid: true,
        ..rule(
            &[T::TextOnStatus],
            &[
                Token(T::StatusGoodSolid),
                Token(T::StatusGoodSolidHover),
                Token(T::StatusGoodSolidPressed),
                Token(T::StatusWarnSolid),
                Token(T::StatusWarnSolidHover),
                Token(T::StatusWarnSolidPressed),
                Token(T::StatusBadSolid),
                Token(T::StatusBadSolidHover),
                Token(T::StatusBadSolidPressed),
                Token(T::StatusInfoSolid),
                Token(T::StatusInfoSolidHover),
                Token(T::StatusInfoSolidPressed),
            ],
            TEXT_MIN.0,
            TEXT_MIN.1,
        )
    },
    rule(&[T::OverlayStroke], HALO_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::OverlayMarquee], &[Token(T::OverlayMarqueeDash)], NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    // A curve editor's curve and points in a channel colour, on its plot (`field.fill`, `surface.sunken`).
    rule(
        &[T::DataRed, T::DataGreen, T::DataBlue, T::DataCyan, T::DataMagenta, T::DataYellow],
        &[Token(T::SurfaceSunken)],
        NON_TEXT_MIN.0,
        NON_TEXT_MIN.1,
    ),
    // Guide and smart-guide colours have no pair: they are the user's choice, as in the apps'
    // preferences, and a bare line on an image can't promise a ratio.
];

/// The Size-dependent values: sizes, spacing, default shape and type roles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Preset {
    pub size: SizeTokens,
    pub space: SpaceTokens,
    pub radius: RadiusSet,
    pub edge: Edge,
    pub typography: TypeTokens,
}

const fn style(size: f32, line_height: f32, font: FontSlot) -> TypeStyle {
    TypeStyle { size, line_height, font }
}

/// Compact (Pro).
pub(super) const COMPACT: Preset = Preset {
    size: SizeTokens {
        control: ControlSizes { sm: 20.0, md: 24.0 },
        icon: IconSizes { sm: 12.0, md: 16.0, lg: 20.0 },
        rail: RailSizes { width: 40.0, button: 30.0 },
        bar: BarSizes { title: 32.0, tab: 26.0, status: 22.0, options: 32.0 },
        panel_header: 28.0,
        list: ListSizes {
            thumb: ThumbSizes { sm: 18.0, md: 24.0, lg: 48.0 },
            thumb_row_min: 32.0,
            indent: 14.0,
            disclosure: 14.0,
            toggle_column: 30.0,
            min_label: 64.0,
            trailing_pad: 10.0,
            rounded: false,
            dividers: true,
        },
    },
    space: SpaceTokens { gap: Gaps { inline: 4.0, stack: 6.0, section: 12.0 }, pad: Pads { control: 6.0, panel: 8.0, dialog: 16.0 } },
    radius: RadiusSet { control: 3.0, popup: 4.0, window: 6.0, panel: 0.0, button: Radius::Full.px(), pill: Radius::Full.px() },
    edge: Edge::Flat,
    typography: TypeTokens {
        caption: style(10.5, 14.0, FontSlot::Ui),
        label: style(11.5, 16.0, FontSlot::Ui),
        body: style(12.0, 16.0, FontSlot::Ui),
        body_strong: style(12.0, 16.0, FontSlot::UiSemibold),
        title: style(13.0, 18.0, FontSlot::UiSemibold),
        heading: style(15.0, 20.0, FontSlot::UiSemibold),
        mono: style(11.5, 16.0, FontSlot::Mono),
    },
};

/// Comfortable (Studio).
pub(super) const COMFORTABLE: Preset = Preset {
    size: SizeTokens {
        control: ControlSizes { sm: 24.0, md: 30.0 },
        icon: IconSizes { sm: 14.0, md: 18.0, lg: 22.0 },
        rail: RailSizes { width: 50.0, button: 36.0 },
        bar: BarSizes { title: 38.0, tab: 30.0, status: 26.0, options: 38.0 },
        panel_header: 34.0,
        list: ListSizes {
            thumb: ThumbSizes { sm: 18.0, md: 34.0, lg: 48.0 },
            thumb_row_min: 40.0,
            indent: 18.0,
            disclosure: 18.0,
            toggle_column: 36.0,
            min_label: 80.0,
            trailing_pad: 12.0,
            rounded: true,
            dividers: false,
        },
    },
    space: SpaceTokens { gap: Gaps { inline: 6.0, stack: 8.0, section: 16.0 }, pad: Pads { control: 8.0, panel: 12.0, dialog: 20.0 } },
    radius: RadiusSet { control: 6.0, popup: 8.0, window: 12.0, panel: 0.0, button: Radius::Full.px(), pill: Radius::Full.px() },
    edge: Edge::Flat,
    typography: TypeTokens {
        caption: style(11.5, 16.0, FontSlot::Ui),
        label: style(12.5, 18.0, FontSlot::Ui),
        body: style(13.0, 18.0, FontSlot::Ui),
        body_strong: style(13.0, 18.0, FontSlot::UiSemibold),
        title: style(14.0, 20.0, FontSlot::UiSemibold),
        heading: style(17.0, 24.0, FontSlot::UiSemibold),
        mono: style(12.5, 18.0, FontSlot::Mono),
    },
};

/// The preset for `size`.
pub(super) const fn preset(size: Size) -> &'static Preset {
    match size {
        Size::Compact => &COMPACT,
        Size::Comfortable => &COMFORTABLE,
    }
}

/// Multiplier on type sizes and line heights per text-size step.
pub const fn text_size_factor(t: TextSize) -> f32 {
    match t {
        TextSize::Small => 0.9,
        TextSize::Default => 1.0,
        TextSize::Large => 1.15,
        TextSize::Larger => 1.3,
    }
}

/// Scaled type sizes round to multiples of this, in points.
pub const TYPE_ROUNDING: f32 = 0.5;

/// Elevation levels (DS-01 primitives) for popups, dialogs and floating surfaces.
pub const ELEVATION_LEVELS: (usize, usize, usize) = (2, 3, 2);
/// Elevation level of a knob on a track (`elevation.knob`) with a flat edge.
pub const KNOB_ELEVATION: usize = 1;
/// How far a disabled knob's face moves from `surface.knob` toward `surface.panel`, 0..=1: far
/// enough to recede in both modes, short of vanishing on the panel in dark mode.
pub const KNOB_DISABLED: f64 = 0.5;
/// Width of the ring round a slider thumb while hovered or pressed, in points.
pub const KNOB_RING: f32 = 2.0;
/// Shadow alpha multiplier in dark mode, where shadows need more weight to show.
pub const DARK_SHADOW_ALPHA_FACTOR: f32 = 1.5;

/// Durations for hover feedback, opening and moving.
pub const MOTION_DURATIONS: (Duration, Duration, Duration) = (Duration::Fast, Duration::Normal, Duration::Normal);
/// Easing for every semantic motion.
pub const MOTION_EASING: Easing = Easing::Standard;

/// Button padding at the standard size, as a multiple of `pad.control`.
pub const BUTTON_PAD_X_MD: f32 = 2.0;
/// Button outline width ([`Stroke::W1`](super::scale::Stroke::W1)); control borders are 1 point.
pub const BUTTON_BORDER_WIDTH: f32 = 1.0;

/// Between a segmented control's track edge and its segments, in points, at every Size.
pub const SEGMENTED_INSET: f32 = 2.0;

/// A text field's default width, in control heights of its size.
pub const FIELD_WIDTH: f32 = 8.0;

/// A number field's default width, in control heights of its size: room for a five-digit number,
/// a short unit and a scrub label inside the field.
pub const NUMBER_FIELD_WIDTH: f32 = 4.0;

/// The control height the Compact property row widths are set for; other sizes scale them by
/// their control height over this.
pub const PROPERTY_BASE_CONTROL: f32 = 24.0;
/// Compact width of a glyph label's column (W, H, X, Y).
pub const PROPERTY_GLYPH_WIDTH: f32 = 22.0;
/// Compact width of a word label's column (Opacity, Angle).
pub const PROPERTY_WORD_WIDTH: f32 = 72.0;
/// Compact least width of a property row's control. A narrower row hides a redundant label, then
/// cuts an essential one; a row too narrow even for the control alone still draws it this wide and
/// overflows.
pub const PROPERTY_MIN_CONTROL: f32 = 36.0;
/// Compact gap between the two columns of a pair row.
pub const PROPERTY_COLUMN_GAP: f32 = 8.0;
/// Compact width of the slot between a pair's columns.
pub const PROPERTY_LINK_WIDTH: f32 = 20.0;

/// The control height the Compact histogram height is set for.
pub const HISTOGRAM_BASE_CONTROL: f32 = 24.0;
/// Compact height of a histogram plot (PhotoCraft's panel).
pub const HISTOGRAM_HEIGHT: f32 = 100.0;
/// Alpha of the RGB ribbons' fills (PhotoCraft's 0.3).
pub const HISTOGRAM_FILL_ALPHA: f32 = 0.3;
/// Alpha of the tone look behind a curve: PhotoCraft's grey 88 / 190 at 0.75 reads as a faint grey.
pub const HISTOGRAM_BACKDROP_ALPHA: f32 = 0.35;
/// Width of the RGB ribbons' outlines.
pub const HISTOGRAM_LINE: f32 = 1.0;
/// Alpha of the marker band (PhotoCraft's `paint_sample`).
pub const HISTOGRAM_MARKER_ALPHA: f32 = 0.4;
/// Width of the marker band (PhotoCraft's `paint_sample`).
pub const HISTOGRAM_MARKER_WIDTH: f32 = 2.0;

/// The control height the Compact menu sizes are set for.
pub const MENU_BASE_CONTROL: f32 = 24.0;
/// Compact least width of a menu (PhotoCraft's).
pub const MENU_MIN_WIDTH: f32 = 220.0;
/// Compact least room between a menu row's label and its shortcut or submenu arrow.
pub const MENU_SHORTCUT_GAP: f32 = 16.0;
/// Compact height of a menu's scroll strip (PhotoCraft's scroll arrows).
pub const MENU_SCROLL_STRIP: f32 = 16.0;

/// The control height the Compact section header sizes are set for.
pub const SECTION_BASE_CONTROL: f32 = 24.0;
/// Compact height of a section header row (PhotoCraft's).
pub const SECTION_ROW_HEIGHT: f32 = 24.0;
/// Compact side of a section chevron.
pub const SECTION_CHEVRON: f32 = 11.0;
/// Compact side of the box a section chevron is centred in.
pub const SECTION_CHEVRON_BOX: f32 = 12.0;
/// Compact distance from a section row's start to its title (the chevron box and its inset).
pub const SECTION_TITLE_OFFSET: f32 = 18.0;

/// The control height the Compact colour picker sizes are set for; other sizes scale them by
/// their control height over this.
pub const COLOR_PICKER_BASE_CONTROL: f32 = 24.0;
/// Compact side of the Full picker's colour area.
pub const COLOR_PICKER_AREA: f32 = 200.0;
/// Compact height of the Compact picker's colour area.
pub const COLOR_PICKER_AREA_HEIGHT: f32 = 120.0;
/// Compact width of the component strip in the Full picker.
pub const COLOR_PICKER_STRIP_FULL: f32 = 18.0;
/// Compact width of the component strip in the Compact picker.
pub const COLOR_PICKER_STRIP: f32 = 14.0;
/// Compact width of the compare swatch.
pub const COLOR_PICKER_COMPARE_WIDTH: f32 = 56.0;
/// Compact height of the compare swatch.
pub const COLOR_PICKER_COMPARE_HEIGHT: f32 = 64.0;
/// Compact radius of the area marker's inner circle.
pub const COLOR_PICKER_MARKER_INNER_RADIUS: f32 = 5.0;
/// Compact stroke width of the marker's inner circle.
pub const COLOR_PICKER_MARKER_INNER_WIDTH: f32 = 1.5;
/// Compact radius of the marker's outer ring.
pub const COLOR_PICKER_MARKER_OUTER_RADIUS: f32 = 6.5;
/// Compact stroke width of the marker's outer ring.
pub const COLOR_PICKER_MARKER_OUTER_WIDTH: f32 = 1.0;
/// The area marker's inner circle: white, as PhotoCraft's, so it shows the colour under it
/// without a tint.
pub const COLOR_PICKER_MARKER: Rgba = Rgba::WHITE;
/// The marker's outer ring: black at alpha 160, as PhotoCraft's.
pub const COLOR_PICKER_MARKER_HALO: Rgba = Rgba([0, 0, 0, 160]);
/// Compact width of a strip pointer triangle.
pub const COLOR_PICKER_POINTER_WIDTH: f32 = 6.0;
/// Compact height of a strip pointer triangle.
pub const COLOR_PICKER_POINTER_HEIGHT: f32 = 8.0;
/// Compact size of a warning icon.
pub const COLOR_PICKER_WARNING_ICON: f32 = 13.0;
/// Compact size of a warning's chip of the nearest passing colour.
pub const COLOR_PICKER_WARNING_CHIP: f32 = 10.0;
/// Compact side of a swatch pair's well.
pub const COLOR_PICKER_PAIR_WELL: f32 = 24.0;
/// Compact offset of a swatch pair's back well from its front well, on both axes.
pub const COLOR_PICKER_PAIR_STEP: f32 = 12.0;
/// Compact width of the gap around a swatch pair's front well.
pub const COLOR_PICKER_PAIR_GAP: f32 = 2.0;
/// Compact width of a swatch pair's ring well.
pub const COLOR_PICKER_PAIR_RING: f32 = 7.0;

/// The control height the Compact curve editor sizes are set for.
pub const CURVE_BASE_CONTROL: f32 = 24.0;
/// Compact least side of a curve editor's graph, bars included (PhotoCraft's Curves).
pub const CURVE_MIN_SIDE: f32 = 120.0;
/// Compact greatest side of a curve editor's graph, bars included.
pub const CURVE_MAX_SIDE: f32 = 300.0;
/// Compact inset of a curve's value area inside its plot, so the end points' handles stay inside.
pub const CURVE_PAD: f32 = 5.0;
/// Compact thickness of a curve editor's gradient bars.
pub const CURVE_BAR: f32 = 8.0;
/// Compact gap between a curve's plot and its gradient bars.
pub const CURVE_BAR_GAP: f32 = 4.0;
/// Compact radius around a curve point's centre that takes a press.
pub const CURVE_HIT: f32 = 9.0;
/// Compact distance outside the plot past which a dragged interior point is removed.
pub const CURVE_DRAG_OFF: f32 = 12.0;
/// Width of the edited curve, in points, at every Size.
pub const CURVE_WIDTH: f32 = 1.5;
/// Width of the other channels' curves behind the edited one, in points, at every Size.
pub const CURVE_OTHER_WIDTH: f32 = 1.0;
/// Opacity of the other channels' curves behind the edited one.
pub const CURVE_OTHER_ALPHA: f32 = 0.8;

/// The control height the Compact gradient editor sizes are set for.
pub const GRADIENT_EDITOR_BASE_CONTROL: f32 = 24.0;
/// Compact height of a gradient editor's ramp (PhotoCraft's stop bar).
pub const GRADIENT_EDITOR_RAMP_HEIGHT: f32 = 24.0;
/// Compact inset of the ramp at both ends of the bar, so an end marker fits.
pub const GRADIENT_EDITOR_INSET: f32 = 8.0;
/// Compact side of a stop marker's square.
pub const GRADIENT_EDITOR_MARKER: f32 = 10.0;
/// Compact height of a stop marker's pointer triangle.
pub const GRADIENT_EDITOR_POINTER: f32 = 5.0;
/// Compact radius of a midpoint diamond.
pub const GRADIENT_EDITOR_DIAMOND: f32 = 3.0;
/// Compact distance from a stop marker that takes a press.
pub const GRADIENT_EDITOR_HIT: f32 = 6.0;
/// Compact distance from a midpoint diamond that takes a press.
pub const GRADIENT_EDITOR_DIAMOND_HIT: f32 = 4.0;
/// Compact distance above or below the ramp past which a dragged stop is torn off.
pub const GRADIENT_EDITOR_TEAR_OFF: f32 = 28.0;
/// Compact thickness of the line that marks a drop place.
pub const GRADIENT_EDITOR_DROP_LINE: f32 = 2.0;

/// Added to a list row's thumbnail side for the row's height.
pub const LIST_THUMB_PAD: f32 = 8.0;
/// Width of a list's drop marker line and outline (PhotoCraft, DesignCraft and docking).
pub const LIST_DROP_WIDTH: f32 = 2.0;
/// Width of the halo on each side of a list's drop marker, as `handle.halo` around a handle.
pub const LIST_DROP_HALO: f32 = 1.0;
/// How far a selected row's dimmed label and detail line move from `text.on_selected` toward the
/// selected fill (OKLab mix), before their contrast minimum.
pub const LIST_SELECTED_QUIET: f64 = 0.4;

/// The control height the Compact angle dial sizes are set for.
pub const ANGLE_DIAL_BASE_CONTROL: f32 = 24.0;
/// Compact diameter of an angle dial's face (PhotoCraft's `angle_dial_row`).
pub const ANGLE_DIAL_SIZE: f32 = 42.0;
/// Compact width of an angle dial's needle.
pub const ANGLE_DIAL_NEEDLE_WIDTH: f32 = 2.0;
/// Compact distance from the dial's edge to the needle's tip.
pub const ANGLE_DIAL_NEEDLE_INSET: f32 = 4.0;
/// Compact radius of the dot at the dial's centre.
pub const ANGLE_DIAL_HUB: f32 = 2.0;
/// Compact radius round the dial's centre within which the pointer changes nothing.
pub const ANGLE_DIAL_DEAD_ZONE: f32 = 2.0;

/// A slider's default length, in control heights of its size.
pub const SLIDER_WIDTH: f32 = 6.0;
/// Thickness of a slider's track, in points, at every Size.
pub const SLIDER_TRACK: f32 = 4.0;

/// Canvas handle side or diameter in points, the same at every Size: handles sit on the
/// document, not in the chrome.
pub const HANDLE_SIZE: f32 = 8.0;
/// Handle outline width ([`Stroke::W1`](super::scale::Stroke::W1)).
pub const HANDLE_STROKE: f32 = 1.0;
/// Width of the halo drawn around a handle so it reads on any image.
pub const HANDLE_HALO: f32 = 1.0;
/// Side of the square around a handle's centre that takes the pointer, in points: larger than the
/// handle, so small handles stay easy to hit.
pub const HANDLE_HIT: f32 = 14.0;

/// Guide line width in points, the same at every Size: guides sit on the document.
pub const GUIDE_WIDTH: f32 = 1.0;
/// Distance from a guide, in points, within which the pointer takes it (PhotoCraft's).
pub const GUIDE_HIT: f32 = 4.0;
/// Dash length of a dashed guide.
pub const GUIDE_DASH: f32 = 4.0;
/// Gap between a dashed guide's dashes.
pub const GUIDE_GAP: f32 = 3.0;
/// Gap between a dotted guide's dots.
pub const GUIDE_DOT_GAP: f32 = 3.0;
/// Radius of a dotted guide's dots.
pub const GUIDE_DOT_RADIUS: f32 = 0.6;
/// Length of the end caps of a smart guide's distance.
pub const GUIDE_MEASURE_CAP: f32 = 4.0;
/// Padding of a distance's label, beside and above the text.
pub const GUIDE_MEASURE_PAD: (f32, f32) = (8.0, 4.0);

/// Length of the marching ants' dashes and gaps, in line widths (PhotoCraft's).
pub const MARQUEE_DASH: f32 = 4.0;
/// How fast the ants march, in line widths per second.
pub const MARQUEE_SPEED: f32 = 10.0;
/// Milliseconds between the ants' repaints.
pub const MARQUEE_TICK_MS: u32 = 100;
/// Dash length of a selection band, in points.
pub const MARQUEE_BAND_DASH: f32 = 3.0;
/// Gap between a selection band's dashes.
pub const MARQUEE_BAND_GAP: f32 = 3.0;
/// Width of the light line under an outline being drawn.
pub const MARQUEE_DRAWING_HALO: f32 = 3.0;
/// Farthest a press may move, in points, and still be a click (PhotoCraft's `is_click`).
pub const MARQUEE_CLICK_SLOP: f32 = 3.0;

/// Ruler strip thickness in points, the same at every Size (PhotoCraft's, Pro and Studio alike).
pub const RULER_SIZE: f32 = 16.0;
/// Ruler label size in points, the same at every Size and Text size.
pub const RULER_LABEL: f32 = 9.0;
/// Between the characters of the left ruler's stacked labels.
pub const RULER_LABEL_PITCH: f32 = 8.5;
/// Between a top-ruler label and its tick.
pub const RULER_LABEL_PAD: f32 = 2.0;
/// Least distance between labelled ticks, in points.
pub const RULER_STEP_MIN: f32 = 60.0;
/// Length of the tick halfway between labelled ticks.
pub const RULER_TICK_MID: f32 = 6.0;
/// Length of the other minor ticks.
pub const RULER_TICK_MINOR: f32 = 3.0;

/// The control height the Compact readout sizes are set for.
pub const READOUT_BASE_CONTROL: f32 = 24.0;
/// Compact distance from the pointer to a readout's corner, across and down (PhotoCraft's).
pub const READOUT_OFFSET: (f32, f32) = (16.0, 18.0);
/// Compact padding of a readout, beside and above the rows.
pub const READOUT_PAD: (f32, f32) = (7.0, 4.0);
/// Compact gap between a readout's label and value columns.
pub const READOUT_COL_GAP: f32 = 12.0;
/// Compact gap between a readout's rows.
pub const READOUT_ROW_GAP: f32 = 1.0;

/// Largest corner radius of a check box, in points: `shape.radius.control` up to this, so a
/// small box keeps square corners and never reads as a radio button.
pub const CHECK_RADIUS_MAX: f32 = 3.0;
/// Stroke width of a check box's check and dash, in points, at the small and standard sizes.
pub const CHECK_MARK_SM: f32 = 1.5;
/// See [`CHECK_MARK_SM`].
pub const CHECK_MARK_MD: f32 = 2.0;
/// Diameter of a radio button's dot as a fraction of the circle's.
pub const RADIO_DOT_RATIO: f32 = 0.4;
/// Width of a switch's track as a multiple of its height.
pub const SWITCH_ASPECT: f32 = 2.0;
/// Between a switch's track edge and its thumb, in points, at every Size.
pub const SWITCH_THUMB_INSET: f32 = 2.0;

/// Largest corner radius of a swatch, in points: `shape.radius.control` up to this.
pub const SWATCH_RADIUS_MAX: f32 = 4.0;
/// Width of a swatch's selection ring, drawn outside the well.
pub const SWATCH_SELECTED_WIDTH: f32 = 2.0;
/// Between a swatch's well and its selection ring, so the ring doesn't merge with the edge.
pub const SWATCH_SELECTED_GAP: f32 = 1.0;
/// Opacity of the black or white ring inside a swatch's edge.
pub const SWATCH_INNER_EDGE_ALPHA: u8 = 0x80;
/// The two checkerboard greys behind a translucent swatch in a light theme.
pub const SWATCH_CHECKER_LIGHT: (Rgba, Rgba) = (Rgba::rgb(255, 255, 255), Rgba::rgb(204, 204, 204));
/// The two checkerboard greys behind a translucent swatch in a dark theme.
pub const SWATCH_CHECKER_DARK: (Rgba, Rgba) = (Rgba::rgb(64, 64, 64), Rgba::rgb(102, 102, 102));

/// Above and below a badge's caption line, in points.
pub const BADGE_PAD_Y: f32 = 1.0;

/// Largest corner radius of a keycap, in points.
pub const KBD_RADIUS_MAX: f32 = 3.0;
/// Above and below a keycap's caption line, in points.
pub const KBD_PAD_Y: f32 = 1.0;

/// Thickness of a progress bar's track, in points, at every Size.
pub const PROGRESS_TRACK: f32 = 4.0;

/// Widest a tooltip's text grows before it wraps, in points.
pub const TOOLTIP_WIDTH: f32 = 320.0;

/// Outer width of a toast card, in points, at every Size.
pub const TOAST_WIDTH: f32 = 360.0;

/// Between a toast card's edge and its content, in points: Compact, Comfortable.
pub const fn toast_pad(size: Size) -> f32 {
    match size {
        Size::Compact => 10.0,
        Size::Comfortable => 12.0,
    }
}

/// From the host rect's reading-end edge and from its bottom edge to the toast stack, in points:
/// Compact, Comfortable.
pub const fn toast_inset(size: Size) -> [f32; 2] {
    match size {
        Size::Compact => [16.0, 12.0],
        Size::Comfortable => [20.0, 16.0],
    }
}

/// Widest the column of a centred empty state grows, in points, at every Size.
pub const EMPTY_STATE_MAX_WIDTH: f32 = 460.0;
