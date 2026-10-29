//! Every tunable number of the semantic layer in one place: the ramp step each colour token
//! takes, fixed colours, contrast minimums, Size presets, type roles, text-size factors,
//! elevation and motion choices. The gallery tunes these; nothing else should hold such values.

use super::color::Rgba;
use super::ramp::Mode;
use super::scale::{Duration, Easing, Elevation, Radius};
use super::theme::{Size, TextSize};
use super::tokens::{
    BarSizes, ColorToken, ControlSizes, Edge, FontSlot, Gaps, IconSizes, ListSizes, Pads, RadiusSet, RailSizes, SizeTokens, SpaceTokens, StatusKind,
    TabSizes, ThumbSizes, TypeStyle, TypeTokens,
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
    /// A solid fill taken from `base` with its OKLCH lightness moved away from `away_from` (darker
    /// under light text, lighter under dark text) by how far `to` lies from `from` in the
    /// unpinned ramp defaults: the pointer or pressed variant of a solid under text. It always
    /// differs from `base`, and the text rule moves it further only if `base` itself misses.
    ShiftedFrom {
        /// The solid the fill starts from.
        base: ColorToken,
        /// The token whose default the offset is measured from.
        from: ColorToken,
        /// The token whose default the offset is measured to.
        to: ColorToken,
        /// The text the fill moves away from.
        away_from: ColorToken,
    },
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
        Accent, AccentMix, Anchored, Fixed, Lighting, Neutral, NeutralAlpha, Oklch, OnSolid, Same, ShiftedFrom, SolidFrom, StateVariant, Status,
        StatusMix,
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
        T::SurfaceChrome => Same(T::SurfacePanel),
        T::SurfaceRaised => Anchored(pick(4, 1)),
        T::SurfaceSunken => Anchored(1),
        // Near white in both modes (dark: about 236/255), as macOS, GNOME and PhotoCraft knobs. A
        // knob face is not a background grey, so it stays on the curve.
        T::SurfaceKnob => Neutral(pick(12, 1)),
        T::SurfaceScrim => Fixed(Rgba::BLACK.with_alpha(alpha(pickf(0.5, 0.35)))),
        T::SurfaceShadow => Fixed(Rgba::BLACK.with_alpha(pick(SHADOW_ALPHA.1, SHADOW_ALPHA.0) as u8)),
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
        // A light knob needs an outline on a light panel (PhotoCraft's `text_dim`); on a dark one
        // its face is the edge. The Pro themes pin PhotoCraft's dark ring.
        T::LineKnobEdge => {
            if dark {
                Fixed(Rgba::TRANSPARENT)
            } else {
                Same(T::TextMuted)
            }
        }
        // The contrast pass moves it to the non-text minimum on the control surfaces.
        T::LineMark => Neutral(8),
        T::LineFocus => Accent(9),
        T::AccentSolid => Accent(9),
        T::AccentHover => Accent(SOLID_HOVER),
        T::AccentPressed => AccentMix { from: pressed_from, to: pressed_to, t: pressed_t },
        T::AccentStrong => SolidFrom(T::AccentSolid),
        T::AccentStrongHover => ShiftedFrom { base: T::AccentStrong, from: T::AccentSolid, to: T::AccentHover, away_from: T::TextOnStrong },
        T::AccentStrongPressed => ShiftedFrom { base: T::AccentStrong, from: T::AccentSolid, to: T::AccentPressed, away_from: T::TextOnStrong },
        T::AccentSoft => Accent(pick(4, 3)),
        T::AccentBorder => Same(T::AccentSolid),
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
        T::OverlayLine => Same(T::AccentSolid),
        T::OverlayHalo => Fixed(Rgba::BLACK.with_alpha(alpha(0.6))),
        T::OverlayHandleFill => Fixed(Rgba::WHITE),
        T::OverlayHandleStroke => Same(T::AccentSolid),
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
        // PhotoCraft's histogram channel colours (`histogram_panel.rs:191`), the same in every theme and mode.
        T::DataRed => Fixed(Rgba::rgb(235, 30, 30)),
        T::DataGreen => Fixed(Rgba::rgb(30, 235, 30)),
        T::DataBlue => Fixed(Rgba::rgb(40, 40, 240)),
        T::DataLuma => Neutral(12),
        T::DataCyan => Fixed(Rgba::rgb(0, 200, 235)),
        T::DataMagenta => Fixed(Rgba::rgb(230, 0, 160)),
        T::DataYellow => Fixed(Rgba::rgb(240, 220, 0)),
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
/// The cue of a selected or on state (a fill or an edge on what it sits on): required in high
/// contrast only. Standard contrast keeps the theme's own fills and has no minimum for it.
pub const STATE_CUE_MIN: (f64, f64) = (0.0, NON_TEXT_MIN.1);
/// The edge that identifies a control (a field's or button's border, a slider track, the active
/// row of a popup) against the surface it sits on: WCAG 1.4.11,
/// required in high contrast only. Standard contrast keeps the theme's own edges.
pub const CONTROL_EDGE_MIN: (f64, f64) = (0.0, 3.0);
/// The active row of a listbox or menu against its popup: a floor in Standard, the control edge
/// minimum in high contrast.
pub const ACTIVE_ROW_MIN: (f64, f64) = (LIST_HOVER_MIN.0, CONTROL_EDGE_MIN.1);

const TEXT_BGS: &[Bg] = &[
    Token(T::SurfaceBackground),
    Token(T::SurfacePanel),
    Token(T::SurfaceChrome),
    Token(T::SurfaceRaised),
    Token(T::SurfaceSunken),
    Token(T::StateHover),
    Token(T::StatePressed),
];
const SELECTED_BGS: &[Bg] = &[Token(T::StateSelected), Token(T::StateSelectedHover)];
const CONTROL_BGS: &[Bg] = &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::SurfaceSunken)];
/// The focus line also marks focus on the title bars and tab strips, which sit on the background.
const FOCUS_BGS: &[Bg] =
    &[Token(T::SurfaceBackground), Token(T::SurfacePanel), Token(T::SurfaceChrome), Token(T::SurfaceRaised), Token(T::SurfaceSunken)];
/// Where a mark or a badge ring is drawn: control surfaces and the hover and pressed fills. The
/// selected fills are left out: no single colour reaches the minimum on both a light panel and
/// Classic's dark selection fill.
const MARK_BGS: &[Bg] = &[Token(T::SurfacePanel), Token(T::SurfaceRaised), Token(T::SurfaceSunken), Token(T::StateHover), Token(T::StatePressed)];
const HALO_BGS: &[Bg] = &[HaloOver(Rgba::BLACK), HaloOver(Rgba::WHITE)];

/// Every contrast pair, in the order the contrast pass fixes them: fills that serve as
/// backgrounds later (state fills, `accent.solid`) come before the text drawn on them.
pub(super) const RULES: &[PairRule] = &[
    rule(&[T::LineFocus], FOCUS_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::AccentSolid], CONTROL_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    // The outline of a selected or on control is a state cue: Standard keeps PhotoCraft's translucent
    // one. Unpinned, it is the final `accent.solid`.
    rule(&[T::AccentBorder], CONTROL_BGS, STATE_CUE_MIN.0, STATE_CUE_MIN.1),
    rule(&[T::LineMark], MARK_BGS, NON_TEXT_MIN.0, NON_TEXT_MIN.1),
    rule(&[T::TextNormal, T::TextMuted, T::TextFaint], TEXT_BGS, TEXT_MIN.0, TEXT_MIN.1),
    // A canvas scroll bar's thumb is a boundary on its track: Standard keeps the theme's own greys.
    rule(&[T::TextFaint, T::TextMuted], &[Token(T::SurfaceChrome)], CONTROL_EDGE_MIN.0, CONTROL_EDGE_MIN.1),
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
    // After the rule on `accent.strong`, which the variants start from.
    PairRule { on_solid: true, ..rule(&[T::TextOnStrong], &[Token(T::AccentStrongHover), Token(T::AccentStrongPressed)], TEXT_MIN.0, TEXT_MIN.1) },
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
    // A selected anchor is a handle filled with its outline colour: the outline on the fill is the
    // only mark of the selection. Standard keeps the accent PhotoCraft outlines its handles in.
    rule(&[T::OverlayHandleStroke], &[Token(T::OverlayHandleFill)], STATE_CUE_MIN.0, STATE_CUE_MIN.1),
    // A curve editor's curve and points in a channel colour, on its plot (`field.fill`, `surface.sunken`).
    // PhotoCraft's channel colours are the same in every theme, so Standard keeps them as they are.
    rule(
        &[T::DataRed, T::DataGreen, T::DataBlue, T::DataCyan, T::DataMagenta, T::DataYellow],
        &[Token(T::SurfaceSunken)],
        STATE_CUE_MIN.0,
        STATE_CUE_MIN.1,
    ),
    // Guide and smart-guide colours have no pair: they are the user's choice, as in the apps'
    // preferences, and a bare line on an image can't promise a ratio. `overlay.line` neither, for
    // the same reason.
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
        rail: RailSizes { width: 40.0, button: 30.0, panel_width: 36.0, panel_button: 28.0 },
        bar: BarSizes { title: 32.0, tab: 26.0, status: 24.0, options: 36.0, options_narrow: 30.0 },
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
        tab: TabSizes { panel_height: 28.0, height_large: 34.0, min_width: 40.0, close: 14.0, close_icon: 10.0, rounded: false, dividers: true },
    },
    space: SpaceTokens { gap: Gaps { inline: 4.0, row: 4.0, stack: 6.0, section: 12.0 }, pad: Pads { control: 6.0, panel: 8.0, dialog: 16.0 } },
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
        rail: RailSizes { width: 50.0, button: 36.0, panel_width: 44.0, panel_button: 32.0 },
        bar: BarSizes { title: 38.0, tab: 30.0, status: 30.0, options: 42.0, options_narrow: 34.0 },
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
        tab: TabSizes { panel_height: 40.0, height_large: 38.0, min_width: 44.0, close: 16.0, close_icon: 12.0, rounded: true, dividers: false },
    },
    space: SpaceTokens { gap: Gaps { inline: 6.0, row: 6.0, stack: 8.0, section: 16.0 }, pad: Pads { control: 8.0, panel: 12.0, dialog: 20.0 } },
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

/// The shadow of a floating window (`elevation.floating`): PhotoCraft's `window_shadow`
/// (`theme.rs:1036`), 10 points down with a 32 point blur, in `surface.shadow`'s alpha.
pub const WINDOW_SHADOW: Elevation = Elevation { offset: [0.0, 10.0], blur: 32.0, spread: 0.0, alpha: 0.0 };
/// The shadow of a popup, menu or dialog (`elevation.popup`): PhotoCraft's `popup_shadow` (`theme.rs:1037`), 6
/// points down with a 20 point blur. Its alpha is `surface.shadow`'s, like every cast shadow's.
pub const POPUP_SHADOW: Elevation = Elevation { offset: [0.0, 6.0], blur: 20.0, spread: 0.0, alpha: 0.0 };
/// The alpha of `surface.shadow` in light and dark mode (0..=255): PhotoCraft's Pro
/// `Tokens::shadow` (`theme.rs`), 50 in light and 150 in dark; themes pin their own.
pub const SHADOW_ALPHA: (usize, usize) = (50, 150);
/// The shadow of a knob on a track with a flat edge (`elevation.knob`): PhotoCraft's hard circle,
/// the knob's size grown by half a point, one point down, black at 90/255 in both modes
/// (`slider.rs:206-207`). egui rounds a shadow's spread to whole points.
pub const KNOB_SHADOW: Elevation = Elevation { offset: [0.0, 1.0], blur: 0.0, spread: 0.5, alpha: 90.0 / 255.0 };
/// How far a disabled knob's face moves from `surface.knob` toward `surface.panel`, 0..=1: far
/// enough to recede in both modes, short of vanishing on the panel in dark mode.
pub const KNOB_DISABLED: f64 = 0.5;
/// Width of the ring round a slider thumb while hovered or pressed, in points.
pub const KNOB_RING: f32 = 2.0;
/// From a slider thumb's edge to its ring's centre line, in points (a 12 point knob has a ring of
/// radius 9.5).
pub const KNOB_RING_OFFSET: f32 = 3.5;
/// Durations for hover feedback, opening and moving.
pub const MOTION_DURATIONS: (Duration, Duration, Duration) = (Duration::Fast, Duration::Normal, Duration::Normal);
/// Easing for every semantic motion.
pub const MOTION_EASING: Easing = Easing::Standard;

/// How much of its fill a check box that is on shows while pressed (`widgets/buttons.rs:247`).
pub const CHECK_ON_PRESSED_OPACITY: f32 = 0.75;

/// Side of the options bar's icon buttons: PhotoCraft's tool icon button (26 Pro, 28 Studio,
/// `panels/options_bar.rs:57`).
pub const fn options_bar_button(size: Size) -> f32 {
    match size {
        Size::Compact => 26.0,
        Size::Comfortable => 28.0,
    }
}

/// A progress bar's length when the caller gives none: PhotoCraft's job progress bar (120 Pro,
/// 140 Studio, `jobs_ui.rs:655`).
pub const fn progress_width(size: Size) -> f32 {
    match size {
        Size::Compact => 120.0,
        Size::Comfortable => 140.0,
    }
}

/// The control height the Compact panel footer sizes are set for.
pub const PANEL_FOOTER_BASE_CONTROL: f32 = 24.0;
/// Compact height of a panel footer's bar (PhotoCraft's `FOOTER_BAR`, `widgets/panel.rs:58`).
pub const PANEL_FOOTER_HEIGHT: f32 = 30.0;
/// Compact gap between a panel footer's buttons.
pub const PANEL_FOOTER_GAP: f32 = 2.0;
/// Compact side of a panel footer's button.
pub const PANEL_FOOTER_BUTTON: f32 = 26.0;

/// Button heights in points, small and standard: PhotoCraft's status-bar button (22, the same in
/// its Pro and Studio layouts' non-Pro branch, `status_bar.rs:8`) and its button (28 Pro, 30
/// Studio, `widgets/buttons.rs:150`).
pub const fn button_height(size: Size) -> (f32, f32) {
    match size {
        Size::Compact => (22.0, 28.0),
        Size::Comfortable => (22.0, 30.0),
    }
}

/// Button padding at the start and end, in points, at both button sizes: PhotoCraft's 14 at
/// Compact, and the 4 points Comfortable's control padding adds.
pub const fn button_pad_x(size: Size) -> f32 {
    match size {
        Size::Compact => 14.0,
        Size::Comfortable => 18.0,
    }
}

/// Button outline width ([`Stroke::W1`](super::scale::Stroke::W1)); control borders are 1 point.
pub const BUTTON_BORDER_WIDTH: f32 = 1.0;
/// How much of its rest fill a primary button shows under the pointer, and a primary or secondary
/// button while pressed (PhotoCraft draws these as the fill's alpha times the factor:
/// `widgets/buttons.rs:189-191`). The Pro themes pin the values of PhotoCraft's Pro buttons
/// (`buttons.rs:165-166`: 0.9 and 0.8).
pub const BUTTON_HOVER_OPACITY: f32 = 0.93;
pub const BUTTON_PRESSED_OPACITY: f32 = 0.85;

/// How far a segmented control's gap between pills is over the inline gap of its Size, in points
/// (PhotoCraft's `pill_tab` item spacing is 8: the Compact inline gap 4 plus this).
pub const SEGMENTED_GAP_OVER_INLINE: f32 = 4.0;
/// Opacity of `state.hover` behind an unselected pill under the pointer (PhotoCraft's `pill_tab`:
/// `hover.gamma_multiply(0.6)`, `widgets/panel.rs:116`).
pub const SEGMENT_HOVER_OPACITY: f32 = 0.6;

/// A text field's default width, in control heights of its size.
pub const FIELD_WIDTH: f32 = 8.0;

/// A number field's default width, in control heights of its size; apps set other widths per field.
pub const NUMBER_FIELD_WIDTH: f32 = 3.0;

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
/// The tone look behind a curve: PhotoCraft's grey 190 at 0.75 (`curves.rs:274`, its non-Pro look; the
/// Pro themes pin grey 88).
pub const HISTOGRAM_BACKDROP: Rgba = Rgba([190, 190, 190, 191]);
/// Alpha of the selected range's accent band (PhotoCraft's `histogram_panel.rs:648`).
pub const HISTOGRAM_SELECTION_ALPHA: f32 = 0.35;
/// Width of the RGB ribbons' outlines.
pub const HISTOGRAM_LINE: f32 = 1.0;
/// Width of a marker tick (PhotoCraft's sampler mark, `histogram_panel.rs:661`).
pub const HISTOGRAM_MARKER_WIDTH: f32 = 1.5;
/// Height of a marker tick.
pub const HISTOGRAM_MARKER_HEIGHT: f32 = 9.0;

/// Points added to `space.pad.control` for the side padding of a list box row: PhotoCraft's
/// button padding of 10 beside a 6 point control padding (`style_spectrum_popup_menu`,
/// `panel.rs:3-21`).
pub const LISTBOX_PAD_X: f32 = 4.0;
/// The most a select's or combo box's list shows before it scrolls, at Compact: PhotoCraft's
/// dropdown height (`dropdown.rs:74`).
pub const LISTBOX_MAX_HEIGHT: f32 = 420.0;
/// The control height the Compact menu sizes are set for.
pub const MENU_BASE_CONTROL: f32 = 24.0;
/// Compact least width of a menu (PhotoCraft's).
pub const MENU_MIN_WIDTH: f32 = 220.0;
/// Compact least room between a menu row's label and its shortcut or submenu arrow.
pub const MENU_SHORTCUT_GAP: f32 = 16.0;
/// The alpha a menu row's label colour starts at before its text is derived against it
/// (PhotoCraft draws 0.3; 0.24 is the most that keeps `text.normal` at 4.5:1 on all its hues in
/// every built-in theme).
pub const MENU_TINT_ALPHA: f32 = 0.24;
/// Compact height of a menu's scroll strip (PhotoCraft's scroll arrows).
pub const MENU_SCROLL_STRIP: f32 = 16.0;
/// The submenu arrow's base, as a fraction of the label size (egui's `SubMenuButton::arrow_atom`,
/// which PhotoCraft's menus draw).
pub const MENU_ARROW_BASE: f32 = 0.55;
/// The submenu arrow's depth (tip to base), as a fraction of the label size.
pub const MENU_ARROW_DEPTH: f32 = 0.35;

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

/// The control height the Compact dock bar heights are set for.
pub const DOCK_BASE_CONTROL: f32 = 24.0;
/// Compact height of a floating window's title bar and of a collapsed column's bar.
pub const DOCK_BAR_HEIGHT: f32 = 14.0;
/// The vertical gap between stacked controls in a panel body: PhotoCraft's dock bodies set item
/// spacing to 0 in the Pro layout and 6 otherwise (`panels/panel_docking.rs:1229`).
pub const fn dock_body_gap(size: Size) -> f32 {
    match size {
        Size::Compact => 0.0,
        Size::Comfortable => 6.0,
    }
}
/// Added to the icon size for the height of an iconic column's row.
pub const DOCK_ICON_PAD: f32 = 10.0;
/// Width of a group's border.
pub const DOCK_BORDER_WIDTH: f32 = 1.0;
/// Width of a drop outline.
pub const DOCK_DROP_WIDTH: f32 = 2.0;
/// Width of a keyboard focus stroke.
pub const DOCK_FOCUS_WIDTH: f32 = 1.0;
/// Corner radius of the insertion marker.
pub const DOCK_MARKER_RADIUS: f32 = 1.0;
/// Alpha factor of the filled drop preview.
pub const DOCK_DROP_FILL: f32 = 0.15;
/// Alpha factor of the highlighted zone under a dragged tab.
pub const DOCK_GHOST_FILL: f32 = 0.3;
/// Alpha factor of the outline of the zones under a dragged tab.
pub const DOCK_GHOST_STROKE: f32 = 0.6;
/// Alpha factor of the light fill inside a tab drop's outline.
pub const DOCK_TAB_FILL: f32 = 0.08;
/// Alpha factor of the resize handle's stroke at hover and focus.
pub const DOCK_GRIP_ALPHA: f32 = 0.5;
/// Width of a floating window's border.
pub const FLOAT_BORDER_WIDTH: f32 = 1.0;
/// Corner radius of a floating window by default: square, lifted by its shadow.
pub const FLOAT_RADIUS: f32 = 0.0;

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
/// Compact depth of a strip pointer triangle (PhotoCraft's thumb is 7 tall, `color_panel_ui.rs:281`).
pub const COLOR_PICKER_POINTER_WIDTH: f32 = 7.0;
/// Compact base of a strip pointer triangle (PhotoCraft's thumb is 10 wide).
pub const COLOR_PICKER_POINTER_HEIGHT: f32 = 10.0;
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

/// The control height the Compact levels editor sizes are set for.
pub const LEVELS_BASE_CONTROL: f32 = 24.0;
/// Compact least width of a levels editor (EffectCraft's Effect Controls).
pub const LEVELS_MIN_WIDTH: f32 = 120.0;
/// Compact greatest width of a levels editor (PhotoCraft's Levels).
pub const LEVELS_MAX_WIDTH: f32 = 300.0;
/// Compact height of a levels editor's row of triangle thumbs.
pub const LEVELS_ROW: f32 = 14.0;
/// Compact width of a levels triangle thumb.
pub const LEVELS_THUMB_WIDTH: f32 = 11.0;
/// Compact height of a levels triangle thumb.
pub const LEVELS_THUMB_HEIGHT: f32 = 9.0;
/// Compact distance from a levels thumb's centre within which a press grabs it: half of a 14 pt
/// hit area.
pub const LEVELS_HIT: f32 = 7.0;
/// Compact thickness of a levels editor's output gradient bar (PhotoCraft's `levels.rs:140`).
pub const LEVELS_BAR: f32 = 10.0;
/// Compact height of a levels editor's histogram (PhotoCraft's `levels.rs:88`).
pub const LEVELS_HISTOGRAM: f32 = 110.0;
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

/// Opacity of the hover fill of a list row (PhotoCraft `hover.gamma_multiply(0.5)`:
/// `layer_row.rs:285`, `history.rs:63`, `channels_panel.rs`, `actions.rs`).
pub const LIST_HOVER_OPACITY: f32 = 0.5;
/// Opacity of the accent tint over the hover fill of a layer-form row (`layer_row.rs:286`).
pub const LIST_LAYER_TINT_OPACITY: f32 = 0.12;
/// Added to a list row's thumbnail side for the row's height.
pub const LIST_THUMB_PAD: f32 = 8.0;
/// Width of a list's drop marker line and outline (PhotoCraft, DesignCraft and docking).
pub const LIST_DROP_WIDTH: f32 = 2.0;
/// Width of the halo on each side of a list's drop marker, as `handle.halo` around a handle.
pub const LIST_DROP_HALO: f32 = 1.0;
/// How far a selected row's dimmed label and detail line move from `text.on_selected` toward the
/// selected fill (OKLab mix), before their contrast minimum.
pub const LIST_SELECTED_QUIET: f64 = 0.4;
/// Least contrast ratio (standard, high) between a list row's hover fill and the fills it sits
/// beside: the rest surface and the selected fill. The built-in hover steps are 1.1 to 1.4 away
/// from the panel; 1.12 is the least that still reads on a screen at normal brightness. A hover
/// step that is closer is moved away along OKLCH lightness.
pub const LIST_HOVER_MIN: (f64, f64) = (1.12, 1.12);

/// The control height the Compact angle dial sizes are set for.
pub const ANGLE_DIAL_BASE_CONTROL: f32 = 24.0;
/// Compact diameter of an angle dial's face (PhotoCraft's `angle_dial_row`).
pub const ANGLE_DIAL_SIZE: f32 = 42.0;
/// Compact distance from the dial's box to its face (`angle_dial.rs:30`: radius is half the box less 2).
pub const ANGLE_DIAL_MARGIN: f32 = 2.0;
/// Compact width of an angle dial's needle.
pub const ANGLE_DIAL_NEEDLE_WIDTH: f32 = 2.0;
/// Compact distance from the dial's edge to the needle's tip.
pub const ANGLE_DIAL_NEEDLE_INSET: f32 = 4.0;
/// Compact radius of the dot at the dial's centre.
pub const ANGLE_DIAL_HUB: f32 = 2.0;
/// Compact radius round the dial's centre within which the pointer changes nothing.
pub const ANGLE_DIAL_DEAD_ZONE: f32 = 2.0;
/// Width of a tab strip's drop marker line and outline.
pub const TAB_DROP_WIDTH: f32 = 2.0;
/// Width of the halo on each side of a tab strip's drop marker.
pub const TAB_DROP_HALO: f32 = 1.0;
/// Height of a tab's progress bar, in points, at every Size.
pub const TAB_PROGRESS_HEIGHT: f32 = 2.0;
/// Thickness of the bar along the top edge of the selected tab, in points, at every Size.
pub const TAB_INDICATOR_WIDTH: f32 = 2.0;
/// How far outside its strip a tab drag still resolves targets (PhotoCraft's `TEAR_OFF`).
pub const TAB_TEAR_OFF: f32 = 24.0;

/// Least title bar height with Adwaita caption buttons: libadwaita's 46 pt header plus its 1 pt
/// separator.
pub const TITLE_BAR_HEIGHT_ADWAITA: f32 = 47.0;
/// Least room between a title bar's title and what is beside it.
pub const TITLE_BAR_TITLE_GAP: f32 = 16.0;
/// Least gap that still shows an elided title bar title.
pub const TITLE_BAR_TITLE_MIN_GAP: f32 = 80.0;
/// Width of the title bar's edge resize zones.
pub const TITLE_BAR_EDGE: f32 = 8.0;
/// Side of the title bar's corner resize zones.
pub const TITLE_BAR_CORNER: f32 = 12.0;
/// Windows 11 Close button under the pointer; held down it is 90 % of this.
pub const TITLE_BAR_WINDOWS_CLOSE: Rgba = Rgba::rgb(0xC4, 0x2B, 0x1C);
/// The glyph on Windows' Close button.
pub const TITLE_BAR_WINDOWS_CLOSE_GLYPH: Rgba = Rgba::WHITE;
/// KDE Breeze Close button under the pointer.
pub const TITLE_BAR_BREEZE_CLOSE_HOVER: Rgba = Rgba::rgb(0xFF, 0x98, 0xA2);
/// The glyph on Breeze's hovered Close button.
pub const TITLE_BAR_BREEZE_CLOSE_HOVER_GLYPH: Rgba = Rgba::rgb(0x23, 0x26, 0x29);
/// KDE Breeze Close button held down.
pub const TITLE_BAR_BREEZE_CLOSE_PRESSED: Rgba = Rgba::rgb(0x6D, 0x22, 0x29);
/// The glyph on Breeze's pressed Close button.
pub const TITLE_BAR_BREEZE_CLOSE_PRESSED_GLYPH: Rgba = Rgba::WHITE;

/// A slider's default length, in control heights of its size.
pub const SLIDER_WIDTH: f32 = 6.0;
/// Thickness of a slider's track, in points, at every Size (PhotoCraft `slider.rs:104-289`).
pub const SLIDER_TRACK: f32 = 3.0;
/// Thickness of a slider's gradient track, in points.
pub const SLIDER_GRADIENT_TRACK: f32 = 5.0;
/// Corner radius of a slider's track, in points.
pub const SLIDER_TRACK_RADIUS: f32 = 2.0;
/// How far under the control height a slider is, in points, at the small and standard sizes
/// (Compact 18, PhotoCraft's).
pub const SLIDER_HEIGHT_INSET: (f32, f32) = (4.0, 6.0);
/// How far under the icon size a slider's thumb is, in points, at the small and standard sizes
/// (Compact 12, PhotoCraft's).
pub const SLIDER_THUMB_INSET: (f32, f32) = (2.0, 4.0);
/// The control height the Compact slider mark height is set for.
pub const SLIDER_BASE_CONTROL: f32 = 24.0;
/// Compact width of a range slider's thumb: PhotoCraft's range handle, a 5 by 12 rectangle
/// (`hue_saturation.rs:144-210`).
pub const RANGE_THUMB_WIDTH: f32 = 5.0;
/// Compact height of a range slider's thumb.
pub const RANGE_THUMB_HEIGHT: f32 = 12.0;
/// Corner radius of a range slider's thumb.
pub const RANGE_THUMB_RADIUS: f32 = 1.0;
/// Compact height of a slider's marks and ticks, in points.
pub const SLIDER_MARK: f32 = 4.0;
/// How near a mark (in points) a drag snaps the slider's thumb to it.
pub const SLIDER_SNAP: f32 = 6.0;

/// Canvas handle side or diameter in points, the same at every Size: handles sit on the
/// document, not in the chrome.
pub const HANDLE_SIZE: f32 = 7.0;
/// Handle outline width ([`Stroke::W1`](super::scale::Stroke::W1)).
pub const HANDLE_STROKE: f32 = 1.0;
/// Width of the halo drawn around a handle so it reads on any image.
pub const HANDLE_HALO: f32 = 1.0;
/// Side of the square around a handle's centre that takes the pointer, in points: larger than the
/// handle, so small handles stay easy to hit.
pub const HANDLE_HIT: f32 = 14.0;

/// Length of each arm of a painted crosshair, from the hotspot (PhotoCraft's).
pub const CURSOR_CROSSHAIR_REACH: f32 = 8.0;
/// Empty space round the hotspot where a painted crosshair's arms begin.
pub const CURSOR_CROSSHAIR_GAP: f32 = 2.0;
/// Half the side of a selection badge's glyph (PhotoCraft's `badge_glyph` size).
pub const CURSOR_BADGE_HALF: f32 = 3.5;
/// Distance of a selection badge from the hotspot, right and down, beside a painted crosshair.
pub const CURSOR_BADGE_OFFSET: f32 = 12.0;
/// Distance of a selection badge from the hotspot, right and down, beside a system pointer: the
/// arrow covers 11 points at 1x, so the badge clears it (PhotoCraft's `draw_badge`).
pub const CURSOR_BADGE_BESIDE: (f32, f32) = (30.0, 11.0);
/// Length of the distort arrowhead (PhotoCraft's).
pub const CURSOR_DART_LENGTH: f32 = 16.0;
/// Half the width of the distort arrowhead's base (PhotoCraft's).
pub const CURSOR_DART_HALF_WIDTH: f32 = 7.0;
/// Width of the halo outline drawn over the distort arrowhead's edge (PhotoCraft's).
pub const CURSOR_DART_OUTLINE: f32 = 1.5;
/// Distance of each skew arrow from the pointer's axis (PhotoCraft's).
pub const CURSOR_SKEW_GAP: f32 = 4.0;
/// Half the length of a skew arrow's shaft (PhotoCraft's).
pub const CURSOR_SKEW_HALF_LENGTH: f32 = 9.0;
/// Reach of a skew arrowhead's tip past the shaft's end, and its half width (PhotoCraft's).
pub const CURSOR_SKEW_HEAD: f32 = 3.5;
/// How far a skew arrowhead's base lies back from the shaft's end (PhotoCraft's).
pub const CURSOR_SKEW_BACK: f32 = 2.5;

/// Width of the halo outline drawn over a skew arrowhead's edge (PhotoCraft's).
pub const CURSOR_SKEW_HEAD_OUTLINE: f32 = 1.0;

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

/// Width of a transform box's line and a warp mesh's patch boundaries, in points, at every Size
/// (PhotoCraft's).
pub const TRANSFORM_LINE: f32 = 1.0;
/// Distance from a box handle or the reference point within which the pointer takes it
/// (PhotoCraft's `HANDLE_PX`).
pub const TRANSFORM_HIT: f32 = 12.0;
/// Distance from a corner, outside the box, within which the rotate cursor shows (PhotoCraft's
/// three handle reaches).
pub const TRANSFORM_ROTATE_REACH: f32 = 36.0;
/// Inner edge of the rotate band outside a corner (PhotoCraft's Move tool controls, DesignCraft).
pub const TRANSFORM_BAND_INNER: f32 = 5.0;
/// Outer edge of the rotate band outside a corner.
pub const TRANSFORM_BAND_OUTER: f32 = 18.0;
/// Farthest a press outside the box may move and still be a click that applies the box.
pub const TRANSFORM_CLICK_SLOP: f32 = 2.0;
/// Radius of the reference point's circle.
pub const TRANSFORM_PIVOT_RADIUS: f32 = 5.0;
/// Length of each arm of the reference point's cross.
pub const TRANSFORM_PIVOT_ARM: f32 = 8.0;
/// Radius of the rotate cursor's arc.
pub const TRANSFORM_TURN_RADIUS: f32 = 12.0;
/// Width of the rotate cursor's arc.
pub const TRANSFORM_TURN_LINE: f32 = 1.5;
/// Width of the halo on each side of the rotate cursor's arc.
pub const TRANSFORM_TURN_HALO: f32 = 1.0;
/// Width of a one-patch warp mesh's rule-of-thirds lines.
pub const TRANSFORM_WARP_MINOR: f32 = 0.75;
/// Side of a warp mesh's anchor squares.
pub const TRANSFORM_WARP_ANCHOR: f32 = 5.0;
/// Diameter of a warp mesh's handle ends.
pub const TRANSFORM_WARP_CONTROL: f32 = 5.5;
/// Distance from the pointer within which a warp handle shows (PhotoCraft's `HANDLE_REVEAL_PX`).
pub const TRANSFORM_WARP_REVEAL: f32 = 56.0;

/// Width of a path's outline, in points, at every Size.
pub const BEZIER_LINE: f32 = 1.0;
/// Width of a selected segment (VectorCraft's).
pub const BEZIER_SELECTED_SEGMENT: f32 = 2.0;
/// Width of the path the pen is drawing.
pub const BEZIER_DRAFT_LINE: f32 = 1.5;
/// Side of an anchor square.
pub const BEZIER_ANCHOR: f32 = 6.0;
/// Diameter of a direction handle's end.
pub const BEZIER_CONTROL: f32 = 6.0;
/// Width of the line from an anchor to its direction handle's end.
pub const BEZIER_CONTROL_LINE: f32 = 1.0;
/// Distance within which the pointer takes an anchor, handle end or segment (PhotoCraft's).
pub const BEZIER_HIT: f32 = 6.0;
/// Farthest a press on a path may move and still be a click (PhotoCraft's).
pub const BEZIER_CLICK_SLOP: f32 = 1.0;
/// Largest distance between a curve and the lines it is drawn with.
pub const BEZIER_FLATTEN: f32 = 0.3;
/// Dash length of the pen's rubber band.
pub const BEZIER_RUBBER_DASH: f32 = 4.0;
/// Gap between the rubber band's dashes.
pub const BEZIER_RUBBER_GAP: f32 = 3.0;
/// Side of the first anchor while a press there would close the path.
pub const BEZIER_CLOSE_ANCHOR: f32 = 8.0;
/// Radius of the pen's close cue ring.
pub const BEZIER_CLOSE_RING: f32 = 3.5;
/// Distance across and down from the pointer to the close cue ring.
pub const BEZIER_CLOSE_OFFSET: f32 = 11.0;

/// Width of the gradient line.
pub const GRADIENT_LINE_LINE: f32 = 1.5;
/// Width of the dark halo on each side of the line.
pub const GRADIENT_LINE_HALO: f32 = 0.75;
/// Radius of an end handle's ring.
pub const GRADIENT_LINE_END_RING: f32 = 8.0;
/// Width of an end handle's ring.
pub const GRADIENT_LINE_END_WIDTH: f32 = 2.0;
/// Width of the dark halo on each side of an end handle's ring.
pub const GRADIENT_LINE_END_HALO: f32 = 0.75;
/// Radius of a colour stop's disc.
pub const GRADIENT_LINE_STOP: f32 = 5.0;
/// Width of the ring round a colour stop's disc.
pub const GRADIENT_LINE_STOP_RING: f32 = 1.5;
/// Radius of a midpoint's diamond.
pub const GRADIENT_LINE_DIAMOND: f32 = 4.5;
/// Distance from a part within which the pointer takes it.
pub const GRADIENT_LINE_HIT: f32 = 8.0;
/// Farthest a press may move and still be a click.
pub const GRADIENT_LINE_CLICK_SLOP: f32 = 2.0;
/// Distance from the line past which a dragged stop is torn off.
pub const GRADIENT_LINE_TEAR_OFF: f32 = 36.0;
/// Dash length of a mirrored extent's line.
pub const GRADIENT_LINE_DASH: f32 = 5.0;
/// Gap between a mirrored extent's dashes.
pub const GRADIENT_LINE_GAP: f32 = 4.0;

/// Side of the compact reference point picker, in points, at every Size (PhotoCraft's options bar).
pub const REF_POINT_COMPACT: f32 = 16.0;
/// Side of the large reference point picker (PhotoCraft's Canvas Size).
pub const REF_POINT_LARGE: f32 = 72.0;
/// Side of a reference point picker's square dots.
pub const REF_POINT_DOT: f32 = 3.5;

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

/// Width of a thin floating panel scroll bar (PhotoCraft's `floating_width`, `theme.rs:1090`).
pub const SCROLL_FLOATING_WIDTH: f32 = 4.0;
/// Handle opacity of a thin bar under the pointer (`theme.rs:1088`): the stock thin style fades
/// the handle to the track's opacity, so the thumb would disappear.
pub const SCROLL_HANDLE_OPACITY: f32 = 1.0;
/// Track opacity of a thin bar under the pointer (`theme.rs:1089`).
pub const SCROLL_TRACK_OPACITY: f32 = 0.4;
/// Thickness of a canvas scroll bar in points (`canvas/scrollbars.rs:20`).
pub const SCROLL_CANVAS_WIDTH: f32 = 12.0;
/// Between a canvas bar's edge and its thumb, in points.
pub const SCROLL_THUMB_INSET: f32 = 3.0;
/// Shortest canvas thumb along its bar, in points (`canvas/scrollbars.rs:22`).
pub const SCROLL_THUMB_MIN: f32 = 24.0;
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

/// The control height the Compact options bar sizes are set for.
pub const OPTIONS_BAR_BASE_CONTROL: f32 = 24.0;
/// Space at the ends of the options bar (`panels/options_bar.rs:47`), the same at every Size.
pub const OPTIONS_BAR_PAD: f32 = 10.0;
/// Between two rows of an options bar with more than one row (the Compact bar's 36 holds a 26 button).
pub const OPTIONS_BAR_ROW_GAP: f32 = 4.0;
/// Between the items of the options bar and between a caption and its control (`theme.rs:1073`).
pub const OPTIONS_BAR_GAP: f32 = 8.0;
/// Length of a divider between groups of the options bar (`widgets::vline(ui, 22.0)`).
pub const OPTIONS_BAR_DIVIDER: f32 = 22.0;
/// Width classes of the options bar's fields: the 50, 54 and 56 wide ones, the 62 wide ones and the widest.
pub const OPTIONS_BAR_FIELD_SM: f32 = 58.0;
pub const OPTIONS_BAR_FIELD_MD: f32 = 62.0;
pub const OPTIONS_BAR_FIELD_LG: f32 = 76.0;
/// Width of the options bar's floating scroll bar (`panels/options_bar.rs:49`), the same at every Size.
pub const OPTIONS_BAR_SCROLL_BAR: f32 = 4.0;

/// Space at the ends of the status bar (`panels/status_bar.rs:19`), the same at every Size.
pub const STATUS_BAR_PAD: f32 = 10.0;
/// Between the items of the status bar (`theme.rs:1073`), Compact.
pub const STATUS_BAR_GAP: f32 = 8.0;
/// Added after the zoom field (`panels/chrome_ui.rs`, `status_bar_pro`: `ui.add_space(12.0)`), Compact.
pub const STATUS_BAR_GROUP_GAP: f32 = 12.0;
/// Length of a divider in the status bar (`widgets::vline(ui, 16.0)`), Compact.
pub const STATUS_BAR_DIVIDER: f32 = 16.0;
/// Width of the zoom field cell: PhotoCraft's Pro 64 at Compact (`chrome_ui.rs`, `status_bar_pro`), its 78 at Comfortable.
pub const fn status_bar_field_width(size: Size) -> f32 {
    match size {
        Size::Compact => 64.0,
        Size::Comfortable => 78.0,
    }
}

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

/// Corner radius of a check box, in points, in every theme.
pub const CHECK_RADIUS: f32 = 2.0;
/// Largest corner radius a theme may give a check box, in points: a small box keeps square
/// corners and never reads as a radio button.
pub const CHECK_RADIUS_MAX: f32 = 3.0;
/// How far under the standard icon size a check box or radio button is, in points.
pub const CHECK_BOX_INSET: f32 = 2.0;
/// Width of a check box's outline, in points, drawn inside the box.
pub const CHECK_OUTLINE: f32 = 1.5;
/// Stroke width of a check box's check and dash, as a fraction of the box.
pub const CHECK_MARK_RATIO: f32 = 0.125;
/// How far toward black an unchecked box's fill goes while pressed.
pub const CHECK_OFF_PRESSED_DARKEN: f32 = 0.18;
/// Diameter of a radio button's dot as a fraction of the circle's.
pub const RADIO_DOT_RATIO: f32 = 0.4;
/// Diameter of an icon button's badge dot as a fraction of the Size's icon.
pub const ICON_BUTTON_BADGE_RATIO: f32 = 0.4;
/// How much smaller an icon button's glyph is than the Size's icon, (small, medium). PhotoCraft
/// draws the glyph at round(0.52 x the box), 10 in a 20 box and 12 in a 24 box (`icons.rs:175`),
/// which is the Compact icon (12, 16) less these; a larger Size keeps the step.
pub const ICON_BUTTON_GLYPH_LESS: (f32, f32) = (2.0, 4.0);
/// Width of a switch's track as a multiple of its height.
pub const SWITCH_ASPECT: f32 = 2.0;
/// Between a switch's track edge and its thumb, in points, at every Size.
pub const SWITCH_THUMB_INSET: f32 = 2.0;

/// Largest corner radius of a swatch, in points: `shape.radius.control` up to this.
pub const SWATCH_RADIUS_MAX: f32 = 4.0;
/// Width of a swatch's selection ring, drawn inside the well's outline.
pub const SWATCH_SELECTED_WIDTH: f32 = 2.0;
/// Opacity of the black or white ring a colour pair draws inside a swatch's edge.
pub const SWATCH_INNER_EDGE_ALPHA: u8 = 0x80;
/// The two checkerboard greys behind a translucent swatch, the same in every theme (PhotoCraft's
/// `checker_tile`: `widgets/color.rs:67`).
pub const SWATCH_CHECKER: (Rgba, Rgba) = (Rgba::rgb(250, 250, 250), Rgba::rgb(214, 214, 214));
/// Side of one checkerboard cell behind a translucent swatch (`widgets/color.rs:92`).
pub const SWATCH_CHECKER_CELL: f32 = 4.0;

/// Above and below a badge's caption line, in points.
pub const BADGE_PAD_Y: f32 = 1.0;

/// Largest corner radius of a keycap, in points.
pub const KBD_RADIUS_MAX: f32 = 3.0;
/// Above and below a keycap's caption line, in points.
pub const KBD_PAD_Y: f32 = 1.0;

/// Thickness of a progress bar's track, in points, at every Size (PhotoCraft's `bar_h`, `jobs_ui.rs:631`).
pub const PROGRESS_TRACK: f32 = 6.0;

/// Widest a tooltip's text grows before it wraps, in points (PhotoCraft's `tooltip_width`, `theme.rs:1093`).
pub const TOOLTIP_WIDTH: f32 = 280.0;

/// Outer width of a toast card, in points, at every Size.
pub const TOAST_WIDTH: f32 = 360.0;

/// Between a toast card's edge and its content, in points: Compact, Comfortable.
pub const fn toast_pad(size: Size) -> f32 {
    match size {
        Size::Compact => 10.0,
        Size::Comfortable => 13.0,
    }
}

/// From the host rect's reading-end edge and from its bottom edge to the toast stack, in points:
/// Compact, Comfortable.
pub const fn toast_inset(size: Size) -> [f32; 2] {
    match size {
        Size::Compact => [16.0, 12.0],
        Size::Comfortable => [20.0, 15.0],
    }
}

/// Offset of a toast card's shadow, in points (PhotoCraft `notices.rs:238`: offset (0, 2)).
pub const TOAST_SHADOW_OFFSET: [f32; 2] = [0.0, 2.0];
/// Blur of a toast card's shadow, in points (`notices.rs:238`).
pub const TOAST_SHADOW_BLUR: f32 = 8.0;

/// Widest the column of a centred empty state grows, in points, at every Size.
pub const EMPTY_STATE_MAX_WIDTH: f32 = 460.0;

/// The control height the Compact slider row sizes are set for; other sizes scale them by their
/// control height over this.
pub const SLIDER_ROW_BASE_CONTROL: f32 = 24.0;
/// Compact width of a slider row's number field (PhotoCraft `slider_row_prec`).
pub const SLIDER_ROW_FIELD_WIDTH: f32 = 74.0;
/// Compact space after a slider row.
pub const SLIDER_ROW_GAP: f32 = 4.0;
