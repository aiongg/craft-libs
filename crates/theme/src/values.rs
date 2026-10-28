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

/// Where a colour token's default value comes from: a ramp step, a mix of two steps, a fixed
/// colour, or other tokens. [`explain`](crate::explain) reports it per token.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ColorSource {
    /// A step (1 to 12) of the neutral ramp.
    Neutral(usize),
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

/// The default source of `token` in `mode`.
pub(super) fn source(token: ColorToken, mode: Mode) -> ColorSource {
    use ColorSource::{Accent, AccentMix, Fixed, Neutral, NeutralAlpha, Oklch, OnSolid, Same, Status, StatusMix};
    use StatusKind::{Bad, Good, Info, Warn};
    let dark = mode == Mode::Dark;
    let pick = |d: usize, l: usize| if dark { d } else { l };
    let pickf = |d: f64, l: f64| if dark { d } else { l };
    let series = |i: usize| Oklch(pickf(DATA_L.0, DATA_L.1), DATA_C, DATA_HUES[i]);
    let (pressed_from, pressed_to, pressed_t) = SOLID_PRESSED;
    let pressed = |kind| StatusMix { kind, from: pressed_from, to: pressed_to, t: pressed_t };
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
        // `text.normal`'s step, so neutral selections look like the rest of the text.
        T::TextOnSelected => Neutral(12),
        T::LineDivider => Neutral(6),
        T::LineBorder => Neutral(7),
        T::LineControlEdge => Neutral(8),
        T::LineFocus => Accent(9),
        T::AccentSolid => Accent(9),
        T::AccentHover => Accent(SOLID_HOVER),
        T::AccentPressed => AccentMix { from: pressed_from, to: pressed_to, t: pressed_t },
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
    rule(&[T::AccentSolid], &[Token(T::SurfacePanel)], NON_TEXT_MIN.0, NON_TEXT_MIN.1),
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

/// Button padding at the standard size, as a multiple of `pad.control`.
pub const BUTTON_PAD_X_MD: f32 = 2.0;
/// Button outline width ([`Stroke::W1`](super::scale::Stroke::W1)); control borders are 1 point.
pub const BUTTON_BORDER_WIDTH: f32 = 1.0;

/// Between a segmented control's track edge and its segments, in points, at every Size.
pub const SEGMENTED_INSET: f32 = 2.0;

/// A slider's default length, in control heights of its size.
pub const SLIDER_WIDTH: f32 = 6.0;
/// Thickness of a slider's track, in points, at every Size.
pub const SLIDER_TRACK: f32 = 4.0;
/// Corner radius of a slider's track and thumb: [`Radius::Full`] gives a pill track and a round
/// thumb.
pub const SLIDER_RADIUS: Radius = Radius::Full;

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

/// Corner radius of a badge: a pill.
pub const BADGE_RADIUS: Radius = Radius::Full;
/// Above and below a badge's caption line, in points.
pub const BADGE_PAD_Y: f32 = 1.0;

/// Largest corner radius of a keycap, in points.
pub const KBD_RADIUS_MAX: f32 = 3.0;
/// Above and below a keycap's caption line, in points.
pub const KBD_PAD_Y: f32 = 1.0;

/// Thickness of a progress bar's track, in points, at every Size.
pub const PROGRESS_TRACK: f32 = 4.0;
/// Corner radius of a progress bar's track and fill: a pill.
pub const PROGRESS_RADIUS: Radius = Radius::Full;

/// Widest a tooltip's text grows before it wraps, in points.
pub const TOOLTIP_WIDTH: f32 = 320.0;
