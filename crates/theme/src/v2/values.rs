//! Every tunable number of the semantic layer in one place: the ramp step each colour token
//! takes, fixed colours, contrast minimums, Size presets, type roles, text-size factors,
//! elevation and motion choices. The gallery tunes these; nothing else should hold such values.

use super::color::Rgba;
use super::ramp::Mode;
use super::scale::{Duration, Easing, Radius};
use super::theme::{Size, TextSize};
use super::tokens::{
    BarSizes, ColorToken, ControlSizes, Edge, FontSlot, Gaps, IconSizes, Pads, RadiusSet, RailSizes, SizeTokens, SpaceTokens, StatusKind, TypeStyle,
    TypeTokens,
};

use Bg::{HaloOver, Token};
use ColorToken as T;

/// Where a colour token's default value comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Source {
    /// A step (1 to 12) of the neutral ramp.
    Neutral(usize),
    /// A neutral step with an alpha (0..=1).
    NeutralAlpha(usize, f64),
    /// A step of the accent ramp.
    Accent(usize),
    /// The accent step `from` moved `t` (0..=1) of the way toward step `to`, in OKLab.
    AccentMix { from: usize, to: usize, t: f64 },
    /// A step of a status ramp.
    Status(StatusKind, usize),
    /// A fixed colour.
    Fixed(Rgba),
    /// An OKLCH colour (lightness, chroma, hue), gamut-mapped.
    Oklch(f64, f64, f64),
    /// The final value of another token.
    Same(ColorToken),
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
const STATUS_SOLIDS: &[ColorToken] =
    &[ColorToken::StatusGoodSolid, ColorToken::StatusWarnSolid, ColorToken::StatusBadSolid, ColorToken::StatusInfoSolid];

/// The default source of `token` in `mode`.
pub(super) fn source(token: ColorToken, mode: Mode) -> Source {
    use Source::{Accent, AccentMix, Fixed, Neutral, NeutralAlpha, Oklch, OnSolid, Same, Status};
    use StatusKind::{Bad, Good, Info, Warn};
    let dark = mode == Mode::Dark;
    let pick = |d: usize, l: usize| if dark { d } else { l };
    let pickf = |d: f64, l: f64| if dark { d } else { l };
    let series = |i: usize| Oklch(pickf(DATA_L.0, DATA_L.1), DATA_C, DATA_HUES[i]);
    match token {
        T::SurfaceBackground => Neutral(pick(2, 3)),
        T::SurfaceCanvas => Same(T::SurfaceBackground),
        T::SurfacePanel => Neutral(pick(3, 2)),
        T::SurfaceRaised => Neutral(pick(4, 1)),
        T::SurfaceSunken => Neutral(1),
        T::SurfaceScrim => Fixed(Rgba::BLACK.with_alpha(alpha(pickf(0.5, 0.35)))),
        T::TextNormal => Neutral(12),
        T::TextMuted => Neutral(11),
        T::TextFaint => Neutral(10),
        T::TextDisabled => Neutral(9),
        T::TextOnAccent => OnSolid(&[T::AccentSolid]),
        T::TextOnStatus => OnSolid(STATUS_SOLIDS),
        T::LineDivider => Neutral(6),
        T::LineBorder => Neutral(7),
        T::LineControlEdge => Neutral(8),
        T::LineFocus => Accent(9),
        T::AccentSolid => Accent(9),
        T::AccentHover => Accent(10),
        T::AccentPressed => AccentMix { from: 10, to: 11, t: 0.5 },
        T::AccentSoft => Accent(pick(4, 3)),
        T::AccentText => Accent(11),
        T::StatusGoodFg => Status(Good, 11),
        T::StatusGoodBg => Status(Good, 3),
        T::StatusGoodBorder => Status(Good, 7),
        T::StatusGoodSolid => Status(Good, 9),
        T::StatusWarnFg => Status(Warn, 11),
        T::StatusWarnBg => Status(Warn, 3),
        T::StatusWarnBorder => Status(Warn, 7),
        T::StatusWarnSolid => Status(Warn, 9),
        T::StatusBadFg => Status(Bad, 11),
        T::StatusBadBg => Status(Bad, 3),
        T::StatusBadBorder => Status(Bad, 7),
        T::StatusBadSolid => Status(Bad, 9),
        T::StatusInfoFg => Status(Info, 11),
        T::StatusInfoBg => Status(Info, 3),
        T::StatusInfoBorder => Status(Info, 7),
        T::StatusInfoSolid => Status(Info, 9),
        T::StateHover => Neutral(pick(4, 3)),
        T::StatePressed => Neutral(pick(5, 4)),
        T::StateSelected => Neutral(pick(6, 5)),
        T::StateSelectedHover => Neutral(pick(7, 6)),
        T::StateDisabledFill => Neutral(pick(3, 2)),
        T::OverlayStroke => Accent(9),
        T::OverlayHalo => Fixed(Rgba::BLACK.with_alpha(alpha(0.6))),
        T::OverlayHandleFill => Fixed(Rgba::WHITE),
        T::OverlayHandleStroke => Same(T::OverlayStroke),
        T::OverlayMarquee => Fixed(Rgba::WHITE),
        T::OverlayGuide => Oklch(pickf(0.75, 0.60), 0.14, 210.0),
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
        T::CaptionCloseHover => Fixed(Rgba::rgb(0xc4, 0x2b, 0x1c)),
        T::CaptionCloseText => Fixed(Rgba::WHITE),
        T::CaptionInactive => Same(T::TextFaint),
        // The lit edge is lighter than the panel and the shaded one darker, in either mode.
        T::EdgeLight => Neutral(pick(7, 1)),
        T::EdgeDark => Neutral(pick(1, 9)),
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

/// WCAG AA text minimum.
pub const TEXT_MIN: (f64, f64) = (4.5, 7.0);
/// WCAG non-text minimum (control boundaries, focus, indicators, overlays).
pub const NON_TEXT_MIN: (f64, f64) = (3.0, 4.5);
/// Design floor for `state.hover` on `surface.panel`.
pub const HOVER_FLOOR: (f64, f64) = (1.25, 1.5);
/// Design floor for `state.pressed` on `surface.panel`.
pub const PRESSED_FLOOR: (f64, f64) = (1.5, 2.0);
/// Design floor for `line.divider`.
pub const DIVIDER_FLOOR: (f64, f64) = (1.3, 2.0);

const TEXT_BGS: &[Bg] = &[
    Token(T::SurfaceBackground),
    Token(T::SurfacePanel),
    Token(T::SurfaceRaised),
    Token(T::SurfaceSunken),
    Token(T::StateHover),
    Token(T::StatePressed),
    Token(T::StateSelected),
    Token(T::StateSelectedHover),
];
const CONTROL_BGS: &[Bg] = &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::SurfaceSunken)];
const HALO_BGS: &[Bg] = &[HaloOver(Rgba::BLACK), HaloOver(Rgba::WHITE)];

/// Every contrast pair, in the order the contrast pass fixes them: fills that serve as
/// backgrounds later (state fills, `accent.solid`) come before the text drawn on them.
pub(super) const RULES: &[PairRule] = &[
    rule(&[T::StateHover], &[Token(T::SurfacePanel)], HOVER_FLOOR.0, HOVER_FLOOR.1),
    rule(&[T::StatePressed], &[Token(T::SurfacePanel)], PRESSED_FLOOR.0, PRESSED_FLOOR.1),
    rule(&[T::LineDivider], &[Token(T::SurfaceBackground), Token(T::SurfacePanel)], DIVIDER_FLOOR.0, DIVIDER_FLOOR.1),
    rule(&[T::LineControlEdge, T::LineFocus], CONTROL_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::AccentSolid], &[Token(T::SurfacePanel)], NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::TextNormal, T::TextMuted, T::TextFaint], TEXT_BGS, TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::AccentText], CONTROL_BGS, TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusGoodFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusGoodBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusWarnFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusWarnBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusBadFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusBadBg)], TEXT_MIN.0, TEXT_MIN.1),
    rule(&[T::StatusInfoFg], &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::StatusInfoBg)], TEXT_MIN.0, TEXT_MIN.1),
    PairRule {
        on_solid: true,
        ..rule(&[T::TextOnAccent], &[Token(T::AccentSolid), Token(T::AccentHover), Token(T::AccentPressed)], TEXT_MIN.0, TEXT_MIN.1)
    },
    PairRule {
        on_solid: true,
        ..rule(
            &[T::TextOnStatus],
            &[Token(T::StatusGoodSolid), Token(T::StatusWarnSolid), Token(T::StatusBadSolid), Token(T::StatusInfoSolid)],
            TEXT_MIN.0,
            TEXT_MIN.1,
        )
    },
    rule(&[T::OverlayStroke, T::OverlayMarquee], HALO_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
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
    },
    space: SpaceTokens { gap: Gaps { inline: 4.0, stack: 6.0, section: 12.0 }, pad: Pads { control: 6.0, panel: 8.0, dialog: 16.0 } },
    radius: RadiusSet { control: 3.0, popup: 4.0, window: 6.0, panel: 0.0, button: Radius::Full.px() },
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
    },
    space: SpaceTokens { gap: Gaps { inline: 6.0, stack: 8.0, section: 16.0 }, pad: Pads { control: 8.0, panel: 12.0, dialog: 20.0 } },
    radius: RadiusSet { control: 6.0, popup: 8.0, window: 12.0, panel: 0.0, button: Radius::Full.px() },
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
/// Shadow alpha multiplier in dark mode, where shadows need more weight to show.
pub const DARK_SHADOW_ALPHA_FACTOR: f32 = 1.5;

/// Durations for hover feedback, opening and moving.
pub const MOTION_DURATIONS: (Duration, Duration, Duration) = (Duration::Fast, Duration::Normal, Duration::Normal);
/// Easing for every semantic motion.
pub const MOTION_EASING: Easing = Easing::Standard;
