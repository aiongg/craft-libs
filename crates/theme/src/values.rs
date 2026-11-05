//! Every tunable number of the semantic layer in one place: the ramp step each colour token
//! takes, fixed colours, contrast minimums, Size presets, type roles, text-size factors,
//! elevation and motion choices. The gallery tunes these; nothing else should hold such values.

use super::color::Rgba;
use super::ramp::Mode;
use super::scale::{Duration, Easing, Elevation};
use super::tokens::{ColorToken, StatusKind};

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
        Accent, AccentMix, Anchored, Fixed, Lighting, Neutral, Oklch, OnSolid, Same, ShiftedFrom, SolidFrom, StateVariant, Status, StatusMix,
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
        T::SurfaceCanvasPattern => Same(T::SurfaceCanvas),
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
        // The crop frame sits on a dimmed document, so its colours do not step with the mode.
        T::OverlayCrop => Fixed(Rgba::WHITE),
        T::OverlayCropGuide => Fixed(Rgba::WHITE.with_alpha(CROP_GUIDE_ALPHA)),
        T::OverlayCropInvalid => Fixed(Rgba::rgb(0xe5, 0x3e, 0x3e)),
        // Drawn over the document, not the panel, so one colour in both modes (PhotoCraft's default).
        T::OverlayGrid => Fixed(Rgba::rgb(0x8c, 0x8c, 0x8c)),
        // CadCraft's pinned grip colours; grips sit on the document, so one value in both modes.
        T::OverlayGrip => Fixed(Rgba::rgb(0x2f, 0x6b, 0xff)),
        T::OverlayGripWarm => Fixed(Rgba::rgb(0xff, 0x7f, 0x9f)),
        T::OverlayGripHot => Fixed(Rgba::rgb(0xff, 0x3b, 0x3b)),
        T::OverlayGripDyn => Fixed(Rgba::rgb(0x26, 0xc6, 0xe8)),
        // Warp pins and mesh sit on the document, so one value in both modes (PhotoCraft `puppet_ui.rs:367-388`).
        T::OverlayPin => Fixed(Rgba::rgb(0xff, 0xd6, 0x0a)),
        T::OverlayPinRing => Fixed(Rgba::rgb(0x00, 0x00, 0x00)),
        T::OverlayPinTurn => Fixed(Rgba::rgb(0xff, 0xff, 0xff)),
        T::OverlayMesh => Fixed(Rgba([40, 40, 40, 150])),
        // Slices and artboard edges sit on the document, so one value in both modes (PhotoCraft `slice_ui.rs:242-321`, `artboard_ui.rs:97`).
        T::OverlaySlice => Fixed(Rgba::rgb(0x38, 0xb5, 0xff)),
        T::OverlaySliceSelected => Fixed(Rgba::rgb(0xff, 0xb0, 0x00)),
        T::OverlaySliceAuto => Fixed(Rgba::rgb(110, 110, 110)),
        T::OverlaySliceText => Fixed(Rgba::rgb(0xff, 0xff, 0xff)),
        T::OverlayArtboard => Fixed(Rgba([0, 0, 0, 90])),
        T::OverlayGripStroke => Fixed(Rgba::rgb(0x10, 0x30, 0x80)),
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

/// The control height the Compact panel footer sizes are set for.
pub const PANEL_FOOTER_BASE_CONTROL: f32 = 24.0;
/// Compact height of a panel footer's bar (PhotoCraft's `FOOTER_BAR`, `widgets/panel.rs:58`).
pub const PANEL_FOOTER_HEIGHT: f32 = 30.0;
/// Compact gap between a panel footer's buttons.
pub const PANEL_FOOTER_GAP: f32 = 2.0;
/// Compact side of a panel footer's button.
pub const PANEL_FOOTER_BUTTON: f32 = 26.0;

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

/// Least width left to an inspector header's name before its kind text is dropped, in points.
pub const INSPECTOR_KIND_MIN_NAME: f32 = 60.0;
/// Taken from a toolbar button's side for the space between two buttons laid out by the toolbar, in points.
pub const TOOLBAR_SPACING_TRIM: f32 = 2.0;
/// Width and height of the chevron slot beside the options bar's tool icon, in points, at every Size.
pub const OPTIONS_BAR_CHEVRON: [f32; 2] = [10.0, 20.0];
/// Room under an options bar's content for font ink, in points.
pub const OPTIONS_BAR_INK_ROOM: f32 = 2.0;
/// Width of a preferences window's page list column, at every Size (PhotoCraft's `prefs_ui` side list).
pub const PREFERENCES_LIST_WIDTH: f32 = 170.0;
/// Width of a choice row's control at Compact (PhotoCraft's select column); Comfortable scales it by the control height.
pub const PREFERENCES_CHOICE_WIDTH: f32 = 220.0;
/// Width of a text row's control at Compact; Comfortable scales it by the control height.
pub const PREFERENCES_TEXT_WIDTH: f32 = 260.0;
/// Room between the divider and a preferences page's contents, and between them and the window's trailing edge, at every Size (PhotoCraft: a 9 pt rule, then `item_spacing`).
pub const PREFERENCES_PAGE_INSET: f32 = 13.0;
/// Height of a page list row, at every Size (PhotoCraft: 22 pt rows 6 pt apart).
pub const PREFERENCES_PAGE_ROW_HEIGHT: f32 = 28.0;
/// Width of a colour row's hex field at Compact (PhotoCraft's `hex_field`); Comfortable scales it by the control height.
pub const PREFERENCES_HEX_WIDTH: f32 = 72.0;
/// Inset of a `.checks()` group's check boxes from the label column, at every Size (PhotoCraft: the 24 pt least column width of the grid that holds them, with no column gap).
pub const PREFERENCES_CHECKS_INSET: f32 = 24.0;
/// Height given to a page body laid out in an unbounded rect, in points.
pub const PREFERENCES_BODY_FALLBACK_HEIGHT: f32 = 600.0;

/// Least thumbnail side of a preset list row, at every Size.
pub const PRESET_THUMB_LIST_MIN: f32 = 16.0;

/// Greatest thumbnail side of a preset list row, at every Size.
pub const PRESET_THUMB_LIST_MAX: f32 = 64.0;

/// Side of the thumbnail in a preset's drag ghost (PhotoCraft's 28).
pub const PRESET_GHOST_THUMB: f32 = 28.0;

/// Width of the ring outside a hovered preset cell.
pub const PRESET_RING_HOVER_WIDTH: f32 = 1.0;

/// Width of the `overlay.halo` line outside the selected preset's ring.
pub const PRESET_RING_HALO_WIDTH: f32 = 1.0;

/// The control height the Compact form grid gaps are set for; other sizes scale them by their
/// control height over this.
pub const FORM_BASE_CONTROL: f32 = 24.0;
/// Compact gap between a form grid's label column and its control column: the spacing of
/// PhotoCraft's `egui::Grid` forms (`panels/properties.rs:116`).
pub const FORM_COLUMN_GAP: f32 = 12.0;
/// Compact gap between a form grid's rows (same source).
pub const FORM_ROW_GAP: f32 = 6.0;
/// Compact least width of a form grid's label column: none, the widest label sets it.
pub const FORM_LABEL_MIN: f32 = 0.0;
/// Compact column gap of the roomy form grid, a preferences page's: PhotoCraft's
/// `Grid::spacing([14.0, 7.0])` (`prefs/prefs_ui/fields.rs:327`).
pub const FORM_ROOMY_COLUMN_GAP: f32 = 14.0;
/// Compact row gap of the roomy form grid (same source).
pub const FORM_ROOMY_ROW_GAP: f32 = 7.0;
/// Compact least width of the roomy form grid's label column: PhotoCraft's
/// `Grid::min_col_width(150.0)` (same source).
pub const FORM_ROOMY_LABEL_MIN: f32 = 150.0;

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
/// Least width or height of a pane in a split.
pub const DOCK_MIN_PANE: f32 = 40.0;
/// Least width of the drag bar left of a group's header.
pub const DOCK_DRAG_BAR_WIDTH: f32 = 40.0;
/// Widest an iconic column gets by dragging its inner edge.
pub const DOCK_ICONIC_MAX_WIDTH: f32 = 240.0;
/// Width from which an iconic column shows panel names beside the icons.
pub const DOCK_ICONIC_LABEL_WIDTH: f32 = 96.0;
/// Height of an iconic column's flyout without a saved height.
pub const DOCK_FLYOUT_HEIGHT: f32 = 320.0;
/// Width of a group's tab strip overflow button.
pub const DOCK_OVERFLOW_WIDTH: f32 = 22.0;
/// Tallest list in a tab overflow or move menu.
pub const DOCK_MENU_MAX_HEIGHT: f32 = 360.0;
/// Width of a floating window by default.
pub const FLOAT_WIDTH: f32 = 260.0;
/// Height of a floating window by default, and the tallest a docked panel floats at.
pub const FLOAT_HEIGHT: f32 = 320.0;
/// Fitted to its content, a floating window is at least this tall.
pub const FLOAT_MIN_FITTED_HEIGHT: f32 = 120.0;
/// How far a window a command floats sits from the dock area's top-left corner, and from the
/// window it cascades from.
pub const FLOAT_CASCADE_OFFSET: f32 = 24.0;
/// Distance from the pointer to the centre of the float hint.
pub const FLOAT_HINT_OFFSET: f32 = 28.0;
/// Narrowest a resized floating window gets.
pub const FLOAT_MIN_WIDTH: f32 = 40.0;
/// Room for content that a resized floating window keeps below its title bar.
pub const FLOAT_MIN_BODY_HEIGHT: f32 = 20.0;
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
pub const COLOR_PICKER_PAIR_WELL: f32 = 18.0;
/// Compact offset of a swatch pair's back well from its front well, on both axes.
pub const COLOR_PICKER_PAIR_STEP: f32 = 10.0;
/// Compact width of the gap around a swatch pair's front well.
pub const COLOR_PICKER_PAIR_GAP: f32 = 2.0;
/// Compact width of a swatch pair's ring well.
pub const COLOR_PICKER_PAIR_RING: f32 = 7.0;
/// Side of the Defaults and Swap icon boxes over a swatch pair's chips, in points, at every Size.
pub const COLOR_PICKER_PAIR_ICON: f32 = 13.0;
/// Between those icon boxes and the chips, in points, at every Size.
pub const COLOR_PICKER_PAIR_ICON_GAP: f32 = 3.0;
/// Compact width of the colour readout's hex field (PhotoCraft's Color panel: six digits).
pub const COLOR_PICKER_READOUT_HEX_WIDTH: f32 = 64.0;
/// Compact width of the colour readout's R, G and B fields.
pub const COLOR_PICKER_READOUT_CHANNEL_WIDTH: f32 = 38.0;
/// Compact gap between the Full picker's side-actions block and its channel grid (PhotoCraft: the
/// 4 point item spacing plus a 4 point space).
pub const COLOR_PICKER_GRID_GAP: f32 = 8.0;
/// Compact gap between the channel grid's rows (PhotoCraft's grid spacing).
pub const COLOR_PICKER_GRID_ROW_GAP: f32 = 2.0;
/// Compact gap between one model's last channel row and the next model's first (PhotoCraft: a 3
/// point row between two row gaps).
pub const COLOR_PICKER_GRID_GROUP_GAP: f32 = 7.0;

/// The control height the Compact colour wheel, ramp and gradient palette sizes are set for;
/// other sizes scale them by their control height over this.
pub const COLOR_WHEEL_BASE_CONTROL: f32 = 24.0;
/// Compact default height of a colour wheel's area (PhotoCraft's `min(220)`).
pub const COLOR_WHEEL_SIDE: f32 = 220.0;
/// Compact least height of a colour wheel's area.
pub const COLOR_WHEEL_SIDE_MIN: f32 = 28.0;
/// Compact least width of the hue ring.
pub const COLOR_WHEEL_RING_MIN: f32 = 6.0;
/// Compact greatest width of the hue ring.
pub const COLOR_WHEEL_RING_MAX: f32 = 22.0;
/// Ring width as a proportion of the outer radius; a proportion, so no size scales it.
pub const COLOR_WHEEL_RING_RATIO: f32 = 0.09;
/// Compact distance from the area's edge to the ring's outer edge.
pub const COLOR_WHEEL_OUTER_INSET: f32 = 2.0;
/// Compact distance from the ring's inner edge to the triangle's circumscribed circle.
pub const COLOR_WHEEL_TRIANGLE_INSET: f32 = 1.0;
/// Compact distance outside the ring's outer edge within which a press still takes the ring.
pub const COLOR_WHEEL_GRAB_OUTER: f32 = 7.0;
/// Compact distance inside the ring's inner edge from which a press takes the ring.
pub const COLOR_WHEEL_GRAB_INNER: f32 = 2.0;
/// Compact distance from a colour disc's area edge to the disc.
pub const COLOR_WHEEL_DISC_INSET: f32 = 10.0;
/// Compact radius of a colour disc's marker.
pub const COLOR_WHEEL_DISC_MARKER: f32 = 5.5;
/// Compact radius of a colour disc's selected marker.
pub const COLOR_WHEEL_DISC_MARKER_SELECTED: f32 = 8.0;
/// Compact distance from a press within which a colour disc grabs its nearest marker.
pub const COLOR_WHEEL_DISC_GRAB: f32 = 14.0;
/// Least brightness a colour disc draws, so the disc never goes black; a proportion.
pub const COLOR_WHEEL_DISC_FLOOR: f32 = 0.15;
/// A colour disc's spokes: white at alpha 160.
pub const COLOR_WHEEL_SPOKE: Rgba = Rgba([255, 255, 255, 160]);

/// Compact height of a colour ramp (PhotoCraft's colour panel).
pub const COLOR_RAMP_HEIGHT: f32 = 36.0;
/// Compact least height of a colour ramp.
pub const COLOR_RAMP_HEIGHT_MIN: f32 = 28.0;
/// Compact width of the white and black boxes at a ramp's end.
pub const COLOR_RAMP_BOXES: f32 = 16.0;
/// Compact gap between a ramp's bar and its boxes.
pub const COLOR_RAMP_BOXES_GAP: f32 = 2.0;
/// Cells across a ramp's bar; a count, so no size scales it.
pub const COLOR_RAMP_CELLS_X: u32 = 48;
/// Cells down a ramp's bar; a count, so no size scales it.
pub const COLOR_RAMP_CELLS_Y: u32 = 8;

/// Compact side of a swatch palette's table cell (PhotoCraft's Save for Web colour table).
pub const SWATCH_PALETTE_TABLE_CELL: f32 = 14.0;

/// Compact width of a gradient palette's grid cell (PhotoCraft's gradient popover).
pub const GRADIENT_PALETTE_CELL_WIDTH: f32 = 32.0;
/// Compact height of a gradient palette's grid cell.
pub const GRADIENT_PALETTE_CELL_HEIGHT: f32 = 18.0;
/// Compact width of a gradient palette's list swatch (PhotoCraft's Gradient Map list).
pub const GRADIENT_PALETTE_LIST_SWATCH_WIDTH: f32 = 48.0;
/// Compact height of a gradient palette's list swatch.
pub const GRADIENT_PALETTE_LIST_SWATCH_HEIGHT: f32 = 16.0;
/// Compact height at which a gradient palette scrolls.
pub const GRADIENT_PALETTE_MAX_HEIGHT: f32 = 120.0;
/// Room round each palette cell, in points, at every Size.
pub const GRADIENT_PALETTE_CELL_ROOM: f32 = 3.0;

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
/// Opacity of `state.hover` as the stripe of a striped list (PdfCraft's `striped(true)` fills with
/// egui's `faint_bg_color`, white at 2% in both modes: "visible, but barely so"); lighter than the
/// hover fill.
pub const LIST_STRIPE_OPACITY: f32 = 0.25;
/// Opacity of the accent tint over the hover fill of a layer-form row (`layer_row.rs:286`).
pub const LIST_LAYER_TINT_OPACITY: f32 = 0.12;
/// Height of a dense list row, in a panel's short list (PhotoCraft's `styles_panel` and
/// `spelling_window` rows).
pub const LIST_ROW_HEIGHT_DENSE: f32 = 22.0;
/// Added to a list row's thumbnail side for the row's height.
pub const LIST_THUMB_PAD: f32 = 8.0;
/// Width of a list's drop marker line and outline (PhotoCraft, DesignCraft and docking).
pub const LIST_DROP_WIDTH: f32 = 2.0;
/// Width of the halo on each side of a list's drop marker, as `handle.halo` around a handle.
pub const LIST_DROP_HALO: f32 = 1.0;
/// How far the second-thumb brackets stand outside the thumbnail.
pub const LIST_BRACKET_OUTSET: f32 = 3.0;
/// Arm length of the second-thumb brackets.
pub const LIST_BRACKET_ARM: f32 = 6.0;
/// Where a drag ghost's box starts from the pointer toward the reading end, in points (PhotoCraft).
pub const LIST_GHOST_OFFSET: [f32; 2] = [12.0, -10.0];
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
/// Between the labels of a text strip's tabs, in points (PhotoCraft's category tabs,
/// `new_doc_ui.rs`, `item_spacing.x`).
pub const TAB_TEXT_GAP: f32 = 18.0;
/// Height of a text strip, in points: the label's line, 6 pt, and the 1 pt hairline
/// (`new_doc_ui.rs`), so the underline's stroke ends 2 pt above the hairline.
pub const TAB_TEXT_HEIGHT: f32 = 23.0;
/// Thickness of a text strip's selected-tab underline, in points.
pub const TAB_UNDERLINE_WIDTH: f32 = 2.0;
/// From the bottom of a text tab's label to the centre of its underline, in points.
pub const TAB_UNDERLINE_GAP: f32 = 3.0;
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
/// Length of each arm of the painting tools' precise crosshair, which has no gap (PhotoCraft's).
pub const CURSOR_PRECISE_REACH: f32 = 6.0;
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

/// Line width of the bitmap pointers (PhotoCraft's `tool_cursor`).
pub const CURSOR_STROKE: f32 = 1.0;
/// Side of the badge beside a tool icon or the arrow (PhotoCraft's `icons::cursor_badge`).
pub const CURSOR_BADGE: f32 = 10.0;
/// Side of the tool icon drawn as a pointer (PhotoCraft's `icons::cursor`).
pub const CURSOR_ICON: f32 = 16.0;

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

/// Width of Guide Layout's Number field (PhotoCraft).
pub const GUIDE_NUMBER_WIDTH: f32 = 68.0;

/// Width of Guide Layout's Width and Height fields (PhotoCraft).
pub const GUIDE_SIZE_WIDTH: f32 = 60.0;

/// Width of the field of each Margin side in Guide Layout (PhotoCraft).
pub const GUIDE_MARGIN_WIDTH: f32 = 64.0;

/// Width of New Guide's Position field (PhotoCraft).
pub const GUIDE_POSITION_WIDTH: f32 = 120.0;

/// Width of New Guide's Color list (PhotoCraft).
pub const GUIDE_COLOR_WIDTH: f32 = 140.0;

/// Width of Guide Layout's Preset and Target lists (PhotoCraft).
pub const GUIDE_LAYOUT_LIST_WIDTH: f32 = 222.0;

/// Width of Guide Layout's Color list (PhotoCraft).
pub const GUIDE_LAYOUT_COLOR_WIDTH: f32 = 174.0;

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
/// Diameter of a Puppet Warp pin's disc (PhotoCraft radius 5.5): `transform.pin`.
pub const TRANSFORM_PIN: f32 = 11.0;
/// Width of a pin's ring: `transform.pin_ring`.
pub const TRANSFORM_PIN_RING: f32 = 1.25;
/// Diameter of the selected pin's core (radius 2): `transform.pin_core`.
pub const TRANSFORM_PIN_CORE: f32 = 4.0;
/// Radius of the fixed-rotation ring and reach of the Option turn: `transform.pin_turn`.
pub const TRANSFORM_PIN_TURN: f32 = 18.0;
/// Width of the fixed-rotation ring and its line: `transform.pin_turn_line`.
pub const TRANSFORM_PIN_TURN_LINE: f32 = 1.0;
/// Width of the pin mesh's lines: `transform.mesh_line`.
pub const TRANSFORM_MESH_LINE: f32 = 0.6;
/// Diameter of a Perspective Warp plane's corner disc (radius 4.5): `transform.plane_corner`.
pub const TRANSFORM_PLANE_CORNER: f32 = 9.0;
/// Width of a plane corner's ring: `transform.plane_ring`.
pub const TRANSFORM_PLANE_RING: f32 = 1.25;

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

/// Side of the Rotate View compass, in points (PhotoCraft's `rotate_view.rs` `S`).
pub const CANVAS_COMPASS: f32 = 36.0;
/// Distance of the compass from the canvas's top and right edges (PhotoCraft's `M`).
pub const CANVAS_COMPASS_MARGIN: f32 = 10.0;

/// Width of the Change To field and the suggestion list (PhotoCraft's `spelling_window`).
pub const CHECK_SPELLING_FIELD_WIDTH: f32 = 240.0;
/// Height of the suggestion list (PhotoCraft's `spelling_window`).
pub const CHECK_SPELLING_SUGGESTIONS_HEIGHT: f32 = 110.0;

/// Default height of the open list (PhotoCraft's `FONT_LIST_HEIGHT`, `FONT_LIST_WIDTH` and `CLASS_COLUMN_WIDTH`).
pub const FONT_PICKER_LIST_HEIGHT: f32 = 440.0;
/// Least width of the open list (PhotoCraft's `FONT_LIST_HEIGHT`, `FONT_LIST_WIDTH` and `CLASS_COLUMN_WIDTH`).
pub const FONT_PICKER_LIST_WIDTH: f32 = 420.0;
/// Width of the class list (PhotoCraft's `FONT_LIST_HEIGHT`, `FONT_LIST_WIDTH` and `CLASS_COLUMN_WIDTH`).
pub const FONT_PICKER_CLASS_COLUMN: f32 = 118.0;
/// Width of a row's class glyph (PhotoCraft's `FONT_LIST_HEIGHT`, `FONT_LIST_WIDTH` and `CLASS_COLUMN_WIDTH`).
pub const FONT_PICKER_CLASS_GLYPH: f32 = 28.0;
/// Height of the Missing Fonts dialog's Document Fonts list (new; Photoshop's list shows about seven rows).
pub const FONT_PICKER_MISSING_LIST_HEIGHT: f32 = 160.0;

/// Height of the list (PhotoCraft's `styles_panel`).
pub const TEXT_STYLES_LIST_HEIGHT: f32 = 220.0;
/// The side of a style tile in the picker's grid view (FigCraft `widgets/style_picker.rs:34` `SWATCH`).
pub const TEXT_STYLES_TILE_SIDE: f32 = 24.0;
/// The side of a style's preview square in the chip and the list rows (FigCraft `widgets/style_picker.rs:226`, `:432`).
pub const TEXT_STYLES_PREVIEW_SIDE: f32 = 14.0;

/// Height of the Glyphs panel's grid, in points (PhotoCraft's `glyphs_panel` scroll area).
pub const GLYPHS_GRID_HEIGHT: f32 = 260.0;
/// Glyph cell size when the settings have none.
pub const GLYPHS_CELL: f32 = 32.0;
/// Smallest and largest cell of the zoom slider.
pub const GLYPHS_CELL_MIN: f32 = 24.0;
pub const GLYPHS_CELL_MAX: f32 = 64.0;
/// Space between a cell's edge and its glyph.
pub const GLYPHS_GLYPH_INSET: f32 = 3.0;
/// Side of a Recently Used cell, and the distance between cells.
pub const GLYPHS_RECENT_CELL: f32 = 24.0;
pub const GLYPHS_RECENT_PITCH: f32 = 26.0;
/// Space between the Recently Used strip's edge and its first cell and top.
pub const GLYPHS_RECENT_PAD: f32 = 1.0;
/// Space between a Recently Used cell's edge and its glyph.
pub const GLYPHS_RECENT_GLYPH_INSET: f32 = 2.0;

/// Width of the text caret's halo and line, in points (PhotoCraft's `CARET_STROKES`).
pub const TEXT_EDITOR_CARET_HALO: f32 = 3.0;
pub const TEXT_EDITOR_CARET_LINE: f32 = 1.0;
/// Share of the accent the selection fill keeps (alpha 110 of 255).
pub const TEXT_EDITOR_SELECTION_ALPHA: f32 = 110.0 / 255.0;
/// Underline widths of marked text and of the segment being converted.
pub const TEXT_EDITOR_PREEDIT_UNDERLINE: f32 = 1.5;
pub const TEXT_EDITOR_ACTIVE_UNDERLINE: f32 = 3.0;
/// The size of the notes' default text, points: a notes indent in points scales as `size / this` (DeckCraft `panels/notes.rs:18`, `:118`).
pub const TEXT_EDITOR_NOTES_POINTS: f32 = 12.0;
/// The largest notes indent, points (DeckCraft `panels/notes.rs:49`).
pub const TEXT_EDITOR_NOTES_MAX_INDENT: f32 = 400.0;

/// Points between split view panes (EffectCraft's `view.rs`).
pub const SPLIT_GAP: f32 = 2.0;
/// Width of the line round the active split pane.
pub const SPLIT_ACTIVE_EDGE: f32 = 1.0;

/// Points between a page's top edge and its name on the pasteboard.
pub const PASTEBOARD_NAME_GAP: f32 = 4.0;
/// Width of a page's outline on the pasteboard (Border: Line).
pub const PASTEBOARD_OUTLINE_WIDTH: f32 = 1.0;

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
/// Half the width of a text-ruler marker (WordCraft `canvas/rulers.rs:75`, `first - 4.5`).
pub const RULER_MARKER_HALF: f32 = 4.5;
/// The height of a text-ruler indent triangle (WordCraft `canvas/rulers.rs:75`, `bar.min.y + 5.0`).
pub const RULER_MARKER_TIP: f32 = 5.0;
/// The height of the left indent's box under its triangle (WordCraft `canvas/rulers.rs:80-86`, `bar.max.y - 3.0` to `bar.max.y + 1.0`).
pub const RULER_MARKER_BAR: f32 = 4.0;
/// The height of a tab stop's stem (WordCraft `ruler_tabs.rs:91`, `base - 5.0`).
pub const RULER_MARKER_STEM: f32 = 5.0;
/// The length of a tab stop's foot, to either side of the stem (WordCraft `ruler_tabs.rs:82`).
pub const RULER_MARKER_FOOT: f32 = 4.0;
/// The side of the tab selector's box (WordCraft `ruler_tabs.rs:129`).
pub const RULER_MARKER_SELECTOR: f32 = 14.0;

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

/// Width of the command palette's box (PhotoCraft `dialogs/palette.rs`), the same at every Size.
pub const COMMAND_PALETTE_WIDTH: f32 = 560.0;
/// From the window's top to the palette box's top.
pub const COMMAND_PALETTE_TOP: f32 = 110.0;
/// Inner margin of the palette box.
pub const COMMAND_PALETTE_PAD: f32 = 10.0;
/// Result rows the palette shows before it scrolls.
pub const COMMAND_PALETTE_MAX_ROWS: usize = 12;
/// Commands the palette remembers for the empty query.
pub const COMMAND_PALETTE_RECENT: usize = 8;

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
/// Distance between the pointer and a pointer-anchored tooltip, in points, at every Size.
pub const TOOLTIP_POINTER_GAP: f32 = 12.0;

/// Outer width of a toast card, in points, at every Size.
pub const TOAST_WIDTH: f32 = 360.0;

/// Offset of a toast card's shadow, in points (PhotoCraft `notices.rs:238`: offset (0, 2)).
pub const TOAST_SHADOW_OFFSET: [f32; 2] = [0.0, 2.0];
/// Blur of a toast card's shadow, in points (`notices.rs:238`).
pub const TOAST_SHADOW_BLUR: f32 = 8.0;

/// Widest the column of a centred empty state grows, in points, at every Size.
pub const EMPTY_STATE_MAX_WIDTH: f32 = 460.0;

/// Least width of each action of a centred empty state, in points, at every Size. PhotoCraft's
/// Home screen buttons (`start_screen.rs` `HOME_BUTTON_WIDTH`).
pub const EMPTY_STATE_ACTION_MIN_WIDTH: f32 = 190.0;

/// The brand title's size in points, at every Size: PhotoCraft's Home screen name
/// (`start_screen.rs`, `semibold(38.0)`).
pub const EMPTY_STATE_BRAND_TITLE_SIZE: f32 = 38.0;

/// The brand title's line height in points.
pub const EMPTY_STATE_BRAND_TITLE_LINE_HEIGHT: f32 = 46.0;

/// The brand byline's size in points: PhotoCraft's "open source" (`start_screen.rs`,
/// `proportional(13.0)`).
pub const EMPTY_STATE_BRAND_BYLINE_SIZE: f32 = 13.0;

/// The brand byline's line height in points.
pub const EMPTY_STATE_BRAND_BYLINE_LINE_HEIGHT: f32 = 18.0;

/// Between the brand title and its byline, in points (`start_screen.rs`, the `+ 10.0`).
pub const EMPTY_STATE_BRAND_GAP: f32 = 10.0;

/// How far the byline's bottom edge sits above the brand title's, in points (`start_screen.rs`,
/// the `- 8.0`).
pub const EMPTY_STATE_BRAND_BYLINE_LIFT: f32 = 8.0;

/// The control height the Compact slider row sizes are set for; other sizes scale them by their
/// control height over this.
pub const SLIDER_ROW_BASE_CONTROL: f32 = 24.0;
/// Compact width of a slider row's number field (PhotoCraft `slider_row_prec`).
pub const SLIDER_ROW_FIELD_WIDTH: f32 = 74.0;
/// Compact space after a slider row.
pub const SLIDER_ROW_GAP: f32 = 4.0;

/// Glyph of a tool-rail button as a share of the button (PhotoCraft's Pro toolbar: two thirds,
/// 20 of 30 points), rounded to whole points.
pub const RAIL_TOOL_ICON_RATIO: f32 = 2.0 / 3.0;
/// Glyph of a panel-rail button as a share of the button (PhotoCraft's rail: 0.52, 15 of 28
/// points), rounded to whole points.
pub const RAIL_PANEL_ICON_RATIO: f32 = 0.52;
/// Height of the column toggle at the top of a tool rail.
pub const RAIL_TOGGLE_HEIGHT: f32 = 14.0;
/// Glyph of the column toggle.
pub const RAIL_TOGGLE_ICON: f32 = 11.0;
/// Row gap of a tool rail over its column gap, as a share of the inline gap (PhotoCraft's
/// `item_spacing` is 2 across and 3 down; Compact inline gap 4), rounded to whole points.
pub const RAIL_ROW_GAP_RATIO: f32 = 0.75;
/// Side of the flyout indicator triangle's legs.
pub const RAIL_INDICATOR_SIZE: f32 = 4.0;
/// Distance of the flyout indicator from the corner of its button.
pub const RAIL_INDICATOR_INSET: f32 = 2.0;
/// Radius of the open-panel marker dot.
pub const RAIL_MARKER_RADIUS: f32 = 3.0;
/// Width of the outline of a selected rail button.
pub const RAIL_SELECTED_BORDER_WIDTH: f32 = 1.0;
/// The control height the Compact dialog gaps are set for; the vertical gaps of other sizes scale
/// by their control height over this.
pub const DIALOG_BASE_CONTROL: f32 = 24.0;
/// Frame inner margin of a dialog: PhotoCraft's `Frame::popup` (`menu_margin` 6 plus the 1 pt stroke).
pub const DIALOG_PAD: f32 = 7.0;
/// Compact gap between the title and the rule, and the rule and the body's start: PhotoCraft's
/// `add_space(4)` above the rule.
pub const DIALOG_TITLE_GAP: f32 = 4.0;
/// Compact gap between the rule and the body (PhotoCraft `add_space(8)`).
pub const DIALOG_BODY_GAP: f32 = 8.0;
/// Compact gap between the body and the footer (PhotoCraft `add_space(8)`).
pub const DIALOG_FOOTER_GAP: f32 = 8.0;
/// Gap between footer buttons (PhotoCraft `button_row`).
pub const DIALOG_BUTTON_GAP: f32 = 10.0;
/// Least width of a footer button (PhotoCraft).
pub const DIALOG_BUTTON_MIN: f32 = 84.0;
/// Gap between the buttons of a side column (PhotoCraft's Color Picker `add_space(4)`).
pub const DIALOG_SIDE_GAP: f32 = 4.0;
/// Least width of a side-column button (PhotoCraft).
pub const DIALOG_SIDE_MIN: f32 = 96.0;
/// How far the title band reaches past the title on the left, right and top, then below
/// (PhotoCraft `dialogs.rs:670`).
pub const DIALOG_DRAG_PAD: [f32; 3] = [7.0, 7.0, 4.0];
/// Greatest content width of a Compact dialog (PhotoCraft `__compact`).
pub const DIALOG_WIDTH_COMPACT: f32 = 260.0;
/// Least content width of a Standard or Wide dialog.
pub const DIALOG_WIDTH_MIN: f32 = 380.0;
/// Greatest content width of a Standard dialog.
pub const DIALOG_WIDTH_STANDARD: f32 = 440.0;
/// Greatest content width of a Wide dialog.
pub const DIALOG_WIDTH_WIDE: f32 = 600.0;
/// Content width of an alert or a prompt.
pub const DIALOG_WIDTH_ALERT: f32 = 420.0;
/// Default size of a secondary viewport's window, in points.
pub const DIALOG_VIEWPORT_SIZE: [f32; 2] = [480.0, 360.0];
/// A dialog's preview pane: least width (it takes the dialog's width) and height, in points
/// (PhotoCraft's `filter_preview_ui` `PREVIEW_H` and the 140 least width).
pub const DIALOG_PREVIEW_PANE: [f32; 2] = [140.0, 150.0];
/// Least size of a secondary viewport's window, in points.
pub const DIALOG_VIEWPORT_MIN: [f32; 2] = [240.0, 160.0];
/// Space kept between a dialog and the window edge on each side (PhotoCraft: width minus 48).
pub const DIALOG_WINDOW_MARGIN: f32 = 24.0;
/// The same for a resizable dialog's greatest size (PhotoCraft: minus 40 in total).
pub const DIALOG_RESIZE_MARGIN: f32 = 20.0;
/// Width of a dragged dialog that stays on screen (DesignCraft `KEEP_VISIBLE`).
pub const DIALOG_KEEP_VISIBLE: f32 = 100.0;
/// Least height of a scrolling dialog body (PhotoCraft `max(120)`).
pub const DIALOG_SCROLL_MIN: f32 = 120.0;
/// Side of one cell of Canvas Size's anchor grid (PhotoCraft `anchor_grid`, `cell = 24.0`).
pub const DIALOG_ANCHOR_CELL: f32 = 24.0;
/// Inset of an anchor cell's rectangle (PhotoCraft `anchor_grid`, `shrink(1.0)`).
pub const DIALOG_ANCHOR_INSET: f32 = 1.0;
/// Corner radius of the chosen anchor cell's mark (PhotoCraft `anchor_grid`, radius 1.0).
pub const DIALOG_ANCHOR_MARK_RADIUS: f32 = 1.0;
/// Side of a popover's resize grip, in points (PhotoCraft's `GRIP`, `brush_picker.rs`).
pub const POPOVER_GRIP_SIZE: f32 = 16.0;

/// Distance between the grip's diagonal ticks, in points.
pub const POPOVER_GRIP_PITCH: f32 = 4.0;

/// Room a popover keeps between its edge and the window's, in points, when it grows or is clamped.
pub const POPOVER_SCREEN_MARGIN: f32 = 40.0;

/// Alpha (of 255) of the crop frame's composition guides (PhotoCraft's).
pub const CROP_GUIDE_ALPHA: u8 = 90;
/// Reach in points of the crop frame's edges and corners; inside the frame it shrinks to a quarter
/// of the side (PhotoCraft's `HANDLE_PX`).
pub const CROP_HIT: f32 = 8.0;
/// Width of the crop frame line.
pub const CROP_LINE: f32 = 1.0;
/// Width of a composition guide line.
pub const CROP_GUIDE_LINE: f32 = 1.0;
/// Thickness of the corner brackets and edge bars.
pub const CROP_BRACKET: f32 = 3.0;
/// Arm length of a corner bracket; at most a third of the shorter side.
pub const CROP_BRACKET_ARM: f32 = 14.0;
/// An edge bar's half-length is `min(arm, side / bar_share) / 2`.
pub const CROP_BAR_SHARE: f32 = 4.0;
/// Default opacity of the crop shield.
pub const CROP_SHIELD_OPACITY: f32 = 0.75;
/// Share of the shield opacity kept during a gesture with Auto Adjust Opacity.
pub const CROP_SHIELD_EDITING: f32 = 0.5;
/// Target cell side of the Grid composition guide, in screen points.
pub const CROP_GRID_STEP: f32 = 32.0;
/// Most Grid guide lines per axis.
pub const CROP_GRID_MAX: f32 = 96.0;
/// Shortest Straighten line, in screen points.
pub const CROP_STRAIGHTEN_MIN: f32 = 3.0;
/// Width of the Straighten line.
pub const CROP_STRAIGHTEN_LINE: f32 = 1.0;
/// Width of the halo under the Straighten line.
pub const CROP_STRAIGHTEN_HALO: f32 = 3.0;
/// Side of a perspective crop's corner square.
pub const CROP_PERSPECTIVE_CORNER: f32 = 8.0;
/// Edge width of a perspective crop's corner square and placed corner dot.
pub const CROP_PERSPECTIVE_CORNER_EDGE: f32 = 1.25;
/// Width of a perspective crop's outline.
pub const CROP_PERSPECTIVE_OUTLINE: f32 = 1.5;
/// Width of a perspective crop's grid line.
pub const CROP_PERSPECTIVE_GRID_WIDTH: f32 = 0.75;
/// Screen points of the longest side per perspective grid step.
pub const CROP_PERSPECTIVE_GRID_STEP: f32 = 48.0;
/// Radius of a placed perspective corner click.
pub const CROP_PERSPECTIVE_CLICK_DOT: f32 = 4.0;
/// Movement under which a perspective press is a click, at the default reach.
pub const CROP_PERSPECTIVE_CLICK_MOVE: f32 = 3.0;
/// Default reach of a perspective corner.
pub const CROP_PERSPECTIVE_REACH: f32 = 8.0;
/// Width of the options bar's overlay menu and gear popover: `crop.menu_width`.
pub const CROP_MENU_WIDTH: f32 = 240.0;
/// Widest the gear popover grows: `crop.menu_max_width`.
pub const CROP_MENU_MAX_WIDTH: f32 = 300.0;
/// Width of the options bar's ratio preset select: `crop.preset_width` (PhotoCraft's 140).
pub const CROP_PRESET_WIDTH: f32 = 140.0;

/// Canvas grid line width.
pub const GRID_WIDTH: f32 = 1.0;
/// Share of the grid colour's alpha that minor lines keep (PhotoCraft's).
pub const GRID_MINOR_ALPHA: f32 = 0.45;
/// Smallest step in screen points at which a grid axis draws.
pub const GRID_MIN_STEP: f32 = 6.0;
/// Side of a Crossings dot.
pub const GRID_CROSSING: f32 = 1.5;
/// Smallest major spacing in screen points at which Crossings draw.
pub const GRID_CROSSING_MIN: f32 = 8.0;
/// Width of the app shell's edge reveal zone along the window edges, at every Size.
pub const APP_SHELL_EDGE_REVEAL: f32 = 4.0;
/// Behind the document in body-only mode, in every theme: black, as Photoshop's Full Screen Mode.
pub const APP_SHELL_BODY_ONLY_BACKDROP: Rgba = Rgba::BLACK;

/// Outer radius of the Eyedropper's sampling ring, in points (Photoshop's proportions in PhotoCraft:
/// about 210 across).
pub const EYEDROPPER_RING_RADIUS: f32 = 105.0;
/// Width of each of the ring's two bands: the coloured inner one and the grey outer one.
pub const EYEDROPPER_RING_BAND: f32 = 17.0;
/// The ring's opaque outer band: neutral grey 127, as Photoshop measures it.
pub const EYEDROPPER_RING_GREY: Rgba = Rgba([127, 127, 127, 255]);

/// Radius of a Color Sampler point's reticle circle, in points (PhotoCraft's).
pub const SAMPLER_MARKER_RADIUS: f32 = 5.0;
/// Distance from the point at which each reticle tick starts.
pub const SAMPLER_TICK_START: f32 = 6.0;
/// Distance from the point at which each reticle tick ends.
pub const SAMPLER_TICK_END: f32 = 8.0;

/// Screen colour picker: screen pixels across and down the loupe (PhotoCraft's).
pub const SCREEN_PICKER_LOUPE_CELLS: u32 = 11;
/// The loupe's starting zoom, in physical pixels per screen pixel.
pub const SCREEN_PICKER_ZOOM: u32 = 12;
/// The loupe's least zoom.
pub const SCREEN_PICKER_ZOOM_MIN: u32 = 4;
/// The loupe's greatest zoom.
pub const SCREEN_PICKER_ZOOM_MAX: u32 = 24;
/// What one wheel step changes the zoom by.
pub const SCREEN_PICKER_ZOOM_STEP: u32 = 2;

/// Compact width of a paint chip (PhotoCraft's `shape_paint_ui.rs`).
pub const PAINT_CHIP_WIDTH: f32 = 30.0;
/// Compact height of a paint chip.
pub const PAINT_CHIP_HEIGHT: f32 = 22.0;
/// Inset of the No Color diagonal's ends from the chip's edge (PhotoCraft's `shrink(3.0)`).
pub const PAINT_CHIP_NONE_INSET: f32 = 3.0;
/// Width of the No Color diagonal (PhotoCraft's 1.5 stroke).
pub const PAINT_CHIP_NONE_LINE: f32 = 1.5;

/// Compact side of the Color Guide's base colour button (VectorCraft's).
pub const HARMONY_BASE: f32 = 26.0;
/// Compact height of the harmony strip.
pub const HARMONY_STRIP: f32 = 20.0;
/// Compact height of a variation row, cell gap included.
pub const HARMONY_ROW: f32 = 18.0;
/// Compact height of a variation cell.
pub const HARMONY_CELL: f32 = 17.0;

/// Least width of a card in the start screen grid (PhotoCraft's).
pub const START_CARD_MIN: f32 = 260.0;

/// Widest a card in the start screen grid grows (PhotoCraft's).
pub const START_CARD_MAX: f32 = 360.0;

/// Width of a tour card's contents (PhotoCraft's).
pub const TOUR_CARD_WIDTH: f32 = 340.0;

/// Gap between a tour target's highlight and its card (PhotoCraft's).
pub const TOUR_TARGET_GAP: f32 = 12.0;

/// How far a tour highlight stands off its target at rest (PhotoCraft's).
pub const TOUR_HIGHLIGHT_OFFSET: f32 = 3.0;

/// How far a tour highlight pulse adds to its offset (PhotoCraft's).
pub const TOUR_HIGHLIGHT_PULSE: f32 = 2.0;

/// Height of the selection bar (PhotoCraft's).
pub const SELECTION_BAR_HEIGHT: f32 = 36.0;

/// Gap between a selection and the bar below it (PhotoCraft's).
pub const SELECTION_BAR_GAP_BELOW: f32 = 14.0;

/// Gap between a selection and the bar above it (PhotoCraft's).
pub const SELECTION_BAR_GAP_ABOVE: f32 = 8.0;

/// Selection bar inset from the canvas edge (PhotoCraft's).
pub const SELECTION_BAR_INSET: f32 = 8.0;

/// Selection bar width before it is measured (PhotoCraft's).
pub const SELECTION_BAR_FIRST_WIDTH: f32 = 420.0;

/// Width of the selection bar grip (PhotoCraft's).
pub const SELECTION_BAR_GRIP_WIDTH: f32 = 12.0;

/// Height of the selection bar grip (PhotoCraft's).
pub const SELECTION_BAR_GRIP_HEIGHT: f32 = 20.0;

/// Radius of a selection bar grip dot (PhotoCraft's).
pub const SELECTION_BAR_GRIP_DOT: f32 = 1.0;

/// Distance between selection bar grip dots (PhotoCraft's).
pub const SELECTION_BAR_GRIP_PITCH: f32 = 4.0;

/// Content width of the dialog (PhotoCraft).
pub const NEW_DOCUMENT_WIDTH: f32 = 800.0;

/// Size of a preset card (PhotoCraft).
pub const NEW_DOCUMENT_CARD: [f32; 2] = [164.0, 112.0];

/// Columns of cards the grid shows before it scrolls (PhotoCraft).
pub const NEW_DOCUMENT_CARD_COLUMNS: usize = 3;

/// Rows of cards the grid shows before it scrolls (PhotoCraft).
pub const NEW_DOCUMENT_CARD_ROWS: usize = 3;

/// Longer side of the page icon on a card (PhotoCraft).
pub const NEW_DOCUMENT_PAGE_ICON: f32 = 30.0;

/// Side of the square the page icon sits in (PhotoCraft).
pub const NEW_DOCUMENT_PAGE_BOX: f32 = 40.0;

/// From a card's top to the centre of its page icon (PhotoCraft).
pub const NEW_DOCUMENT_CARD_ICON_CENTER: f32 = 34.0;

/// From a card's top to the centre of its name (PhotoCraft).
pub const NEW_DOCUMENT_CARD_TITLE_CENTER: f32 = 70.0;

/// From a card's top to the centre of its size line (PhotoCraft).
pub const NEW_DOCUMENT_CARD_LINE_CENTER: f32 = 97.0;

/// Room beside the card grid for its scroll bar (PhotoCraft).
pub const NEW_DOCUMENT_SCROLL_ROOM: f32 = 16.0;

/// Height of the Saved list (PhotoCraft).
pub const NEW_DOCUMENT_SAVED_HEIGHT: f32 = 320.0;

/// Height of a row of the Saved list (PhotoCraft).
pub const NEW_DOCUMENT_SAVED_ROW: f32 = 32.0;

/// Content width of Save for Web (PhotoCraft).
pub const EXPORT_WEB_WIDTH: f32 = 900.0;

/// Width of Save for Web's Matte field (PhotoCraft).
pub const EXPORT_WEB_FIELD_WIDTH: f32 = 80.0;

/// Width of the settings column of Export As and Save for Web (PhotoCraft: 248 and 250).
pub const EXPORT_SETTINGS_WIDTH: f32 = 250.0;

/// Between Export As's settings and its preview (PhotoCraft).
pub const EXPORT_PANE_GAP: f32 = 12.0;

/// Width of Save PDF's section list (VectorCraft).
pub const EXPORT_PDF_LIST_WIDTH: f32 = 150.0;

/// Width of Save PDF's section content (VectorCraft).
pub const EXPORT_PDF_CONTENT_WIDTH: f32 = 500.0;

/// Height of Save PDF's section content before it scrolls (VectorCraft).
pub const EXPORT_PDF_CONTENT_HEIGHT: f32 = 360.0;

/// Between Save PDF's section list and its content (VectorCraft).
pub const EXPORT_PDF_GAP: f32 = 14.0;

/// Width of the Save PDF dialog: list, gap and content.
pub const EXPORT_PDF_WIDTH: f32 = EXPORT_PDF_LIST_WIDTH + EXPORT_PDF_GAP + EXPORT_PDF_CONTENT_WIDTH;

/// Size of a chip (PhotoCraft).
pub const THEME_CHIP_SIZE: [f32; 2] = [30.0, 26.0];

/// From the outline to the fill (PhotoCraft).
pub const THEME_CHIP_INSET: f32 = 3.0;

/// Height of the chrome strip along the top of the fill (PhotoCraft).
pub const THEME_CHIP_STRIP: f32 = 5.0;

/// Radius of the accent dot (PhotoCraft).
pub const THEME_CHIP_DOT_RADIUS: f32 = 2.5;

/// From the fill's bottom end corner to the dot's centre, on both axes (PhotoCraft).
pub const THEME_CHIP_DOT_OFFSET: f32 = 6.0;

/// Corner radius of the outline (PhotoCraft).
pub const THEME_CHIP_OUTLINE_RADIUS: f32 = 3.0;

/// Corner radius of the fill (PhotoCraft).
pub const THEME_CHIP_FILL_RADIUS: f32 = 2.0;

/// Radius of the favourite star's badge (PhotoCraft).
pub const THEME_CHIP_BADGE_RADIUS: f32 = 7.0;

/// How far the badge's centre sits inside the chip's top end corner, as `[x, y]` (PhotoCraft).
pub const THEME_CHIP_BADGE_INSET: [f32; 2] = [3.0, 3.0];

/// Between two chips, with room for the badge that overhangs a corner (PhotoCraft).
pub const THEME_CHIP_GAP: f32 = 6.0;

/// Radius of the summary's status dot (PhotoCraft).
pub const FINDINGS_STATUS_DOT: f32 = 4.5;

/// Size of the list box (PhotoCraft).
pub const FILE_LIST_SIZE: [f32; 2] = [300.0, 160.0];

/// Least width of a button beside the list (PhotoCraft).
pub const FILE_LIST_BUTTON_MIN: f32 = 120.0;

// Step 5 controls (S5C-F8).

/// Side of a table header's sort chevron (PdfCraft's Combine Files table).
pub const TABLE_SORT_GLYPH: f32 = 12.0;

/// From a header's text to its sort chevron (PdfCraft: the chevron's centre 14 past the label with a 12 chevron).
pub const TABLE_SORT_GAP: f32 = 8.0;

/// Width of a column edge's grab strip: three points either side of the edge.
pub const TABLE_GRIP_WIDTH: f32 = 6.0;

/// Width of the drop line of a moved column (PdfCraft).
pub const TABLE_DROP_LINE_WIDTH: f32 = 2.5;

/// Least width of a table column that sets none of its own (room for a short word).
pub const TABLE_MIN_COLUMN: f32 = 40.0;

/// How far past a header cell the drop line of a moved column may reach: it straddles the cell's edge, so its half width and a little more.
pub const TABLE_DROP_LINE_SLACK: f32 = 3.0;

/// Height of a chip, small and medium (LightCraft's keyword chip 22 and filter chip 18).
pub const CHIP_HEIGHT: (f32, f32) = (18.0, 22.0);

/// Between a chip's edge and its label (LightCraft).
pub const CHIP_PAD_X: f32 = 8.0;

/// Width of a chip's remove target (LightCraft's `CROSS_W`).
pub const CHIP_REMOVE_WIDTH: f32 = 22.0;

/// Between chips and between lines of chips (LightCraft).
pub const CHIP_GAP: f32 = 6.0;

/// Reach of a chip's remove cross from its centre (LightCraft).
pub const CHIP_CROSS_ARM: f32 = 3.5;

/// Stroke width of a chip's remove cross (LightCraft).
pub const CHIP_CROSS_STROKE: f32 = 1.5;

/// Side of an icon picker's cell (DeckCraft's `icon_cell`).
pub const ICON_PICKER_CELL: f32 = 36.0;

/// Between an icon picker's cells (PhotoCraft's preset panels).
pub const ICON_PICKER_GAP: f32 = 3.0;

/// Between an icon picker's cell edge and its icon (DeckCraft).
pub const ICON_PICKER_INSET: f32 = 6.0;

/// Most an icon picker's grid takes before it scrolls (DeckCraft's `max_height(420.0)`).
pub const ICON_PICKER_MAX_HEIGHT: f32 = 420.0;

/// Width of a split button's arrow part (VectorCraft).
pub const SPLIT_BUTTON_ARROW_WIDTH: f32 = 18.0;

/// Between a split button's divider and its top and bottom edge (VectorCraft).
pub const SPLIT_BUTTON_DIVIDER_INSET: f32 = 5.0;

/// Side of the chevron between breadcrumb segments (LightCraft).
pub const BREADCRUMB_SEPARATOR: f32 = 16.0;

/// Space beside a breadcrumb segment's text and either side of a chevron (LightCraft).
pub const BREADCRUMB_GAP: f32 = 4.0;

/// The room one breadcrumb segment counts for when deciding how many fit (LightCraft).
pub const BREADCRUMB_MIN_SEGMENT: f32 = 80.0;

/// Least side of a date picker's cell (LightCraft).
pub const DATE_CELL: f32 = 26.0;

/// Between a date picker's cells (LightCraft).
pub const DATE_GAP: f32 = 2.0;

/// Width of a date picker's popup (LightCraft).
pub const DATE_POPUP_WIDTH: f32 = 220.0;

/// Padding either side of a month or year cell's text (LightCraft).
pub const DATE_CELL_PAD: f32 = 6.0;

/// Radius of a reset dot's circle (VectorCraft).
pub const RESET_DOT_RADIUS: f32 = 3.0;

/// Corner radius of a reset dot's state fills (VectorCraft).
pub const RESET_DOT_CORNER: f32 = 3.0;

/// Distance of a drag badge from the pointer, across (LightCraft's `pos + vec2(14, 10)`).
pub const DRAG_BADGE_OFFSET_X: f32 = 14.0;

/// Distance of a drag badge from the pointer, down (LightCraft).
pub const DRAG_BADGE_OFFSET_Y: f32 = 10.0;

/// Padding beside a drag badge's text (LightCraft).
pub const DRAG_BADGE_PAD_X: f32 = 9.0;

/// Padding above and below a drag badge's text (LightCraft).
pub const DRAG_BADGE_PAD_Y: f32 = 3.0;

/// Distance of a drop veil's frame from the window's edge (LightCraft).
pub const DROP_VEIL_INSET: f32 = 10.0;

/// Stroke width of a drop veil's frame (LightCraft).
pub const DROP_VEIL_FRAME_WIDTH: f32 = 2.0;

/// Corner radius of a drop veil's frame (LightCraft).
pub const DROP_VEIL_FRAME_RADIUS: f32 = 12.0;

/// Corner radius of a drop veil's card (LightCraft).
pub const DROP_VEIL_CARD_RADIUS: f32 = 10.0;

/// Between a drop veil card's lines (LightCraft).
pub const DROP_VEIL_CARD_GAP: f32 = 8.0;

/// Padding at the sides of a drop veil's card (LightCraft).
pub const DROP_VEIL_CARD_PAD_X: f32 = 32.0;

/// Padding at the top and bottom of a drop veil's card (LightCraft).
pub const DROP_VEIL_CARD_PAD_Y: f32 = 24.0;

/// A drop veil's lines wrap at the window's width less this (LightCraft).
pub const DROP_VEIL_WRAP_MARGIN: f32 = 120.0;

/// The least width a drop veil's lines wrap at (LightCraft).
pub const DROP_VEIL_MIN_WRAP: f32 = 200.0;

/// Least height of an event log's details box (FilmCraft).
pub const EVENT_LOG_DETAILS_HEIGHT: f32 = 70.0;

/// Vertical inset of an event log's details box (FilmCraft).
pub const EVENT_LOG_DETAILS_PAD_Y: f32 = 4.0;

/// Least width of an autocomplete list (GridCraft's formula bar).
pub const AUTOCOMPLETE_MIN_WIDTH: f32 = 220.0;

/// Between an editor's bottom edge and its autocomplete list (GridCraft).
pub const AUTOCOMPLETE_GAP: f32 = 4.0;

/// Between a metadata table's name and value columns (LightCraft's grid spacing).
pub const METADATA_COLUMN_GAP: f32 = 16.0;

/// Between a metadata table's rows (LightCraft's grid spacing).
pub const METADATA_ROW_GAP: f32 = 3.0;

/// Above a metadata group's title, except the first (LightCraft).
pub const METADATA_GROUP_GAP: f32 = 6.0;

/// The most a metadata table's rows take before they scroll (LightCraft's dialog).
pub const METADATA_MAX_HEIGHT: f32 = 440.0;

/// Side of a rating row's star, small and medium (LightCraft's `widgets/rating.rs`).
pub const RATING_STAR: (f32, f32) = (12.0, 14.0);

/// Side of a rating row's star cell, small and medium (LightCraft).
pub const RATING_CELL: (f32, f32) = (15.0, 19.0);

/// Side of a painted badge star, small and medium (LightCraft's film-strip and grid badges).
pub const RATING_BADGE_STAR: (f32, f32) = (8.0, 12.0);

/// From one painted badge star to the next, small and medium (LightCraft).
pub const RATING_BADGE_STEP: (f32, f32) = (9.0, 13.0);

/// Duration of a text caret's glide, in milliseconds (WordCraft's `caret_glide.rs`, measured from Word).
pub const MOTION_CARET_MS: u32 = 100;

/// Easing of a text caret's glide: Word's `cubic-bezier(0, 0, 0, 1)` (WordCraft's `caret_glide.rs`).
pub const MOTION_CARET_EASING: Easing = Easing::Enter;

/// Duration of a selection outline's glide, in milliseconds (GridCraft's `selection_glide.rs`).
pub const MOTION_OUTLINE_MS: u32 = 120;

/// Easing of a selection outline's glide. GridCraft's `ease_out` is `1 - (1 - t)^3` (0.875 at the
/// half way); `Standard` reaches 0.878 there and is the closest of the three curves.
pub const MOTION_OUTLINE_EASING: Easing = Easing::Standard;

/// Width of the About window (PhotoCraft).
pub const ABOUT_WIDTH: f32 = 700.0;

/// Height of the About window's Contributors list (PhotoCraft).
pub const ABOUT_LIST_HEIGHT: f32 = 340.0;

/// Gaps between the names of the About grab bag (PhotoCraft).
pub const ABOUT_BAG_GAP: [f32; 2] = [14.0, 8.0];

/// Width of the About window's community button (the module's constant).
pub const ABOUT_COMMUNITY_WIDTH: f32 = 220.0;

/// Least width of the About tables' flexible column (PhotoCraft).
pub const ABOUT_FLEX_MIN: f32 = 110.0;

/// Width of the password dialog (PdfCraft).
pub const PASSWORD_WIDTH: f32 = 400.0;

/// Size of a list manager's list box (GridCraft).
pub const LIST_MANAGER_LIST: [f32; 2] = [330.0, 200.0];

/// Least width of a list manager's buttons (GridCraft).
pub const LIST_MANAGER_BUTTON_WIDTH: f32 = 100.0;

/// Between a list manager's list and its buttons (the module's constant).
pub const LIST_MANAGER_COLUMN_GAP: f32 = 12.0;

/// Between an align bar's buttons and its ⋯ button at the Compact size (PhotoCraft's `add_space(6.0)`); scaled with the control height.
pub const ALIGN_MENU_GAP: f32 = 6.0;

/// Most height of a tree picker's list (the module's constant).
pub const TREE_PICKER_LIST_HEIGHT: f32 = 320.0;

/// Least width of a tree picker's popup (the module's constant).
pub const TREE_PICKER_MIN_POPUP_WIDTH: f32 = 220.0;

/// Width of an alt text editor's box (the module's constant).
pub const ALT_TEXT_BOX_WIDTH: f32 = 340.0;

/// Least gap between an alt text editor's rows (the module's constant).
pub const ALT_TEXT_ROW_GAP: f32 = 6.0;

/// Side of a checkerboard cell behind a translucent thumbnail (PhotoCraft `layer_row.rs:534`).
pub const THUMBNAIL_CHECKER_CELL: f32 = 5.0;

/// Side of a spot channel's ink swatch on its thumbnail (PhotoCraft `channels_panel.rs:245`).
pub const CHANNELS_SPOT_SWATCH: f32 = 9.0;

/// Side of a History snapshot's thumbnail (PhotoCraft `history.rs:284`).
pub const HISTORY_SNAPSHOT_THUMB: f32 = 28.0;
/// Width of the Undo list popup (WordCraft `chrome.rs:153`).
pub const HISTORY_UNDO_LIST_WIDTH: f32 = 210.0;

/// Box of a Layers indicator icon (PhotoCraft `layer_row_ui.rs`, `ICON_W` with its padding).
pub const LAYERS_INDICATOR_ICON: f32 = 14.0;
/// Width of the Layers effects triangle (PhotoCraft `TRIANGLE_W`).
pub const LAYERS_INDICATOR_TRIANGLE: f32 = 10.0;
/// Box of a Layers mask badge (PhotoCraft `mask_thumbs_ui.rs`).
pub const LAYERS_MASK_BADGE: f32 = 9.0;
/// Least width of the Layers blend select before the "Opacity:" label hides.
pub const LAYERS_BLEND_MIN_WIDTH: f32 = 80.0;
/// Width of the Layers Opacity and Fill fields, without the pop-up arrow.
pub const LAYERS_PERCENT_FIELD_WIDTH: f32 = 66.0;
/// Width of the Layers filter type select (PhotoCraft `layer_filter_ui.rs`).
pub const LAYERS_FILTER_TYPE_WIDTH: f32 = 64.0;
/// Panel Options' sample side as a factor of the row thumbnail.
pub const LAYERS_OPTIONS_SAMPLE_SCALE: f32 = 2.0;
/// Space after the Layers filter type select (PhotoCraft `layer_filter_ui.rs`).
pub const LAYERS_FILTER_TYPE_GAP: f32 = 4.0;
/// Room kept free between the "Opacity:" label and its field (PhotoCraft `opacity_row_layout`).
pub const LAYERS_BLEND_LABEL_SLACK: f32 = 16.0;
/// Least width of the blend select when the row is too narrow for anything else (PhotoCraft
/// `opacity_row_layout`).
pub const LAYERS_BLEND_FLOOR_WIDTH: f32 = 40.0;
/// Space between the blend, Opacity, Lock and Fill rows (PhotoCraft `ROW_GAP`).
pub const LAYERS_BLEND_ROW_GAP: f32 = 4.0;
/// Ink of the Type layer's kind tile, the same in every theme like its paper (PhotoCraft
/// `layer_row.rs`).
pub const LAYERS_KIND_TYPE_INK: Rgba = Rgba::rgb(40, 40, 40);
/// Space between the effects triangle and the `fx` mark (PhotoCraft `layer_row_ui.rs`).
pub const LAYERS_INDICATOR_FX_GAP: f32 = 1.0;
/// Stroke of the cross over a disabled mask.
pub const LAYERS_MASK_CROSS_WIDTH: f32 = 1.5;
/// How far the cross over a disabled mask is inset from its thumbnail.
pub const LAYERS_MASK_CROSS_INSET: f32 = 1.0;
/// Stroke of the Smart Filters indicator's rings.
pub const LAYERS_INDICATOR_RING_WIDTH: f32 = 1.0;

/// The timecode and toolbar strip over the track headers.
pub const TIMELINE_TOP_HEIGHT: f32 = 58.0;
/// The ruler's height.
pub const TIMELINE_RULER_HEIGHT: f32 = 44.0;
/// The scroll bar's height.
pub const TIMELINE_SCROLLBAR_HEIGHT: f32 = 17.0;
/// The least thumb length of the scroll bar.
pub const TIMELINE_SCROLL_THUMB_MIN: f32 = 34.0;
/// The divider between video and audio tracks.
pub const TIMELINE_DIVIDER_HEIGHT: f32 = 5.0;
/// The Mix row at the bottom.
pub const TIMELINE_MIX_ROW_HEIGHT: f32 = 34.0;
/// The track headers' default width.
pub const TIMELINE_HEADER_WIDTH: f32 = 204.0;
/// A new video track's height.
pub const TIMELINE_VIDEO_TRACK_HEIGHT: f32 = 60.0;
/// A new audio track's height.
pub const TIMELINE_AUDIO_TRACK_HEIGHT: f32 = 56.0;
/// The least a track's height can be dragged to.
pub const TIMELINE_TRACK_HEIGHT_MIN: f32 = 8.0;
/// The most a track's height can be dragged to.
pub const TIMELINE_TRACK_HEIGHT_MAX: f32 = 600.0;
/// The colour label strip at a header's leading edge.
pub const TIMELINE_LABEL_STRIP: f32 = 5.0;
/// How near a snap target must be.
pub const TIMELINE_SNAP_DISTANCE: f32 = 9.0;
/// The widest a clip's edge zone gets on each side of an edge.
pub const TIMELINE_EDGE_ZONE: f32 = 7.0;
/// The distance from a cut within which the Selection tool rolls.
pub const TIMELINE_ROLL_ZONE: f32 = 2.5;
/// The room `TimeView::fit` leaves.
pub const TIMELINE_FIT_MARGIN: f32 = 10.0;
/// A transport button's square.
pub const TIMELINE_TRANSPORT_BUTTON: f32 = 28.0;
/// A keyframe glyph's box.
pub const TIMELINE_KEYFRAME_GLYPH: f32 = 15.0;
/// A hold key's square inside the glyph box.
pub const TIMELINE_KEYFRAME_HOLD: f32 = 9.0;
/// A bezier key's circle radius.
pub const TIMELINE_KEYFRAME_BEZIER: f32 = 5.5;
/// How near the playhead or another key a Shift-dragged key snaps.
pub const TIMELINE_KEYFRAME_SNAP: f32 = 8.0;
/// The keyframe navigator's arrow box width.
pub const TIMELINE_KEYFRAME_ARROW_WIDTH: f32 = 14.0;
/// The keyframe navigator's arrow box height.
pub const TIMELINE_KEYFRAME_ARROW_HEIGHT: f32 = 20.0;
/// The room between the keyframe navigator's parts.
pub const TIMELINE_KEYFRAME_NAV_STEP: f32 = 16.0;
/// A marker flag's width.
pub const TIMELINE_MARKER_WIDTH: f32 = 8.0;
/// How near a work-area end must be to grab it.
pub const TIMELINE_WORK_AREA_GRAB: f32 = 8.0;
/// The least room between two ruler labels.
pub const TIMELINE_RULER_LABEL_GAP: f32 = 110.0;
/// The least room between two ruler ticks.
pub const TIMELINE_RULER_TICK_GAP: f32 = 12.0;
/// The drag length that moves a timecode field's value one frame.
pub const TIMELINE_SCRUB_STEP: f32 = 1.0;
/// A marker flag's height at most (EffectCraft `markers_ui.rs:287`).
pub const TIMELINE_MARKER_HEIGHT: f32 = 10.0;
/// A marker's hit width (`markers_ui.rs:310`).
pub const TIMELINE_MARKER_HIT: f32 = 10.0;
/// The room either side of a marker's name inside its box (`markers_ui.rs:304`).
pub const TIMELINE_MARKER_NAME_PAD: f32 = 4.0;
/// The room between a marker's flag and its name box (`markers_ui.rs:381`, `:389`).
pub const TIMELINE_MARKER_NAME_GAP: f32 = 6.0;
/// The room between a range marker's end and a name box after it (`markers_ui.rs:383`).
pub const TIMELINE_MARKER_END_GAP: f32 = 3.0;
/// A graph key's square (EffectCraft `graph.rs:543`).
pub const TIMELINE_GRAPH_KEY: f32 = 7.0;
/// A graph key's hit square (`graph.rs:545`).
pub const TIMELINE_GRAPH_KEY_HIT: f32 = 11.0;
/// A graph handle's dot radius (`graph.rs:488`).
pub const TIMELINE_GRAPH_HANDLE: f32 = 3.5;
/// A graph handle's hit square (`graph.rs:490`).
pub const TIMELINE_GRAPH_HANDLE_HIT: f32 = 10.0;
/// The room over the graph's plot (`graph.rs:412`).
pub const TIMELINE_GRAPH_TOP_INSET: f32 = 6.0;
/// The room between the plot's edge and its value labels (`graph.rs:441`).
pub const TIMELINE_GRAPH_LABEL_INSET: f32 = 4.0;
/// How far past the plot a key still counts as shown (`graph.rs:479`).
pub const TIMELINE_GRAPH_REACH: f32 = 6.0;
/// The graph's bottom bar (`graph.rs:411`).
pub const TIMELINE_GRAPH_BAR_HEIGHT: f32 = 28.0;
/// The room either side of the overview's runs (SoundCraft `universe.rs:61`).
pub const TIMELINE_OVERVIEW_INSET_X: f32 = 8.0;
/// The room over and under the overview's runs (`universe.rs:61`).
pub const TIMELINE_OVERVIEW_INSET_Y: f32 = 4.0;
/// How far the overview's view box reaches past the runs (`universe.rs:81`).
pub const TIMELINE_OVERVIEW_BOX_OUTSET: f32 = 2.0;
/// The least width the overview takes the time area to have (`universe.rs:80`).
pub const TIMELINE_OVERVIEW_VIEW_MIN: f32 = 100.0;
/// The ruler's marker band: the markers sit on its first row (FilmCraft `timeline_hit.rs:107`,
/// the work-area bar starts under it; `ruler.rs` draws the pentagons at 2 pt, 12 pt tall).
pub const TIMELINE_RULER_MARKER_BAND: f32 = 14.0;
/// The work-area bar's height (`timeline_hit.rs:108`).
pub const TIMELINE_RULER_WORK_HEIGHT: f32 = 6.0;
/// How far below the ruler's top the time labels' centre line is (FilmCraft `timeline/ruler.rs:82`).
pub const TIMELINE_RULER_LABEL_Y: f32 = 24.0;
/// How far above the ruler's bottom the ticks start (`ruler.rs:83`).
pub const TIMELINE_RULER_TICK_INSET: f32 = 3.0;
/// A labelled tick's length (`ruler.rs:114`).
pub const TIMELINE_RULER_MAJOR_TICK: f32 = 10.0;
/// An unlabelled tick's length (`ruler.rs:114`).
pub const TIMELINE_RULER_MINOR_TICK: f32 = 4.0;
/// The render and cache bar's height at the ruler's bottom (`ruler.rs:125`).
pub const TIMELINE_RULER_RENDER_BAR: f32 = 2.0;
/// Half the playhead head's width (FilmCraft `timeline.rs:414`).
pub const TIMELINE_RULER_HEAD_HALF_WIDTH: f32 = 5.0;
/// The playhead head's height (`timeline.rs:413`).
pub const TIMELINE_RULER_HEAD_HEIGHT: f32 = 13.0;
/// A work-area end cap's width (`ruler.rs:157`).
pub const TIMELINE_RULER_CAP_WIDTH: f32 = 3.0;
/// The radius of the knobs at the scroll thumb's ends (`scrollbars.rs:35`).
pub const TIMELINE_SCROLL_KNOB_RADIUS: f32 = 4.5;
/// How near a thumb's end a press must be to resize the view (`scrollbars.rs:63`, `:65`).
pub const TIMELINE_SCROLL_END_ZONE: f32 = 9.0;
/// The least the track headers' width can be dragged to: FilmCraft's fixed header width
/// (`state.rs:232`), which has no header resize.
pub const TIMELINE_HEADER_WIDTH_MIN: f32 = 204.0;
/// A caption row's height when the app gives none: FilmCraft `timeline_captions.rs:22` (`ROW_H`).
pub const TIMELINE_CAPTION_ROW_HEIGHT: f32 = 28.0;
/// How far each level of a track's depth indents its header: EffectCraft `panels/timeline.rs:2024`.
pub const TIMELINE_HEADER_INDENT: f32 = 14.0;
/// The most the track headers' width can be dragged to. FilmCraft has no header resize, so this
/// value is the lead's.
pub const TIMELINE_HEADER_WIDTH_MAX: f32 = 480.0;
/// A comment pin's size.
pub const REVIEW_PIN: f32 = 28.0;
/// Pins nearer than this gather into a cluster.
pub const REVIEW_CLUSTER_RADIUS: f32 = 24.0;
/// The thread popover's width.
pub const REVIEW_POPOVER_WIDTH: f32 = 300.0;
/// The version history's timeline dot column.
pub const REVIEW_VERSION_DOT_X: f32 = 20.0;
/// The version history's text column.
pub const REVIEW_VERSION_TEXT_X: f32 = 36.0;
/// A cursor chat bubble's width.
pub const REVIEW_CHAT_WIDTH: f32 = 220.0;
/// A cursor chat bubble's corner radius.
pub const REVIEW_CHAT_RADIUS: f32 = 14.0;
/// The join field's width.
pub const REVIEW_JOIN_WIDTH: f32 = 196.0;
/// The revision card's colour bar width.
pub const REVIEW_REVISION_BAR_WIDTH: f32 = 4.0;
/// The revision card's colour bar height.
pub const REVIEW_REVISION_BAR_HEIGHT: f32 = 16.0;
/// The column headers' height.
pub const SHEET_COLUMN_HEADER_HEIGHT: f32 = 21.0;
/// The row headers' width per digit of the last visible row.
pub const SHEET_ROW_HEADER_DIGIT: f32 = 7.0;
/// The room around the row header's digits.
pub const SHEET_ROW_HEADER_PADDING: f32 = 14.0;
/// One outline lane.
pub const SHEET_OUTLINE_LANE: f32 = 15.0;
/// What an outline button's side is short of the lane (GridCraft `outline.rs:197`).
pub const SHEET_OUTLINE_BUTTON_MARGIN: f32 = 4.0;
/// An outline button's least side (GridCraft `outline.rs:197`).
pub const SHEET_OUTLINE_BUTTON_MIN: f32 = 7.0;
/// The first outline lane's inset from the band's edge (GridCraft `outline.rs:203`).
pub const SHEET_OUTLINE_INSET: f32 = 2.0;
/// How far an outline button's hit area reaches past its square (GridCraft `outline.rs:288`).
pub const SHEET_OUTLINE_HIT: f32 = 1.5;
/// An outline button's corner radius (GridCraft `outline.rs:268`).
pub const SHEET_OUTLINE_RADIUS: f32 = 1.0;
/// A slicer's header.
pub const SHEET_SLICER_HEADER: f32 = 26.0;
/// A slicer's value tile.
pub const SHEET_SLICER_TILE: f32 = 22.0;
/// The gap between slicer tiles.
pub const SHEET_SLICER_GAP: f32 = 3.0;
/// The padding round a slicer's tiles.
pub const SHEET_SLICER_PADDING: f32 = 6.0;
/// A slicer's clear button.
pub const SHEET_SLICER_CLEAR: f32 = 16.0;
/// The error checking tag's square.
pub const SHEET_ERROR_TAG: f32 = 18.0;
/// The filter button's least side.
pub const SHEET_FILTER_BUTTON_MIN: f32 = 10.0;
/// The filter button's largest side.
pub const SHEET_FILTER_BUTTON_MAX: f32 = 18.0;
/// The filter button's inset from the header cell's corner.
pub const SHEET_FILTER_BUTTON_INSET: f32 = 2.0;
/// The gap between the error checking tag and its cell.
pub const SHEET_ERROR_TAG_GAP: f32 = 2.0;
/// The error checking tag's corner radius.
pub const SHEET_ERROR_TAG_RADIUS: f32 = 3.0;
/// The warning diamond's half-diagonal in the error checking tag.
pub const SHEET_ERROR_TAG_MARK: f32 = 6.0;
/// The filter button's corner radius.
pub const SHEET_FILTER_BUTTON_RADIUS: f32 = 2.0;
/// The filter button's inset round its glyph.
pub const SHEET_FILTER_BUTTON_GLYPH_INSET: f32 = 1.5;
/// The gap between statistics in the status bar.
pub const SHEET_STATS_GAP: f32 = 10.0;
/// How far from a header's far edge a press still grabs it (GridCraft `grid/interact.rs:949`).
pub const SHEET_EDGE_GRAB: f32 = 4.0;
/// The fill handle's side (GridCraft `grid/selection.rs:133`).
pub const SHEET_FILL_HANDLE: f32 = 6.0;
/// The accent bar on a selected header's inner edge (`grid/show.rs:566`).
pub const SHEET_HEADER_ACCENT_BAR: f32 = 2.0;
/// The distance of each line of the double divider from a hidden line's edge (`grid/show.rs:569-574`,
/// `:610-614`).
pub const SHEET_HIDDEN_MARK_OFFSET: f32 = 1.5;
/// How far the headers' separators stop short of the header's start (`grid/show.rs:568`).
pub const SHEET_HEADER_SEPARATOR_INSET: f32 = 3.0;
/// The status bar menu's least width (`tabs.rs:480`).
pub const SHEET_STATUS_MENU_MIN_WIDTH: f32 = 180.0;
/// The field list's checklist, at most this tall (`pivot_pane.rs:39`).
pub const SHEET_FIELD_LIST_CHECKLIST_MAX: f32 = 220.0;
/// A field list area's least height (`pivot_pane.rs:90`).
pub const SHEET_FIELD_LIST_AREA_MIN: f32 = 60.0;
/// The alpha (0..=255) of the selection's fill in light and in dark themes (GridCraft
/// `theme.rs:203`: 0x1C, 0x24).
pub const SHEET_SELECTION_FILL_ALPHA: (u8, u8) = (0x1C, 0x24);

/// The selected comment card's accent bar width (PdfCraft `comments_panel.rs:464`).
pub const REVIEW_CARD_ACCENT_WIDTH: f32 = 4.0;
/// The room between a pin's bubble and the avatar circle inside it: an 11 pt circle in a 28 pt
/// bubble (FigCraft `canvas/comments.rs:357`).
pub const REVIEW_PIN_INSET: f32 = 3.0;
/// The filled version dot's radius (FigCraft `panels/versions.rs:386`).
pub const REVIEW_VERSION_DOT_RADIUS: f32 = 4.0;
/// The hollow version dot's radius (`panels/versions.rs:388`).
pub const REVIEW_VERSION_DOT_HOLLOW_RADIUS: f32 = 3.5;
/// The dark ink on a light person colour, the same in every theme (FigCraft `live.rs:182`).
pub const REVIEW_INK_DARK: Rgba = Rgba::rgb(0x1E, 0x1E, 0x1E);
/// Between pages, and between rows.
pub const PAGES_GAP: f32 = 14.0;
/// Above the first and below the last row.
pub const PAGES_MARGIN: f32 = 28.0;
/// Kept clear at each side of the pages.
pub const PAGES_SIDE: f32 = 70.0;
/// What Hide White Space leaves above and below each page's text area.
pub const PAGES_WHITE_SPACE_PAD: f32 = 6.0;
/// The seam Hide White Space draws between pages.
pub const PAGES_SEAM: f32 = 1.0;
/// The thumbnail width at zoom 1.
pub const PAGES_THUMBNAIL_USUAL_WIDTH: f32 = 150.0;
/// What the panel keeps free beside the thumbnails.
pub const PAGES_THUMBNAIL_PANEL_INSET: f32 = 40.0;
/// How far either side of an edge grabs it.
pub const PAGES_MARGIN_HANDLE_GRAB: f32 = 4.0;
/// A section's inset from its band, top and bottom.
pub const PAGES_HEADER_FOOTER_INSET: f32 = 3.0;
/// The dashed outline of a hovered empty section.
pub const PAGES_HEADER_FOOTER_HINT_OUTLINE: f32 = 1.0;
/// The edited section's outline and its neighbours'.
pub const PAGES_HEADER_FOOTER_EDIT_OUTLINE: f32 = 1.0;
/// Corner radius of the selection rects.
pub const PAGES_TEXT_SELECTION_RADIUS: f32 = 1.0;
/// The column rectangle's outline while dragging.
pub const PAGES_TEXT_SELECTION_COLUMN_OUTLINE: f32 = 1.0;
/// Alpha of the selection tint.
pub const PAGES_TEXT_SELECTION_FILL_ALPHA: u8 = 70;
/// Corner radius of the current match.
pub const PAGES_FIND_CURRENT_RADIUS: f32 = 2.0;
/// How much the current match grows past the others.
pub const PAGES_FIND_CURRENT_GROW: f32 = 1.0;
/// Alpha of a match's tint.
pub const PAGES_FIND_MATCHED_ALPHA: u8 = 90;
/// Alpha of the current match's tint.
pub const PAGES_FIND_CURRENT_ALPHA: u8 = 110;
/// The bar's outline.
pub const PAGES_COLUMN_BAR_OUTLINE: f32 = 1.0;
/// A divider at rest.
pub const PAGES_COLUMN_BAR_DIVIDER: f32 = 1.5;
/// A divider hovered, dragged or focused.
pub const PAGES_COLUMN_BAR_DIVIDER_ACTIVE: f32 = 3.0;
/// How far either side of a divider grabs it.
pub const PAGES_COLUMN_BAR_GRAB: f32 = 4.0;
/// The narrowest column that paints its percentage (PdfCraft `field_table_ui.rs:363`: wider
/// than 32).
pub const PAGES_COLUMN_BAR_LABEL_MIN: f32 = 32.0;
/// Between columns.
pub const PAGES_READING_COLUMN_GAP: f32 = 18.0;
/// Room under the columns for the page-turn bar.
pub const PAGES_READING_NAV_BAR: f32 = 44.0;
/// One column per this much width.
pub const PAGES_READING_COLUMN_TARGET: f32 = 295.0;
/// The least column width.
pub const PAGES_READING_COLUMN_MIN: f32 = 192.0;
/// A port's side.
pub const PAGES_FRAME_MARKS_PORT: f32 = 8.5;
/// A port's distance from the frame's corner.
pub const PAGES_FRAME_MARKS_PORT_INSET: f32 = 10.0;
/// The overset mark's arm length.
pub const PAGES_FRAME_MARKS_OVERSET_ARM: f32 = 5.0;
/// The overset mark's stroke.
pub const PAGES_FRAME_MARKS_OVERSET_STROKE: f32 = 1.5;
/// The least hidden-character mark size.
pub const PAGES_FRAME_MARKS_MARK_MIN: f32 = 6.0;
/// The largest hidden-character mark size.
pub const PAGES_FRAME_MARKS_MARK_MAX: f32 = 40.0;
/// The square's side.
pub const PAGES_EDGE_PICKER_SIDE: f32 = 64.0;
/// The lines' inset from the square.
pub const PAGES_EDGE_PICKER_INSET: f32 = 8.0;
/// A chosen line.
pub const PAGES_EDGE_PICKER_LINE_ON: f32 = 2.5;
/// A line not chosen.
pub const PAGES_EDGE_PICKER_LINE_OFF: f32 = 1.0;
/// The line under the pointer.
pub const PAGES_EDGE_PICKER_LINE_HOVER: f32 = 3.0;
/// A square's side.
pub const PAGES_TABLE_SIZE_CELL: f32 = 16.0;
/// The space between squares.
pub const PAGES_TABLE_SIZE_GAP: f32 = 2.0;
/// The range outline.
pub const PAGES_TABLE_SELECTION_OUTLINE: f32 = 1.0;
/// Alpha of the selected cells' tint.
pub const PAGES_TABLE_SELECTION_FILL_ALPHA: u8 = 110;
/// The room the page preview keeps around the page (PdfCraft `canvas.rs:75` `MARGIN`, the value `pages.margin` has).
pub const PAGES_PREVIEW_ROOM: f32 = 28.0;
/// Alpha of an even column's fill in the column bar: 0.11 of the accent (PdfCraft `tools/field_table_ui.rs:362`).
pub const PAGES_COLUMN_BAR_FILL_ALPHA: u8 = 28;
/// Alpha of an odd column's fill in the column bar: 0.055 of the accent (PdfCraft `tools/field_table_ui.rs:362`).
pub const PAGES_COLUMN_BAR_FILL_ALT_ALPHA: u8 = 14;
/// The margin guides of a page grid: InDesign's magenta (DesignCraft `doc/src/lib.rs:476` `margin_color`).
pub const GRID_PAGE_MARGINS: Rgba = Rgba::rgb(255, 56, 255);
/// The column guides of a page grid: InDesign's violet (DesignCraft `doc/src/lib.rs:477` `column_color`).
pub const GRID_PAGE_COLUMNS: Rgba = Rgba::rgb(166, 40, 255);
/// The baseline grid's lines: InDesign's light blue (DesignCraft `doc/src/lib.rs:148` `BaselineGrid::default`).
pub const GRID_PAGE_BASELINE: Rgba = Rgba::rgb(140, 205, 230);

/// Side of a media tile's flag glyph in its badge bar (LightCraft `grid.rs`).
pub const TILE_FLAG: f32 = 14.0;
/// Side of a media tile's colour dot (LightCraft: radius 5).
pub const TILE_DOT: f32 = 10.0;
/// Height of a media tile's badge bar (LightCraft `grid.rs:878`).
pub const TILE_BAR: f32 = 24.0;
/// Side of a media tile's check box in check mode (LightCraft `import/body.rs:417`).
pub const TILE_CHECKBOX: f32 = 16.0;
/// Margin of the scrim behind a media tile's check box (LightCraft `import/body.rs:417`).
pub const TILE_CHECKBOX_BACKDROP: f32 = 2.0;
/// Height of a media tile's badge pill (LightCraft).
pub const TILE_BADGE_HEIGHT: f32 = 16.0;
/// Height of a media tile's status band (FilmCraft).
pub const TILE_STATUS_BAND: f32 = 20.0;
/// Height of a media tile's In/Out range bar (FilmCraft).
pub const TILE_RANGE_BAR: f32 = 4.0;
/// Width of a media tile's selection ring (PhotoCraft preset ring).
pub const TILE_RING_WIDTH: f32 = 2.0;
/// Width of a media tile's hover ring (PhotoCraft preset ring).
pub const TILE_RING_HOVER_WIDTH: f32 = 1.0;
/// Width of the halo outside a media tile's selection ring (PhotoCraft preset ring).
pub const TILE_RING_HALO_WIDTH: f32 = 1.0;
/// Width of the current tile's ring (PdfCraft `panels/pages.rs`: the current page's border).
pub const TILE_RING_CURRENT_WIDTH: f32 = 2.0;
/// Width of the caption column at a tile's reading start (DeckCraft `panels/thumbs.rs:184`: the
/// slide starts 30 in).
pub const TILE_START_CAPTION_WIDTH: f32 = 30.0;
/// Share of the grid's height one wheel notch scrolls (LightCraft `WHEEL_STRIP_SHARE`).
pub const TILE_WHEEL_SHARE: f32 = 1.0 / 3.0;
/// Opacity of the veil over a rejected tile, as a factor of `surface.scrim` (LightCraft alpha 110).
pub const TILE_VEIL_ALPHA: f32 = 0.45;
/// Height of the compare tile's caption strip (LightCraft `compare.rs:219`).
pub const TILE_COMPARE_CAPTION: f32 = 26.0;
/// Margin around the compare panes (LightCraft).
pub const TILE_COMPARE_MARGIN: f32 = 18.0;
/// Gap between the compare panes (LightCraft).
pub const TILE_COMPARE_GAP: f32 = 16.0;
/// Room the compare caption needs to keep the name (LightCraft).
pub const TILE_COMPARE_NAME_ROOM: f32 = 110.0;
/// Side of the media bin's label chip (FilmCraft `project_views.rs`).
pub const TILE_BIN_CHIP: f32 = 12.0;
/// Radius of the media bin's label chip (FilmCraft `project_views.rs`).
pub const TILE_BIN_CHIP_RADIUS: f32 = 2.5;

/// Side of a link status badge (VideoCraft).
pub const LINK_BADGE: f32 = 16.0;
/// Size of the Link Media preview box (FilmCraft `media_dialogs.rs`).
pub const LINK_PREVIEW: [f32; 2] = [192.0, 108.0];
/// Height of the Link Media candidate list (FilmCraft).
pub const LINK_CANDIDATES_HEIGHT: f32 = 108.0;
/// Least height of the Link Media table (FilmCraft).
pub const LINK_TABLE_MIN: f32 = 90.0;
/// Greatest height of the Link Media table (FilmCraft).
pub const LINK_TABLE_MAX: f32 = 320.0;
/// Share of the window's height the Link Media table takes before its clamp (FilmCraft).
pub const LINK_TABLE_SHARE: f32 = 0.3;
/// Greatest width of the Link Media dialog (FilmCraft).
pub const LINK_DIALOG_MAX_WIDTH: f32 = 760.0;

/// The Navigator's default proxy rectangle colour (PhotoCraft `navigator.rs`).
pub const NAVIGATOR_PROXY: Rgba = Rgba::rgb(255, 84, 84);
/// The Navigator proxy's stroke width (PhotoCraft `navigator.rs:105`).
pub const NAVIGATOR_PROXY_WIDTH: f32 = 1.5;
/// Width of the halo under the navigator's proxy outline.
pub const NAVIGATOR_HALO: f32 = 1.0;
/// Corner radius of the proxy on an upright view.
pub const NAVIGATOR_PROXY_RADIUS: f32 = 2.0;
/// Space between the Navigator's preview box and the fitted document.
pub const NAVIGATOR_INSET: f32 = 8.0;
/// The least height of the Navigator's preview box.
pub const NAVIGATOR_MIN_PREVIEW: f32 = 40.0;
/// Side of the Navigator's checkerboard square.
pub const NAVIGATOR_CHECKER: f32 = 6.0;
/// The gap above the Navigator's zoom row and between its controls (PhotoCraft `navigator.rs:109`).
pub const NAVIGATOR_ROW_GAP: f32 = 4.0;
/// The Navigator's named proxy colours after Default: red, orange, yellow, green, blue, violet,
/// gray.
pub const NAVIGATOR_PROXY_COLORS: [Rgba; 7] = [
    Rgba::rgb(255, 84, 84),
    Rgba::rgb(255, 160, 60),
    Rgba::rgb(255, 214, 64),
    Rgba::rgb(92, 214, 110),
    Rgba::rgb(84, 160, 255),
    Rgba::rgb(176, 124, 255),
    Rgba::rgb(200, 200, 200),
];

/// A clipping warning button's side (PhotoCraft `camera_raw_scope_ui.rs:389`).
pub const SCOPES_WARNING_SIZE: f32 = 18.0;
/// The gap between the scope statistics columns (PhotoCraft `histogram_panel.rs:721`).
pub const SCOPES_STATISTICS_GAP: f32 = 6.0;
/// Each per-channel plot of All Channels View (PhotoCraft `histogram_panel.rs:797`).
pub const SCOPES_ALL_CHANNELS_HEIGHT: f32 = 90.0;
/// The vectorscope rings' alpha (PhotoCraft `camera_raw_scope_ui.rs:461`).
pub const SCOPES_VECTOR_RING_ALPHA: f32 = 0.45;
/// The smallest half-side of a vectorscope cloud dot (PhotoCraft `camera_raw_scope_ui.rs:471`).
pub const SCOPES_VECTOR_DOT_MIN: f32 = 0.7;
/// The largest half-side of a vectorscope cloud dot.
pub const SCOPES_VECTOR_DOT_MAX: f32 = 3.0;
/// The scope hover readout badge's height (PhotoCraft `camera_raw_scope_ui.rs:397`).
pub const SCOPES_HOVER_BADGE_HEIGHT: f32 = 20.0;
/// The hovered zone band's alpha (PhotoCraft `camera_raw_scope_ui.rs:417`).
pub const SCOPES_ZONE_ALPHA: f32 = 0.055;
/// The vectorscope's smallest default height (PhotoCraft `camera_raw_scope_ui.rs:511`).
pub const SCOPES_VECTOR_HEIGHT_MIN: f32 = 100.0;
/// The vectorscope's largest default height.
pub const SCOPES_VECTOR_HEIGHT_MAX: f32 = 220.0;
/// The vectorscope's margin between the outer ring and the plot's edge, which holds the hue
/// letters (PhotoCraft `camera_raw_scope_ui.rs:443`).
pub const SCOPES_VECTOR_MARGIN: f32 = 14.0;
/// The inset of a clipping warning's triangle in its button (PhotoCraft `camera_raw_scope_ui.rs:352`).
pub const SCOPES_WARNING_INSET: f32 = 3.0;

/// A set header's height in the Actions panel (PhotoCraft `actions.rs:831`).
pub const ACTIONS_SET_ROW_HEIGHT: f32 = 24.0;
/// An action row's height in the Actions panel.
pub const ACTIONS_ROW_HEIGHT: f32 = 26.0;
/// A step row's height in the Actions panel.
pub const ACTIONS_STEP_ROW_HEIGHT: f32 = 20.0;
/// The radius of the dot on the action being recorded (PhotoCraft `actions.rs:1040`).
pub const ACTIONS_RECORDING_DOT_RADIUS: f32 = 4.0;
/// The Button Mode fills, red to violet (PhotoCraft `actions.rs:297-313`).
pub const ACTIONS_COLORS: [Rgba; 6] = [
    Rgba::rgb(196, 64, 64),
    Rgba::rgb(204, 120, 40),
    Rgba::rgb(196, 176, 48),
    Rgba::rgb(56, 140, 72),
    Rgba::rgb(48, 104, 176),
    Rgba::rgb(120, 72, 168),
];
/// A Button Mode label on a coloured fill (PhotoCraft `actions.rs:297-313`).
pub const ACTIONS_LABEL_ON_COLOR: Rgba = Rgba::rgb(20, 20, 20);

// --- select ---
/// Side of a Magnetic Lasso fastening point: `select.fastening`.
pub const SELECT_FASTENING: f32 = 5.0;
/// Radius of the Magnetic Lasso closing ring: `select.ring_radius`.
pub const SELECT_RING_RADIUS: f32 = 4.0;
/// Offset of the ring right of and below the pointer: `select.ring_offset`.
pub const SELECT_RING_OFFSET: f32 = 12.0;
/// Width of the ring halo: `select.ring_halo`.
pub const SELECT_RING_HALO: f32 = 3.0;
/// Width of the ring line: `select.ring_line`.
pub const SELECT_RING_LINE: f32 = 1.2;
/// Smallest radius of the Magnetic Lasso width circle: `select.width_ring_min`.
pub const SELECT_WIDTH_RING_MIN: f32 = 2.0;
/// Gap between the width circle's two rings: `select.width_ring_gap`.
pub const SELECT_WIDTH_RING_GAP: f32 = 0.5;

// --- hover ---
/// Line width of a rect or path outline: `hover.outline`.
pub const HOVER_OUTLINE: f32 = 1.5;
/// A mask texel's alpha per unit of coverage: `hover.fill_alpha`.
pub const HOVER_FILL_ALPHA: f32 = 0.42;
/// A mask edge texel's alpha: `hover.edge_alpha`.
pub const HOVER_EDGE_ALPHA: f32 = 0.95;
/// How long the pointer rests before a lookup, in milliseconds: `hover.debounce_ms`.
pub const HOVER_DEBOUNCE_MS: f32 = 120.0;
/// Repaint interval while a lookup runs, in milliseconds: `hover.busy_ms`.
pub const HOVER_BUSY_MS: f32 = 40.0;

// --- outline_draft ---
/// A press this near the first point closes an outline, in screen points: `outline_draft.close`.
pub const OUTLINE_DRAFT_CLOSE: f32 = 8.0;
/// A press this near the last point is the same click again: `outline_draft.same_point`.
pub const OUTLINE_DRAFT_SAME_POINT: f32 = 3.0;

// --- selection_frame ---
/// The box line and a solid outline: `selection_frame.line`.
pub const SELECTION_FRAME_LINE: f32 = 1.0;
/// The key object's box line: `selection_frame.key_line`.
pub const SELECTION_FRAME_KEY_LINE: f32 = 3.0;
/// The centre mark's side: `selection_frame.centre`.
pub const SELECTION_FRAME_CENTRE: f32 = 3.5;
/// A dashed outline's width: `selection_frame.outline`.
pub const SELECTION_FRAME_OUTLINE: f32 = 1.5;
/// A dashed outline's dash: `selection_frame.dash`.
pub const SELECTION_FRAME_DASH: f32 = 4.0;
/// A dashed outline's gap: `selection_frame.gap`.
pub const SELECTION_FRAME_GAP: f32 = 3.0;
/// What one grow step adds to a grip: `selection_frame.grow_step`.
pub const SELECTION_FRAME_GROW_STEP: f32 = 1.0;

// --- grip ---
/// A square grip's side, a midpoint's width: `grip.size`.
pub const GRIP_SIZE: f32 = 10.0;
/// A midpoint grip's height: `grip.midpoint_height`.
pub const GRIP_MIDPOINT_HEIGHT: f32 = 5.0;
/// The outline width: `grip.stroke`.
pub const GRIP_STROKE: f32 = 1.0;
/// The hit area beyond a grip's bounds: `grip.hit_pad`.
pub const GRIP_HIT_PAD: f32 = 3.0;
/// The warm area beyond a grip's bounds: `grip.warm_pad`.
pub const GRIP_WARM_PAD: f32 = 2.0;

// --- constrain ---
/// Screen points of travel after which ⌥ held at the press copies: `constrain.copy_reach`.
pub const CONSTRAIN_COPY_REACH: f32 = 2.0;
/// Screen points a stroke travels before its direction locks: `constrain.stroke_lock`.
pub const CONSTRAIN_STROKE_LOCK: f32 = 3.0;

// --- line_guide ---
/// The line width: `line_guide.width`.
pub const LINE_GUIDE_WIDTH: f32 = 1.0;
/// The halo's width, under the line: `line_guide.halo`.
pub const LINE_GUIDE_HALO: f32 = 3.0;

// --- before_after ---
/// The divider line's width: `before_after.line`.
pub const BEFORE_AFTER_LINE: f32 = 2.0;
/// The halo's extra width on each side of the line: `before_after.halo`.
pub const BEFORE_AFTER_HALO: f32 = 1.0;
/// The knob's diameter: `before_after.knob`.
pub const BEFORE_AFTER_KNOB: f32 = 20.0;
/// Between two side-by-side panes: `before_after.gap_side`.
pub const BEFORE_AFTER_GAP_SIDE: f32 = 12.0;
/// Between two stacked panes: `before_after.gap_stacked`.
pub const BEFORE_AFTER_GAP_STACKED: f32 = 28.0;
/// Below a caption: `before_after.caption_gap`.
pub const BEFORE_AFTER_CAPTION_GAP: f32 = 4.0;

// --- brush ---
/// Screen points of vertical resize travel from level 0 to 1: `brush.resize_travel`.
pub const BRUSH_RESIZE_TRAVEL: f32 = 200.0;
/// Below this on-screen radius the outline is unreadable and the crosshair shows: `brush.tip_min_radius`.
pub const BRUSH_TIP_MIN_RADIUS: f32 = 3.0;
/// The arm of that crosshair: `brush.tip_min_crosshair`.
pub const BRUSH_TIP_MIN_CROSSHAIR: f32 = 4.0;
/// The picker's width before the user drags its grip: `brush.picker_width`.
pub const BRUSH_PICKER_WIDTH: f32 = 300.0;
/// Its height: `brush.picker_height`.
pub const BRUSH_PICKER_HEIGHT: f32 = 420.0;
/// Its least width: `brush.picker_min_width`.
pub const BRUSH_PICKER_MIN_WIDTH: f32 = 230.0;
/// Its least height: `brush.picker_min_height`.
pub const BRUSH_PICKER_MIN_HEIGHT: f32 = 170.0;
/// The Size and Hardness block's greatest width: `brush.picker_head_max`.
pub const BRUSH_PICKER_HEAD_MAX: f32 = 350.0;
/// The least height of the card list: `brush.picker_list_min`.
pub const BRUSH_PICKER_LIST_MIN: f32 = 80.0;
/// Between the head and the search row, and around the hairline: `brush.picker_gap`.
pub const BRUSH_PICKER_GAP: f32 = 6.0;
/// Room the footer slider leaves for the grip: `brush.picker_grip_room`.
pub const BRUSH_PICKER_GRIP_ROOM: f32 = 20.0;
/// The search field's and the card scale slider's least width: `brush.picker_field_min`.
pub const BRUSH_PICKER_FIELD_MIN: f32 = 60.0;
/// Padding inside a card's tile: `brush.card_pad`.
pub const BRUSH_CARD_PAD: f32 = 4.0;
/// The tile's inset in its cell, at both ends: `brush.card_inset`.
pub const BRUSH_CARD_INSET: f32 = 4.0;
/// The tip cell's width: `brush.card_tip_width`.
pub const BRUSH_CARD_TIP_WIDTH: f32 = 46.0;
/// The tip thumbnail's base side: `brush.card_tip_base`.
pub const BRUSH_CARD_TIP_BASE: f32 = 44.8;
/// The name row's height: `brush.card_name_height`.
pub const BRUSH_CARD_NAME_HEIGHT: f32 = 20.0;
/// The name's inset from the row's ends: `brush.card_name_inset`.
pub const BRUSH_CARD_NAME_INSET: f32 = 6.0;
/// A compact card's tip thumbnail sits this far inside its cell's shorter side: `brush.card_tip_compact_inset`.
pub const BRUSH_CARD_TIP_COMPACT_INSET: f32 = 2.0;
/// The size number's distance from the bottom of the tip cell: `brush.card_number_gap`.
pub const BRUSH_CARD_NUMBER_GAP: f32 = 1.0;
/// The tip thumbnail's distance above the size number: `brush.card_tip_gap`.
pub const BRUSH_CARD_TIP_GAP: f32 = 2.0;
/// The stroke's inset from the tip cell: `brush.card_stroke_inset`.
pub const BRUSH_CARD_STROKE_INSET: f32 = 8.0;
/// The stroke's least width: `brush.card_stroke_min`.
pub const BRUSH_CARD_STROKE_MIN: f32 = 24.0;
/// The stroke's greatest width: `brush.card_stroke_max`.
pub const BRUSH_CARD_STROKE_MAX: f32 = 260.0;
/// The stroke's height: `brush.card_stroke_height`.
pub const BRUSH_CARD_STROKE_HEIGHT: f32 = 36.0;
/// A full card, width then height: `brush.card_max`.
pub const BRUSH_CARD_MAX: [f32; 2] = [300.0, 70.0];
/// A tip-only cell, width then height: `brush.card_tip_only`.
pub const BRUSH_CARD_TIP_ONLY: [f32; 2] = [60.0, 70.0];
/// A tip-and-name cell, width then height: `brush.card_tip_name`.
pub const BRUSH_CARD_TIP_NAME: [f32; 2] = [150.0, 70.0];
/// The card tile's corner radius: `brush.card_radius`.
pub const BRUSH_CARD_RADIUS: f32 = 3.0;
/// The options-bar chip, width then height: `brush.chip`.
pub const BRUSH_CHIP: [f32; 2] = [44.0, 30.0];
/// The chip's tip image side: `brush.chip_tip`.
pub const BRUSH_CHIP_TIP: f32 = 14.0;
/// The tip image's centre from the chip's leading top corner: `brush.chip_tip_at`.
pub const BRUSH_CHIP_TIP_AT: [f32; 2] = [14.0, 11.0];
/// The chip's chevron side: `brush.chip_chevron`.
pub const BRUSH_CHIP_CHEVRON: f32 = 10.0;
/// The chip's size number, its centre's distance from the chip's top: `brush.chip_number_at`.
pub const BRUSH_CHIP_NUMBER_AT: f32 = 25.0;
/// The chevron's centre, its distance from the chip's trailing end: `brush.chip_chevron_inset`.
pub const BRUSH_CHIP_CHEVRON_INSET: f32 = 9.0;
/// The Brush Settings panel's preferred width: `brush.panel_width`.
pub const BRUSH_PANEL_WIDTH: f32 = 540.0;
/// The section list's width: `brush.section_list_width`.
pub const BRUSH_SECTION_LIST_WIDTH: f32 = 150.0;
/// A section row's height: `brush.section_row`.
pub const BRUSH_SECTION_ROW: f32 = 23.0;
/// Before the first switch row: `brush.section_switch_gap`.
pub const BRUSH_SECTION_SWITCH_GAP: f32 = 6.0;
/// The stroke strip's height: `brush.strip_height`.
pub const BRUSH_STRIP_HEIGHT: f32 = 76.0;
/// The Control select's width in a dynamic control row: `brush.control_select`.
pub const BRUSH_CONTROL_SELECT: f32 = 130.0;
/// The tip ellipse widget's side: `brush.ellipse_size`.
pub const BRUSH_ELLIPSE_SIZE: f32 = 84.0;
/// The disc's radius: `brush.ellipse_disc`.
pub const BRUSH_ELLIPSE_DISC: f32 = 40.0;
/// The ellipse's major radius: `brush.ellipse_radius`.
pub const BRUSH_ELLIPSE_RADIUS: f32 = 34.0;
/// A roundness dot's radius: `brush.ellipse_dot`.
pub const BRUSH_ELLIPSE_DOT: f32 = 4.0;
/// How near a dot a press grabs it: `brush.ellipse_grab`.
pub const BRUSH_ELLIPSE_GRAB: f32 = 7.0;
/// The ellipse outline and arrow width: `brush.ellipse_line`.
pub const BRUSH_ELLIPSE_LINE: f32 = 1.5;
/// How far the arrow reaches past the ellipse: `brush.ellipse_arrow_reach`.
pub const BRUSH_ELLIPSE_ARROW_REACH: f32 = 5.0;
/// The arrow head's length then width: `brush.ellipse_head`.
pub const BRUSH_ELLIPSE_HEAD: [f32; 2] = [7.0, 8.0];
/// A cell of the grid of tips, width then height: `brush.tip_grid_cell`.
pub const BRUSH_TIP_GRID_CELL: [f32; 2] = [44.0, 50.0];
/// A tip's side in that grid before its size scale: `brush.tip_grid_thumb`.
pub const BRUSH_TIP_GRID_THUMB: f32 = 34.0;
/// The grid's greatest height: `brush.tip_grid_max`.
pub const BRUSH_TIP_GRID_MAX: f32 = 104.0;

// --- full_window ---
/// A standard side column: `full_window.side.width`.
pub const FULL_WINDOW_SIDE_WIDTH: f32 = 300.0;
/// A wide side column: `full_window.side.width_wide`.
pub const FULL_WINDOW_SIDE_WIDTH_WIDE: f32 = 350.0;
/// A narrow side column: `full_window.side.width_narrow`.
pub const FULL_WINDOW_SIDE_WIDTH_NARROW: f32 = 220.0;
/// The padding inside a side column, across: `full_window.side.pad_x`.
pub const FULL_WINDOW_SIDE_PAD_X: f32 = 14.0;
/// The padding inside a side column, along: `full_window.side.pad_y`.
pub const FULL_WINDOW_SIDE_PAD_Y: f32 = 12.0;
/// The room kept on each side of a fitted image: `full_window.canvas.margin`.
pub const FULL_WINDOW_CANVAS_MARGIN: f32 = 20.0;
/// The zoom bar's height: `full_window.zoom_bar.height`.
pub const FULL_WINDOW_ZOOM_BAR_HEIGHT: f32 = 30.0;
/// The zoom bar's padding, across: `full_window.zoom_bar.pad_x`.
pub const FULL_WINDOW_ZOOM_BAR_PAD_X: f32 = 10.0;
/// The zoom bar's padding, along: `full_window.zoom_bar.pad_y`.
pub const FULL_WINDOW_ZOOM_BAR_PAD_Y: f32 = 3.0;
/// The zoom bar's progress bar width: `full_window.progress.width`.
pub const FULL_WINDOW_PROGRESS_WIDTH: f32 = 90.0;

/// Alpha (of 255) of the brush resize preview's default tint (PhotoCraft's).
pub const BRUSH_PREVIEW_ALPHA: u8 = 110;
/// Share of `surface.raised` a brush card's tile keeps (PhotoCraft darkens it to 90 %).
pub const BRUSH_CARD_FILL_SHARE: f32 = 0.9;

/// `c` with each colour channel multiplied by `share`, in gamma (sRGB) space.
pub(super) fn darkened(c: Rgba, share: f32) -> Rgba {
    let [r, g, b, a] = c.0;
    let k = |v: u8| (f32::from(v) * share + 0.5) as u8;
    Rgba([k(r), k(g), k(b), a])
}

// --- corner ---
/// The widget's diameter: `corner.widget`.
pub const CORNER_WIDGET: f32 = 10.0;
/// The inner dot's diameter: `corner.dot`.
pub const CORNER_DOT: f32 = 5.0;
/// The outline's width: `corner.line`.
pub const CORNER_LINE: f32 = 1.5;
/// The pointer's reach: `corner.hit`.
pub const CORNER_HIT: f32 = 7.0;
/// Farthest a press on a widget may move and still be a click (the value of `bezier.click_slop`): `corner.click_slop`.
pub const CORNER_CLICK_SLOP: f32 = 1.0;
/// The least distance inside a corner: `corner.inset`.
pub const CORNER_INSET: f32 = 10.0;
/// The shortest side, on the screen, whose corners show widgets: `corner.min_side`.
pub const CORNER_MIN_SIDE: f32 = 30.0;

// --- stroke_editor ---
/// The popover body's width: `stroke_editor.width`.
pub const STROKE_EDITOR_WIDTH: f32 = 272.0;
/// The room kept between the popover and the window's edge: `stroke_editor.screen_margin`.
pub const STROKE_EDITOR_SCREEN_MARGIN: f32 = 24.0;
/// The room kept above and below the popover: `stroke_editor.screen_margin_y`.
pub const STROKE_EDITOR_SCREEN_MARGIN_Y: f32 = 40.0;
/// The narrowest the body gets in a small window: `stroke_editor.min_width`.
pub const STROKE_EDITOR_MIN_WIDTH: f32 = 160.0;
/// The shortest the popover's scrolling body gets: `stroke_editor.min_height`.
pub const STROKE_EDITOR_MIN_HEIGHT: f32 = 80.0;
/// The opener's width: `stroke_editor.opener_width`.
pub const STROKE_EDITOR_OPENER_WIDTH: f32 = 138.0;
/// The opener's height: `stroke_editor.opener_height`.
pub const STROKE_EDITOR_OPENER_HEIGHT: f32 = 24.0;
/// A preset row's height: `stroke_editor.preset_height`.
pub const STROKE_EDITOR_PRESET_HEIGHT: f32 = 32.0;
/// A choice select's width: `stroke_editor.choice_width`.
pub const STROKE_EDITOR_CHOICE_WIDTH: f32 = 152.0;
/// A number field's width: `stroke_editor.field_width`.
pub const STROKE_EDITOR_FIELD_WIDTH: f32 = 74.0;
/// The tall sample's height: `stroke_editor.sample_height`.
pub const STROKE_EDITOR_SAMPLE_HEIGHT: f32 = 58.0;
/// The samples' line width: `stroke_editor.sample_stroke`.
pub const STROKE_EDITOR_SAMPLE_STROKE: f32 = 2.5;
/// The tall sample's rectangle inset: `stroke_editor.sample_inset`.
pub const STROKE_EDITOR_SAMPLE_INSET: f32 = 12.0;
/// The open sample line's distance from the sample's ends: `stroke_editor.sample_margin`.
pub const STROKE_EDITOR_SAMPLE_MARGIN: f32 = 4.0;
/// The off opener's line width: `stroke_editor.off_stroke`.
pub const STROKE_EDITOR_OFF_STROKE: f32 = 1.0;
/// The off opener's slash width: `stroke_editor.off_slash_stroke`.
pub const STROKE_EDITOR_OFF_SLASH_STROKE: f32 = 1.5;

// --- measure ---
/// A press within this many screen points of an end grabs it: `measure.hit`.
pub const MEASURE_HIT: f32 = 7.0;
/// Each arm of an end's cross, in screen points: `measure.handle_arm`.
pub const MEASURE_HANDLE_ARM: f32 = 5.0;
/// The gap after each readout in the options bar: `measure.readout_gap`.
pub const MEASURE_READOUT_GAP: f32 = 6.0;

// --- symmetry ---
/// Width of each axis line: `symmetry.line`.
pub const SYMMETRY_LINE: f32 = 1.0;
/// A press within this many screen points of the centre or rotation handle takes it: `symmetry.hit`.
pub const SYMMETRY_HIT: f32 = 8.0;
/// The rotation handle's distance from the centre as a share of the document's shorter side (a ratio, not a size): `symmetry.handle_share`.
pub const SYMMETRY_HANDLE_SHARE: f32 = 0.35;

// --- adjust ---
/// A Hue/Saturation spectrum bar's height: `adjust.spectrum_bar`.
pub const ADJUST_SPECTRUM_BAR: f32 = 10.0;
/// The range band's height between the bars: `adjust.range_band`.
pub const ADJUST_RANGE_BAND: f32 = 6.0;
/// The height between the two bars that holds the range thumbs: `adjust.spectrum_master_extra`.
pub const ADJUST_SPECTRUM_MASTER_EXTRA: f32 = 20.0;
/// A range thumb's width: `adjust.range_thumb_width`.
pub const ADJUST_RANGE_THUMB_WIDTH: f32 = 5.0;
/// A range thumb's height: `adjust.range_thumb_height`.
pub const ADJUST_RANGE_THUMB_HEIGHT: f32 = 12.0;
/// A range thumb's corner radius (PhotoCraft `hue_saturation.rs:177`): `adjust.range_thumb_radius`.
pub const ADJUST_RANGE_THUMB_RADIUS: f32 = 1.0;
/// Distance from a press within which a range thumb is grabbed (PhotoCraft `hue_saturation.rs:25`): `adjust.hue_grab`.
pub const ADJUST_HUE_GRAB: f32 = 10.0;
/// A point picker's tone dot radius: `adjust.picker_dot`.
pub const ADJUST_PICKER_DOT: f32 = 3.5;
/// The tone dot's centre, in from the button's bottom-trailing corner: `adjust.picker_dot_inset`.
pub const ADJUST_PICKER_DOT_INSET: f32 = 5.5;
/// Threshold's histogram height: `adjust.threshold_histogram`.
pub const ADJUST_THRESHOLD_HISTOGRAM: f32 = 90.0;
/// Threshold's histogram widest width: `adjust.threshold_width`.
pub const ADJUST_THRESHOLD_WIDTH: f32 = 300.0;
/// Threshold's level marker inset from the histogram's edges (PhotoCraft `basic.rs:78`): `adjust.threshold_inset`.
pub const ADJUST_THRESHOLD_INSET: f32 = 6.0;
/// Width of Threshold's level marker (PhotoCraft `basic.rs:83`): `adjust.threshold_marker_width`.
pub const ADJUST_THRESHOLD_MARKER_WIDTH: f32 = 1.0;
/// The alpha multiplier of a disabled picker's tone dot and ring (PhotoCraft `adjust_dialog.rs:181`): `adjust.disabled_alpha`.
pub const ADJUST_DISABLED_ALPHA: f32 = 0.4;

// --- layer_style ---
/// The effect list's width: `layer_style.list_width`.
pub const LAYER_STYLE_LIST_WIDTH: f32 = 190.0;
/// The page column's width: `layer_style.page_width`.
pub const LAYER_STYLE_PAGE_WIDTH: f32 = 330.0;
/// A Styles page thumbnail's side: `layer_style.preset_thumb`.
pub const LAYER_STYLE_PRESET_THUMB: f32 = 32.0;
/// The Styles page's scroll height: `layer_style.presets_height`.
pub const LAYER_STYLE_PRESETS_HEIGHT: f32 = 340.0;
/// The contour field's thumbnail side: `layer_style.contour_thumb`.
pub const LAYER_STYLE_CONTOUR_THUMB: f32 = 36.0;
/// The contour field's picker arrow width: `layer_style.contour_arrow`.
pub const LAYER_STYLE_CONTOUR_ARROW: f32 = 14.0;
/// A cell of the contour preset picker: `layer_style.contour_cell`.
pub const LAYER_STYLE_CONTOUR_CELL: f32 = 40.0;
/// The Contour Editor's graph side: `layer_style.contour_graph`.
pub const LAYER_STYLE_CONTOUR_GRAPH: f32 = 220.0;

// --- blend_if ---
/// The bar's narrowest width: `blend_if.width_min`.
pub const BLEND_IF_WIDTH_MIN: f32 = 120.0;
/// The bar's widest width: `blend_if.width_max`.
pub const BLEND_IF_WIDTH_MAX: f32 = 320.0;
/// The bar's height: `blend_if.height`.
pub const BLEND_IF_HEIGHT: f32 = 26.0;
/// The ramp's height: `blend_if.ramp`.
pub const BLEND_IF_RAMP: f32 = 12.0;
/// The ramp's side inset: `blend_if.inset`.
pub const BLEND_IF_INSET: f32 = 6.0;
/// A handle triangle's width: `blend_if.handle_width`.
pub const BLEND_IF_HANDLE_WIDTH: f32 = 10.0;
/// A handle triangle's height: `blend_if.handle_height`.
pub const BLEND_IF_HANDLE_HEIGHT: f32 = 10.0;

// --- add_beside ---
/// From the object's edge to a button's centre, in screen points: `add_beside.offset`.
pub const ADD_BESIDE_OFFSET: f32 = 18.0;
/// A button's radius: `add_beside.radius`.
pub const ADD_BESIDE_RADIUS: f32 = 8.0;
/// The plus's arm as a share of the radius (a ratio, not a size): `add_beside.arm`.
pub const ADD_BESIDE_ARM: f32 = 0.5;
/// The plus's width: `add_beside.line`.
pub const ADD_BESIDE_LINE: f32 = 1.5;
/// The ring's width: `add_beside.ring`.
pub const ADD_BESIDE_RING: f32 = 1.0;

// --- shape_drag ---
/// A press and release closer than this many screen points is a click: `shape_drag.click_slop`.
pub const SHAPE_DRAG_CLICK_SLOP: f32 = 3.0;
/// The outline of the shape being drawn: `shape_drag.preview_line`.
pub const SHAPE_DRAG_PREVIEW_LINE: f32 = 1.5;
/// The dark halo's extra width on each side of the preview outline (PhotoCraft's `contrast_path`: 3 under 1): `shape_drag.preview_halo`.
pub const SHAPE_DRAG_PREVIEW_HALO: f32 = 1.0;

// --- slice ---
/// A slice's outline width: `slice.outline`.
pub const SLICE_OUTLINE: f32 = 1.0;
/// The selected slice's outline width: `slice.selected_outline`.
pub const SLICE_SELECTED_OUTLINE: f32 = 2.0;
/// An auto slice's dash: `slice.auto_dash`.
pub const SLICE_AUTO_DASH: f32 = 3.0;
/// An auto slice's gap: `slice.auto_gap`.
pub const SLICE_AUTO_GAP: f32 = 3.0;
/// An auto slice's alpha over the slice colour (a ratio, not a size): `slice.auto_alpha`.
pub const SLICE_AUTO_ALPHA: f32 = 0.7;
/// A new slice preview's dash: `slice.drag_dash`.
pub const SLICE_DRAG_DASH: f32 = 4.0;
/// A new slice preview's gap: `slice.drag_gap`.
pub const SLICE_DRAG_GAP: f32 = 4.0;
/// The badge's offset from the slice's corner: `slice.badge_offset`.
pub const SLICE_BADGE_OFFSET: f32 = 2.0;
/// The badge's padding around the number, horizontal then vertical: `slice.badge_pad`.
pub const SLICE_BADGE_PAD: [f32; 2] = [3.0, 1.0];
/// The badge's corner radius: `slice.badge_radius`.
pub const SLICE_BADGE_RADIUS: f32 = 2.0;
/// The smallest slice on screen that shows a badge, width then height: `slice.badge_min`.
pub const SLICE_BADGE_MIN: [f32; 2] = [18.0, 12.0];
/// A drag no wider and no taller than this many document pixels is a click: `slice.click_slop`.
pub const SLICE_CLICK_SLOP: f32 = 2.0;

// --- artboard ---
/// A drag shorter than this many screen points draws nothing: `artboard.click_slop`.
pub const ARTBOARD_CLICK_SLOP: f32 = 3.0;
/// The drag preview's width: `artboard.preview_line`.
pub const ARTBOARD_PREVIEW_LINE: f32 = 1.5;

// --- connector ---
/// The route and rings' width: `connector.line`.
pub const CONNECTOR_LINE: f32 = 1.5;
/// An attach point's radius: `connector.attach`.
pub const CONNECTOR_ATTACH: f32 = 4.0;
/// The active attach point's radius: `connector.attach_active`.
pub const CONNECTOR_ATTACH_ACTIVE: f32 = 7.0;
/// The active attach point's fill alpha (a ratio, not a size): `connector.attach_fill_alpha`.
pub const CONNECTOR_ATTACH_FILL_ALPHA: f32 = 0.25;
/// The ghost's fill alpha (a ratio, not a size): `connector.ghost_alpha`.
pub const CONNECTOR_GHOST_ALPHA: f32 = 0.08;
/// The ghost outline's dash: `connector.ghost_dash`.
pub const CONNECTOR_GHOST_DASH: f32 = 6.0;
/// The ghost outline's gap: `connector.ghost_gap`.
pub const CONNECTOR_GHOST_GAP: f32 = 4.0;
/// The ghost outline's alpha (a ratio, not a size): `connector.ghost_line_alpha`.
pub const CONNECTOR_GHOST_LINE_ALPHA: f32 = 0.6;
/// A drag shorter than this many screen points makes nothing: `connector.click_slop`.
pub const CONNECTOR_CLICK_SLOP: f32 = 4.0;
/// An end handle's radius: `connector.end_handle`.
pub const CONNECTOR_END_HANDLE: f32 = 5.0;
/// A path handle's radius: `connector.path_handle`.
pub const CONNECTOR_PATH_HANDLE: f32 = 3.5;
/// An end within this many screen points of a site glues to it: `connector.attach_reach`.
pub const CONNECTOR_ATTACH_REACH: f32 = 10.0;
/// An end or path handle within this many screen points of a press is grabbed: `connector.handle_reach`.
pub const CONNECTOR_HANDLE_REACH: f32 = 8.0;
/// An elbow run shorter than this many screen points has no path handle: `connector.min_run`.
pub const CONNECTOR_MIN_RUN: f32 = 28.0;
/// A target is hit within this many screen points of its bounds: `connector.target_tolerance`.
pub const CONNECTOR_TARGET_TOLERANCE: f32 = 2.0;
/// A path handle's ring width: `connector.path_ring`.
pub const CONNECTOR_PATH_RING: f32 = 1.0;
/// The hovered target's outline width: `connector.drop_outline`.
pub const CONNECTOR_DROP_OUTLINE: f32 = 1.0;

// --- vanishing_point ---
/// A plane's outline width: `vanishing_point.line`.
pub const VANISHING_POINT_LINE: f32 = 1.25;
/// The selected plane's outline width: `vanishing_point.line_selected`.
pub const VANISHING_POINT_LINE_SELECTED: f32 = 2.0;
/// A grid line's width: `vanishing_point.grid`.
pub const VANISHING_POINT_GRID: f32 = 0.75;
/// A grid line's alpha: `vanishing_point.grid_alpha`.
pub const VANISHING_POINT_GRID_ALPHA: f32 = 0.6;
/// A node square's side: `vanishing_point.node`.
pub const VANISHING_POINT_NODE: f32 = 7.0;
/// Create Plane's pending corner square: `vanishing_point.pending`.
pub const VANISHING_POINT_PENDING: f32 = 6.0;
/// The extra width of the halo: `vanishing_point.halo_width`.
pub const VANISHING_POINT_HALO_WIDTH: f32 = 2.5;
/// An arm of the source cross: `vanishing_point.source_arm`.
pub const VANISHING_POINT_SOURCE_ARM: f32 = 7.0;
/// How far from a node's centre a press reaches, in points (PhotoCraft `SNAP_PX`): `vanishing_point.press_reach`.
pub const VANISHING_POINT_PRESS_REACH: f32 = 8.0;
/// The click-versus-drag distance of a torn plane's far edge node, in document pixels (PhotoCraft `vp_ui.rs:728`): `vanishing_point.click_slop`.
pub const VANISHING_POINT_CLICK_SLOP: f32 = 2.0;
/// The white line of the stamp ring and the source cross: `vanishing_point.ring_line`.
pub const VANISHING_POINT_RING_LINE: f32 = 1.0;
/// The alpha of the pasted image's white tint, 0 to 1 (PhotoCraft draws it at 230 of 255): `vanishing_point.paste_alpha`.
pub const VANISHING_POINT_PASTE_ALPHA: f32 = 230.0 / 255.0;

// --- wide_angle ---
/// A constraint line's width: `wide_angle.line`.
pub const WIDE_ANGLE_LINE: f32 = 2.0;
/// The end dots' radius: `wide_angle.end`.
pub const WIDE_ANGLE_END: f32 = 3.5;
/// How near a right-click must pass a curve to open its menu: `wide_angle.hit`.
pub const WIDE_ANGLE_HIT: f32 = 10.0;
/// A drag shorter than this many screen points adds no constraint: `wide_angle.min_drag`.
pub const WIDE_ANGLE_MIN_DRAG: f32 = 6.0;
/// The extra width of the halo: `wide_angle.halo_width`.
pub const WIDE_ANGLE_HALO_WIDTH: f32 = 2.5;

// --- blur_pins ---
/// A pin's dot radius: `blur_pins.dot`.
pub const BLUR_PINS_DOT: f32 = 5.0;
/// How near a press grabs a pin: `blur_pins.grab`.
pub const BLUR_PINS_GRAB: f32 = 9.0;
/// The ring's radius: `blur_pins.ring`.
pub const BLUR_PINS_RING: f32 = 24.0;
/// The ring's band width: `blur_pins.ring_band`.
pub const BLUR_PINS_RING_BAND: f32 = 5.0;
/// A shape handle's radius: `blur_pins.handle`.
pub const BLUR_PINS_HANDLE: f32 = 4.5;
/// How near a press grabs a Tilt-Shift line: `blur_pins.line_grab`.
pub const BLUR_PINS_LINE_GRAB: f32 = 5.0;
/// A mark's line width: `blur_pins.line_width`.
pub const BLUR_PINS_LINE_WIDTH: f32 = 1.5;
/// The halo under a mark's line: `blur_pins.line_halo`.
pub const BLUR_PINS_LINE_HALO: f32 = 3.0;
/// The amount arc's width: `blur_pins.arc`.
pub const BLUR_PINS_ARC: f32 = 2.5;
/// The halo under the amount arc: `blur_pins.arc_halo`.
pub const BLUR_PINS_ARC_HALO: f32 = 4.5;
/// A dash of a Tilt-Shift transition line: `blur_pins.dash`.
pub const BLUR_PINS_DASH: f32 = 6.0;
/// The gap between dashes: `blur_pins.dash_gap`.
pub const BLUR_PINS_DASH_GAP: f32 = 5.0;
/// A handle's outline width: `blur_pins.handle_outline`.
pub const BLUR_PINS_HANDLE_OUTLINE: f32 = 1.0;
/// How much larger than a handle a feather dot is: `blur_pins.feather_extra`.
pub const BLUR_PINS_FEATHER_EXTRA: f32 = 1.0;
/// How much larger than a handle a Tilt-Shift turn dot is: `blur_pins.turn_extra`.
pub const BLUR_PINS_TURN_EXTRA: f32 = 0.5;
/// The rounding square's corner radius: `blur_pins.round_corner`.
pub const BLUR_PINS_ROUND_CORNER: f32 = 1.0;

// --- liquify ---
/// The Freeze Lasso's line width: `liquify.lasso`.
pub const LIQUIFY_LASSO: f32 = 1.5;
/// The freeze mask's opacity over the image: `liquify.mask_opacity`.
pub const LIQUIFY_MASK_OPACITY: f32 = 0.5;

// --- select_mask ---
/// A click this near the Polygonal Lasso's first vertex closes it: `select_mask.close_reach`.
pub const SELECT_MASK_CLOSE_REACH: f32 = 6.0;
/// An Object Selection drag shorter than this is a click: `select_mask.click_reach`.
pub const SELECT_MASK_CLICK_REACH: f32 = 4.0;
/// The opacity of the accent for the stroke being drawn: `select_mask.trail`.
pub const SELECT_MASK_TRAIL_OPACITY: f32 = 0.43;

// --- filter_gallery ---
/// A filter thumbnail's width: `filter_gallery.thumb_width`.
pub const FILTER_GALLERY_THUMB_WIDTH: f32 = 80.0;
/// A filter thumbnail's height: `filter_gallery.thumb_height`.
pub const FILTER_GALLERY_THUMB_HEIGHT: f32 = 56.0;
/// The effect-layer stack's height: `filter_gallery.stack_height`.
pub const FILTER_GALLERY_STACK_HEIGHT: f32 = 190.0;
