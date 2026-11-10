//! Semantic tokens: the resolved values component tokens derive from. Colours, shape, space,
//! sizes, type roles, elevation and motion, all concrete.

use super::color::Rgba;
use super::components::{ComponentField, ComponentTokens};
use super::scale::{Easing, Elevation};
use super::theme::EdgeSeed;

/// One value per status: good, warn, bad, info.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Status<T> {
    /// Success, valid input.
    pub good: T,
    /// Caution, unsaved or risky state.
    pub warn: T,
    /// Errors and destructive actions.
    pub bad: T,
    /// Neutral information.
    pub info: T,
}

impl<T> Status<T> {
    /// The value for `kind`.
    pub fn get(&self, kind: StatusKind) -> &T {
        match kind {
            StatusKind::Good => &self.good,
            StatusKind::Warn => &self.warn,
            StatusKind::Bad => &self.bad,
            StatusKind::Info => &self.info,
        }
    }

    /// Applies `f` to each value.
    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> Status<U> {
        Status { good: f(&self.good), warn: f(&self.warn), bad: f(&self.bad), info: f(&self.info) }
    }
}

/// Names one of the four statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatusKind {
    /// Success, valid input.
    Good,
    /// Caution, unsaved or risky state.
    Warn,
    /// Errors and destructive actions.
    Bad,
    /// Neutral information.
    Info,
}

/// Window and container backgrounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceColors {
    /// The window background.
    pub background: Rgba,
    /// Around the document on a canvas; follows `background` unless pinned.
    pub canvas: Rgba,
    /// The dots or grid an app may draw on the canvas outside the document. Equal to `canvas` (no
    /// pattern) unless the theme sets it.
    pub canvas_pattern: Rgba,
    /// Panels, bars and rails.
    pub panel: Rgba,
    /// Window chrome: title bars, toolbars, option bars, rulers and scrollbar tracks; follows
    /// `panel` unless pinned.
    pub chrome: Rgba,
    /// Popups, menus and dialogs.
    pub raised: Rgba,
    /// Fields and wells.
    pub sunken: Rgba,
    /// The face of a knob that rides on a track: slider and switch thumbs. Light in both modes.
    pub knob: Rgba,
    /// Translucent cover behind modal surfaces.
    pub scrim: Rgba,
    /// The colour of every cast shadow (popups, menus, dialogs, floating windows, toasts,
    /// tooltips): black at the theme's alpha, as PhotoCraft's `Tokens::shadow`. Each use keeps its
    /// own offset and blur (`elevation.*`).
    pub shadow: Rgba,
}

/// Text and glyph colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextColors {
    /// Body text.
    pub normal: Rgba,
    /// Secondary text.
    pub muted: Rgba,
    /// Placeholders and hints; still held to the text minimum.
    pub faint: Rgba,
    /// Disabled text; exempt from contrast minimums.
    pub disabled: Rgba,
    /// Text on `accent.solid`, `accent.hover` and `accent.pressed`. The components'
    /// accent fills that carry text use `accent.strong` and `text.on_strong` instead.
    pub on_accent: Rgba,
    /// Text on `accent.strong`, `accent.strong_hover` and `accent.strong_pressed`: white. Every
    /// accent fill that carries text (primary buttons, solid badges, active menu rows) uses this
    /// pair.
    pub on_strong: Rgba,
    /// Text on every `status.<s>.solid`, `solid_hover` and `solid_pressed`.
    pub on_status: Rgba,
    /// Text on `state.selected` and `state.selected_hover`.
    pub on_selected: Rgba,
}

/// Lines and outlines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineColors {
    /// Decorative separators.
    pub divider: Rgba,
    /// Container borders.
    pub border: Rgba,
    /// The boundary of a control that must be seen to be used (fields, checkboxes).
    pub control_edge: Rgba,
    /// The outline of a `surface.knob` face; clear where the face stands out on its own (dark
    /// mode).
    pub knob_edge: Rgba,
    /// Marks that carry meaning and sit on control surfaces: slider marks and ticks, the ring
    /// around a badge dot. Holds the non-text minimum on them.
    pub mark: Rgba,
    /// Keyboard focus ring.
    pub focus: Rgba,
}

/// The accent family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccentColors {
    /// Solid fill: primary buttons, selected indicators.
    pub solid: Rgba,
    /// Solid fill under the pointer.
    pub hover: Rgba,
    /// Solid fill while pressed.
    pub pressed: Rgba,
    /// Solid fill under `text.on_strong`: `accent.solid` with its lightness moved away from that
    /// text only as far as the text needs to reach its minimum, and `accent.solid` itself where
    /// the text already does. Accent fills that carry text (primary buttons, solid badges, active
    /// menu rows) use the strong pair; fills without text (check marks, slider fills) use
    /// `accent.solid`.
    pub strong: Rgba,
    /// `accent.strong` under the pointer: its lightness moved away from `text.on_strong` (darker
    /// under white text) by the distance `accent.hover` lies from `accent.solid`, so it always
    /// differs from `accent.strong` and meets the text minimum wherever `accent.strong` does.
    pub strong_hover: Rgba,
    /// `accent.strong` while pressed: its lightness moved away from `text.on_strong` by the
    /// distance `accent.pressed` lies from `accent.solid`, further than `accent.strong_hover`.
    pub strong_pressed: Rgba,
    /// Tinted background.
    pub soft: Rgba,
    /// The outline of a selected or on control (a selected icon button, a toggle button that is
    /// on): `accent.solid` unless pinned. PhotoCraft's is translucent in several themes, so it
    /// reads on the control's own fill; high contrast draws it opaque.
    pub border: Rgba,
    /// Accent-coloured text and links.
    pub text: Rgba,
}

/// Colours of one status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatusColors {
    /// Text and icons.
    pub fg: Rgba,
    /// Tinted background.
    pub bg: Rgba,
    /// Border around a `bg` area.
    pub border: Rgba,
    /// Solid fill (badges, destructive buttons).
    pub solid: Rgba,
    /// Solid fill under the pointer.
    pub solid_hover: Rgba,
    /// Solid fill while pressed.
    pub solid_pressed: Rgba,
}

/// Neutral interaction fills, drawn over `surface.panel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateColors {
    /// Under the pointer.
    pub hover: Rgba,
    /// While pressed.
    pub pressed: Rgba,
    /// Selected items (neutral, as in pro apps).
    pub selected: Rgba,
    /// Selected and under the pointer.
    pub selected_hover: Rgba,
    /// Fill of disabled controls.
    pub disabled_fill: Rgba,
    /// Selected text and table cells: `accent.solid` at 27 %.
    pub text_selection: Rgba,
    /// Find results: `data.yellow` at 35 %.
    pub r#match: Rgba,
    /// The current find result: `status.warn.solid` at 43 %.
    pub match_current: Rgba,
}

/// Canvas overlays: selection paths, handles, guides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OverlayColors {
    /// Marks drawn over `halo` (the rotate cursor); handle outlines follow it.
    pub stroke: Rgba,
    /// Bare lines on the canvas: a transform box, path outlines, a pen draft, warp boundaries.
    /// The accent itself, with no halo and no contrast pair, as guides are.
    pub line: Rgba,
    /// The outline drawn under overlays so they read on any image.
    pub halo: Rgba,
    /// Inside of handles.
    pub handle_fill: Rgba,
    /// Outline of handles; follows `stroke` unless pinned.
    pub handle_stroke: Rgba,
    /// Marching ants and selection bands: the light colour.
    pub marquee: Rgba,
    /// Ruler guides, unless the host gives a guide its own colour.
    pub guide: Rgba,
    /// Canvas grid.
    pub grid: Rgba,
    /// The light cells of the transparency checkerboard.
    pub checker_light: Rgba,
    /// The dark cells of the transparency checkerboard.
    pub checker_dark: Rgba,
    /// The brush stroke being drawn on a selection mask, before it is committed: `accent.solid` at 43 %.
    pub trail: Rgba,
}

/// Chart and histogram colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DataColors {
    /// Eight categorical series.
    pub series: [Rgba; 8],
    /// Histogram red channel.
    pub red: Rgba,
    /// Histogram green channel.
    pub green: Rgba,
    /// Histogram blue channel.
    pub blue: Rgba,
    /// Histogram luminance.
    pub luma: Rgba,
    /// Curve and histogram cyan channel (CMYK Cyan).
    pub cyan: Rgba,
    /// Curve and histogram magenta channel (CMYK Magenta, Lab a).
    pub magenta: Rgba,
    /// Curve and histogram yellow channel (CMYK Yellow, Lab b).
    pub yellow: Rgba,
    /// The plot behind an RGB histogram: dark in every theme.
    pub scope: Rgba,
    /// The tone backdrop behind a curve graph's curve: `text.muted` at 50 %.
    pub backdrop: Rgba,
    /// The selected range's band on a histogram: `accent.solid` at 35 %.
    pub selection: Rgba,
}

/// Custom window caption (title bar) colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptionColors {
    /// Close button under the pointer.
    pub close_hover: Rgba,
    /// Close button while pressed; follows `close_hover` at 90 % unless pinned.
    pub close_pressed: Rgba,
    /// Close glyph on `close_hover` and `close_pressed`.
    pub close_text: Rgba,
    /// Resting disc of a round caption button (Adwaita); follows `text.normal` at 10 % unless pinned.
    pub disc: Rgba,
    /// Caption glyphs of an inactive window; follows `text.faint` unless pinned.
    pub inactive: Rgba,
    /// Pressed disc of an inverting caption button (Breeze); follows `text.muted` unless pinned.
    pub pressed: Rgba,
}

/// Bevel colours, used when `shape.edge` is [`Edge::Bevel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeColors {
    /// The lit edge (top and left of a raised surface).
    pub light: Rgba,
    /// The shaded edge (bottom and right of a raised surface).
    pub dark: Rgba,
}

/// Every semantic colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColorTokens {
    /// Backgrounds.
    pub surface: SurfaceColors,
    /// Text.
    pub text: TextColors,
    /// Lines.
    pub line: LineColors,
    /// Accent.
    pub accent: AccentColors,
    /// Statuses.
    pub status: Status<StatusColors>,
    /// Interaction fills.
    pub state: StateColors,
    /// Canvas overlays.
    pub overlay: OverlayColors,
    /// Charts and histograms.
    pub data: DataColors,
    /// Window caption.
    pub caption: CaptionColors,
    /// Bevel colours.
    pub edge: EdgeColors,
}

impl ColorTokens {
    /// Every colour set to `c`; the starting point that derivation overwrites.
    pub(super) const fn splat(c: Rgba) -> ColorTokens {
        let status = StatusColors { fg: c, bg: c, border: c, solid: c, solid_hover: c, solid_pressed: c };
        ColorTokens {
            surface: SurfaceColors {
                background: c,
                canvas: c,
                canvas_pattern: c,
                panel: c,
                chrome: c,
                raised: c,
                sunken: c,
                knob: c,
                scrim: c,
                shadow: c,
            },
            text: TextColors { normal: c, muted: c, faint: c, disabled: c, on_accent: c, on_strong: c, on_status: c, on_selected: c },
            line: LineColors { divider: c, border: c, control_edge: c, knob_edge: c, mark: c, focus: c },
            accent: AccentColors { solid: c, hover: c, pressed: c, strong: c, strong_hover: c, strong_pressed: c, soft: c, border: c, text: c },
            status: Status { good: status, warn: status, bad: status, info: status },
            state: StateColors {
                hover: c,
                pressed: c,
                selected: c,
                selected_hover: c,
                disabled_fill: c,
                text_selection: c,
                r#match: c,
                match_current: c,
            },
            overlay: OverlayColors {
                stroke: c,
                line: c,
                halo: c,
                handle_fill: c,
                handle_stroke: c,
                marquee: c,
                guide: c,
                grid: c,
                checker_light: c,
                checker_dark: c,
                trail: c,
            },
            data: DataColors {
                series: [c; 8],
                red: c,
                green: c,
                blue: c,
                luma: c,
                cyan: c,
                magenta: c,
                yellow: c,
                scope: c,
                backdrop: c,
                selection: c,
            },
            caption: CaptionColors { close_hover: c, close_pressed: c, close_text: c, disc: c, inactive: c, pressed: c },
            edge: EdgeColors { light: c, dark: c },
        }
    }
}

/// Defines [`ColorToken`] with its paths and the matching [`ColorTokens`] accessors from one
/// list, so the enum, the paths and the struct fields cannot drift apart.
macro_rules! color_tokens {
    ($( #[doc = $doc:literal] $var:ident = $path:literal => $($field:ident).+ $([$idx:literal])? ;)*) => {
        /// Names one semantic colour. The theme file addresses tokens by [`ColorToken::path`].
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum ColorToken {
            $( #[doc = $doc] $var, )*
        }

        impl ColorToken {
            /// Every colour token, in declaration order.
            pub const ALL: &'static [ColorToken] = &[$(ColorToken::$var),*];

            /// The dotted path, e.g. `"surface.panel"` or `"status.bad.fg"`.
            pub const fn path(self) -> &'static str {
                match self { $(ColorToken::$var => $path,)* }
            }

            /// The token whose [`path`](ColorToken::path) is `path`.
            pub fn from_path(path: &str) -> Option<ColorToken> {
                match path { $($path => Some(ColorToken::$var),)* _ => None }
            }
        }

        impl ColorTokens {
            /// The value of one token.
            pub fn get(&self, token: ColorToken) -> Rgba {
                match token { $(ColorToken::$var => self.$($field).+ $([$idx])?,)* }
            }

            /// Replaces the value of one token.
            pub fn set(&mut self, token: ColorToken, value: Rgba) {
                match token { $(ColorToken::$var => self.$($field).+ $([$idx])? = value,)* }
            }
        }
    };
}

color_tokens! {
    /// `surface.background`
    SurfaceBackground = "surface.background" => surface.background;
    /// `surface.canvas`
    SurfaceCanvas = "surface.canvas" => surface.canvas;
    /// `surface.canvas_pattern`
    SurfaceCanvasPattern = "surface.canvas_pattern" => surface.canvas_pattern;
    /// `surface.panel`
    SurfacePanel = "surface.panel" => surface.panel;
    /// `surface.chrome`
    SurfaceChrome = "surface.chrome" => surface.chrome;
    /// `surface.raised`
    SurfaceRaised = "surface.raised" => surface.raised;
    /// `surface.sunken`
    SurfaceSunken = "surface.sunken" => surface.sunken;
    /// `surface.knob`
    SurfaceKnob = "surface.knob" => surface.knob;
    /// `surface.scrim`
    SurfaceScrim = "surface.scrim" => surface.scrim;
    /// `surface.shadow`
    SurfaceShadow = "surface.shadow" => surface.shadow;
    /// `text.normal`
    TextNormal = "text.normal" => text.normal;
    /// `text.muted`
    TextMuted = "text.muted" => text.muted;
    /// `text.faint`
    TextFaint = "text.faint" => text.faint;
    /// `text.disabled`
    TextDisabled = "text.disabled" => text.disabled;
    /// `text.on_accent`
    TextOnAccent = "text.on_accent" => text.on_accent;
    /// `text.on_strong`
    TextOnStrong = "text.on_strong" => text.on_strong;
    /// `text.on_status`
    TextOnStatus = "text.on_status" => text.on_status;
    /// `text.on_selected`
    TextOnSelected = "text.on_selected" => text.on_selected;
    /// `line.divider`
    LineDivider = "line.divider" => line.divider;
    /// `line.border`
    LineBorder = "line.border" => line.border;
    /// `line.control_edge`
    LineControlEdge = "line.control_edge" => line.control_edge;
    /// `line.knob_edge`
    LineKnobEdge = "line.knob_edge" => line.knob_edge;
    /// `line.mark`
    LineMark = "line.mark" => line.mark;
    /// `line.focus`
    LineFocus = "line.focus" => line.focus;
    /// `accent.solid`
    AccentSolid = "accent.solid" => accent.solid;
    /// `accent.hover`
    AccentHover = "accent.hover" => accent.hover;
    /// `accent.pressed`
    AccentPressed = "accent.pressed" => accent.pressed;
    /// `accent.strong`
    AccentStrong = "accent.strong" => accent.strong;
    /// `accent.strong_hover`
    AccentStrongHover = "accent.strong_hover" => accent.strong_hover;
    /// `accent.strong_pressed`
    AccentStrongPressed = "accent.strong_pressed" => accent.strong_pressed;
    /// `accent.soft`
    AccentSoft = "accent.soft" => accent.soft;
    /// `accent.border`
    AccentBorder = "accent.border" => accent.border;
    /// `accent.text`
    AccentText = "accent.text" => accent.text;
    /// `status.good.fg`
    StatusGoodFg = "status.good.fg" => status.good.fg;
    /// `status.good.bg`
    StatusGoodBg = "status.good.bg" => status.good.bg;
    /// `status.good.border`
    StatusGoodBorder = "status.good.border" => status.good.border;
    /// `status.good.solid`
    StatusGoodSolid = "status.good.solid" => status.good.solid;
    /// `status.good.solid_hover`
    StatusGoodSolidHover = "status.good.solid_hover" => status.good.solid_hover;
    /// `status.good.solid_pressed`
    StatusGoodSolidPressed = "status.good.solid_pressed" => status.good.solid_pressed;
    /// `status.warn.fg`
    StatusWarnFg = "status.warn.fg" => status.warn.fg;
    /// `status.warn.bg`
    StatusWarnBg = "status.warn.bg" => status.warn.bg;
    /// `status.warn.border`
    StatusWarnBorder = "status.warn.border" => status.warn.border;
    /// `status.warn.solid`
    StatusWarnSolid = "status.warn.solid" => status.warn.solid;
    /// `status.warn.solid_hover`
    StatusWarnSolidHover = "status.warn.solid_hover" => status.warn.solid_hover;
    /// `status.warn.solid_pressed`
    StatusWarnSolidPressed = "status.warn.solid_pressed" => status.warn.solid_pressed;
    /// `status.bad.fg`
    StatusBadFg = "status.bad.fg" => status.bad.fg;
    /// `status.bad.bg`
    StatusBadBg = "status.bad.bg" => status.bad.bg;
    /// `status.bad.border`
    StatusBadBorder = "status.bad.border" => status.bad.border;
    /// `status.bad.solid`
    StatusBadSolid = "status.bad.solid" => status.bad.solid;
    /// `status.bad.solid_hover`
    StatusBadSolidHover = "status.bad.solid_hover" => status.bad.solid_hover;
    /// `status.bad.solid_pressed`
    StatusBadSolidPressed = "status.bad.solid_pressed" => status.bad.solid_pressed;
    /// `status.info.fg`
    StatusInfoFg = "status.info.fg" => status.info.fg;
    /// `status.info.bg`
    StatusInfoBg = "status.info.bg" => status.info.bg;
    /// `status.info.border`
    StatusInfoBorder = "status.info.border" => status.info.border;
    /// `status.info.solid`
    StatusInfoSolid = "status.info.solid" => status.info.solid;
    /// `status.info.solid_hover`
    StatusInfoSolidHover = "status.info.solid_hover" => status.info.solid_hover;
    /// `status.info.solid_pressed`
    StatusInfoSolidPressed = "status.info.solid_pressed" => status.info.solid_pressed;
    /// `state.hover`
    StateHover = "state.hover" => state.hover;
    /// `state.pressed`
    StatePressed = "state.pressed" => state.pressed;
    /// `state.selected`
    StateSelected = "state.selected" => state.selected;
    /// `state.selected_hover`
    StateSelectedHover = "state.selected_hover" => state.selected_hover;
    /// `state.disabled_fill`
    StateDisabledFill = "state.disabled_fill" => state.disabled_fill;
    /// `state.text_selection`
    StateTextSelection = "state.text_selection" => state.text_selection;
    /// `state.match`
    StateMatch = "state.match" => state.r#match;
    /// `state.match_current`
    StateMatchCurrent = "state.match_current" => state.match_current;
    /// `overlay.stroke`
    OverlayStroke = "overlay.stroke" => overlay.stroke;
    /// `overlay.line`
    OverlayLine = "overlay.line" => overlay.line;
    /// `overlay.halo`
    OverlayHalo = "overlay.halo" => overlay.halo;
    /// `overlay.handle_fill`
    OverlayHandleFill = "overlay.handle_fill" => overlay.handle_fill;
    /// `overlay.handle_stroke`
    OverlayHandleStroke = "overlay.handle_stroke" => overlay.handle_stroke;
    /// `overlay.marquee`
    OverlayMarquee = "overlay.marquee" => overlay.marquee;
    /// `overlay.guide`
    OverlayGuide = "overlay.guide" => overlay.guide;
    /// `overlay.grid`
    OverlayGrid = "overlay.grid" => overlay.grid;
    /// `overlay.checker_light`
    OverlayCheckerLight = "overlay.checker_light" => overlay.checker_light;
    /// `overlay.checker_dark`
    OverlayCheckerDark = "overlay.checker_dark" => overlay.checker_dark;
    /// `overlay.trail`
    OverlayTrail = "overlay.trail" => overlay.trail;
    /// `data.series.0`
    DataSeries0 = "data.series.0" => data.series[0];
    /// `data.series.1`
    DataSeries1 = "data.series.1" => data.series[1];
    /// `data.series.2`
    DataSeries2 = "data.series.2" => data.series[2];
    /// `data.series.3`
    DataSeries3 = "data.series.3" => data.series[3];
    /// `data.series.4`
    DataSeries4 = "data.series.4" => data.series[4];
    /// `data.series.5`
    DataSeries5 = "data.series.5" => data.series[5];
    /// `data.series.6`
    DataSeries6 = "data.series.6" => data.series[6];
    /// `data.series.7`
    DataSeries7 = "data.series.7" => data.series[7];
    /// `data.red`
    DataRed = "data.red" => data.red;
    /// `data.green`
    DataGreen = "data.green" => data.green;
    /// `data.blue`
    DataBlue = "data.blue" => data.blue;
    /// `data.luma`
    DataLuma = "data.luma" => data.luma;
    /// `data.cyan`
    DataCyan = "data.cyan" => data.cyan;
    /// `data.magenta`
    DataMagenta = "data.magenta" => data.magenta;
    /// `data.yellow`
    DataYellow = "data.yellow" => data.yellow;
    /// `data.scope`
    DataScope = "data.scope" => data.scope;
    /// `data.backdrop`
    DataBackdrop = "data.backdrop" => data.backdrop;
    /// `data.selection`
    DataSelection = "data.selection" => data.selection;
    /// `caption.close_hover`
    CaptionCloseHover = "caption.close_hover" => caption.close_hover;
    /// `caption.close_pressed`
    CaptionClosePressed = "caption.close_pressed" => caption.close_pressed;
    /// `caption.close_text`
    CaptionCloseText = "caption.close_text" => caption.close_text;
    /// `caption.disc`
    CaptionDisc = "caption.disc" => caption.disc;
    /// `caption.inactive`
    CaptionInactive = "caption.inactive" => caption.inactive;
    /// `caption.pressed`
    CaptionPressed = "caption.pressed" => caption.pressed;
    /// `edge.light`
    EdgeLight = "edge.light" => edge.light;
    /// `edge.dark`
    EdgeDark = "edge.dark" => edge.dark;
}

/// Names a colour of either layer: a semantic colour token or a component colour field. Theme
/// files address both by [`TokenRef::path`]; component paths never start with a semantic group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TokenRef {
    /// A semantic colour.
    Color(ColorToken),
    /// A component field.
    Component(ComponentField),
}

impl ColorToken {
    /// Whether this is a surface that text and fills sit on, or a knob that covers them
    /// (`surface.*` except `scrim` and `shadow`); those must be opaque.
    pub const fn is_opaque_surface(self) -> bool {
        use ColorToken::{
            SurfaceBackground, SurfaceCanvas, SurfaceCanvasPattern, SurfaceChrome, SurfaceKnob, SurfacePanel, SurfaceRaised, SurfaceSunken,
        };
        matches!(
            self,
            SurfaceBackground | SurfaceCanvas | SurfaceCanvasPattern | SurfaceChrome | SurfacePanel | SurfaceRaised | SurfaceSunken | SurfaceKnob
        )
    }
}

impl TokenRef {
    /// The dotted path, e.g. `"surface.panel"` or `"button.primary.fill.hover"`.
    pub fn path(self) -> String {
        match self {
            TokenRef::Color(t) => t.path().to_owned(),
            TokenRef::Component(f) => f.path(),
        }
    }

    /// The token or field whose [`path`](TokenRef::path) is `path`.
    pub fn from_path(path: &str) -> Option<TokenRef> {
        ColorToken::from_path(path).map(TokenRef::Color).or_else(|| ComponentField::from_path(path).map(TokenRef::Component))
    }
}

impl From<ColorToken> for TokenRef {
    fn from(t: ColorToken) -> TokenRef {
        TokenRef::Color(t)
    }
}

impl From<ComponentField> for TokenRef {
    fn from(f: ComponentField) -> TokenRef {
        TokenRef::Component(f)
    }
}

/// Corner radii in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadiusSet {
    /// Small parts: half of `control`, rounded; resolved after a theme's own shape applies, so it
    /// is not in a size set file.
    pub sm: f32,
    /// Buttons other than pills, fields, small controls.
    pub control: f32,
    /// Popups and menus.
    pub surface: f32,
    /// Windows and dialogs.
    pub window: f32,
    /// Docked panels.
    pub panel: f32,
    /// Push buttons; [`Radius::Full`](super::scale::Radius::Full) gives pills.
    pub button: f32,
    /// Pill-shaped parts (switch, slider, progress, badge); each clamps it to half its height.
    /// [`Radius::Full`](super::scale::Radius::Full) gives pills, 0 squares them.
    pub pill: f32,
}

/// How surfaces draw their edge. Only the `Surface` primitive reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Edge {
    /// No edge effect.
    #[default]
    Flat,
    /// A two-tone bevel: `light` on the lit sides, `dark` on the shaded ones.
    Bevel {
        /// The lit edge colour.
        light: Rgba,
        /// The shaded edge colour.
        dark: Rgba,
        /// Line width in logical pixels.
        width: f32,
    },
}

impl Edge {
    /// The seed this edge resolves from; a bevel drops its colours.
    pub fn seed(self) -> EdgeSeed {
        match self {
            Edge::Flat => EdgeSeed::Flat,
            Edge::Bevel { width, .. } => EdgeSeed::Bevel { width },
        }
    }
}

/// How list and tab rows are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowStyle {
    /// Square rows separated by dividers.
    #[default]
    Divided,
    /// Rounded rows without dividers.
    Rounded,
}

/// Corner radii, edge style and row style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeTokens {
    /// Corner radii.
    pub radius: RadiusSet,
    /// Edge style.
    pub edge: Edge,
    /// How list and tab rows are drawn.
    pub rows: RowStyle,
}

/// Gaps between neighbouring elements, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gaps {
    /// Between tightly packed items.
    pub tight: f32,
    /// Between items on one line.
    pub inline: f32,
    /// Between the rows of a panel body (PhotoCraft's `ROW_GAP`) and around a section header.
    pub row: f32,
    /// Between stacked rows.
    pub stack: f32,
    /// Between neighbouring controls in a bar, and between a bar caption and its control.
    pub item: f32,
    /// Between groups.
    pub section: f32,
}

/// Inner padding, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pads {
    /// Inside controls.
    pub control: f32,
    /// Inside panels and default dialog frames.
    pub regular: f32,
    /// Inside roomy dialogs.
    pub roomy: f32,
    /// Inside badges, key caps and chips: tighter than a control.
    pub chip: f32,
    /// Beside the label of a medium segment of a segmented control.
    pub segment: f32,
}

/// Spacing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpaceTokens {
    /// Gaps.
    pub gap: Gaps,
    /// Padding.
    pub pad: Pads,
    /// Insets from the edges of the screen.
    pub inset: Insets,
}

/// The gaps floating layers keep from the edges of the screen, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// The gap a floating layer (a dialog, a floating window) keeps from the screen edge.
    pub screen: f32,
}

/// Control heights, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlSizes {
    /// Small controls.
    pub sm: f32,
    /// Standard controls.
    pub md: f32,
}

/// Icon sizes, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IconSizes {
    /// Inline and small-control icons.
    pub sm: f32,
    /// Standard icons.
    pub md: f32,
    /// Rail and toolbar icons.
    pub lg: f32,
}

/// Tool-rail sizes, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RailSizes {
    /// Rail width.
    pub width: f32,
    /// Square button inside the rail.
    pub button: f32,
}

/// Bar heights, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BarSizes {
    /// Window title bar.
    pub title: f32,
    /// Tab strip.
    pub tab: f32,
    /// Status bar.
    pub status: f32,
    /// Tool options bar.
    pub options: f32,
    /// Tool options bar in its narrow form, for a window too small for the full bar.
    pub options_narrow: f32,
}

/// Fixed sizes of controls and chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SizeTokens {
    /// Control heights.
    pub control: ControlSizes,
    /// Icon sizes.
    pub icon: IconSizes,
    /// Tool rail.
    pub rail: RailSizes,
    /// Bars.
    pub bar: BarSizes,
    /// Panel header height.
    pub panel_header: f32,
    /// Thumbnail sides.
    pub thumb: ThumbSizes,
    /// Canvas drag handles.
    pub handle: HandleSizes,
    /// Pointer tolerance.
    pub hit: HitSizes,
    /// Side of a checkerboard cell in UI swatches and thumbnails.
    pub checker: f32,
    /// The side of a rotary control: the dial of an angle dial.
    pub dial: f32,
    /// The least size of a plot or two-dimensional value area: the well of a point picker, before the
    /// picker's own fitting.
    pub plot: PlotSizes,
    /// Lengths of controls that stretch along one axis.
    pub length: LengthSizes,
    /// The thickness of the groove of a value track (a slider, a range slider, a progress bar).
    pub groove: f32,
    /// The long side of the track of a two-state toggle (a switch).
    pub toggle: f32,
    /// Where a tip that follows the pointer (a gesture readout) sits from the pointer.
    pub pointer_tip: PointerTipOffset,
    /// The windows a dock floats its panels in.
    pub float: FloatSizes,
    /// The app's windows.
    pub window: WindowSizes,
    /// The panes a dock splits its area into.
    pub pane: PaneSizes,
    /// The docks' columns.
    pub dock: DockColumnSizes,
    /// The rulers of a canvas.
    pub ruler: RulerSizes,
    /// The controls drawn over a canvas.
    pub canvas: CanvasSizes,
}

/// The sizes of the controls drawn over a canvas, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasSizes {
    /// The side of the canvas compass, the control that shows and resets the view rotation.
    pub compass: f32,
}

/// The sizes of a canvas ruler, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RulerSizes {
    /// The tick marks.
    pub tick: RulerTickSizes,
}

/// The lengths of a ruler's tick marks, in logical pixels. The minor tick is `space.gap.tight` and the major tick
/// `space.gap.inline`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RulerTickSizes {
    /// The tick halfway between two labelled ticks.
    pub mid: f32,
}

/// The sizes of the panes of a dock's split, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaneSizes {
    /// The least extent of a pane along a split, and of a dock, before its panels' own limits.
    pub least: f32,
}

/// The sizes of a dock's columns, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DockColumnSizes {
    /// The width an expanded column has until the user resizes it.
    pub column: f32,
}

/// The sizes of a floating dock window, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloatSizes {
    /// The width a floating dock window opens at.
    pub width: f32,
    /// The height a floating dock window opens at.
    pub height: f32,
}

/// The sizes of the app's windows, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowSizes {
    /// The offset between cascaded windows (document windows and floating dock windows), along both axes.
    pub cascade: f32,
}

/// The offset of a pointer-following tip from the pointer, toward the reading end and down, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerTipOffset {
    /// Toward the reading end.
    pub x: f32,
    /// Down.
    pub y: f32,
}

/// Lengths of controls that stretch along one axis, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LengthSizes {
    /// The width of a text or value field.
    pub field: f32,
    /// The length of a value track (a slider, a range slider, a thumb row).
    pub track: f32,
    /// The width of a short field, such as a number beside a slider.
    pub short: f32,
}

/// Plot sizes, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlotSizes {
    /// Least width.
    pub width: f32,
    /// Least height.
    pub height: f32,
}

/// Canvas drag handle sizes, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandleSizes {
    /// Small handle: side or diameter.
    pub sm: f32,
    /// Standard handle: side or diameter.
    pub md: f32,
    /// The radius of the ring a pin's fixed rotation is set on.
    pub ring: f32,
    /// The hit radius of a transform box's handle.
    pub hit: f32,
    /// How far from a transform box's corner the rotate pointer shows outside the box.
    pub rotate_reach: f32,
    /// Where the rotate band outside a transform box's corner begins, from the corner.
    pub band_inner: f32,
    /// Where the rotate band outside a transform box's corner ends, from the corner.
    pub band_outer: f32,
}

/// Pointer tolerances, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitSizes {
    /// How far a press may move and still be a click.
    pub slop: f32,
    /// The least hit size of a part, in logical pixels; a thin handle or line is padded to it.
    pub target: f32,
    /// How far from the centre of a rotary control a press sets no value.
    pub dead_zone: f32,
    /// How far from the pointer the handles of an on-canvas object show.
    pub reveal: f32,
}

/// Line widths, dash lengths and gaps, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StrokeTokens {
    /// Hairline.
    pub thin: f32,
    /// Standard line.
    pub medium: f32,
    /// Heavy line.
    pub thick: f32,
    /// Length of a dash.
    pub dash: f32,
    /// Gap between dashes.
    pub dash_gap: f32,
}

/// Thumbnail sides, in logical pixels, for the three user settings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThumbSizes {
    /// Small.
    pub sm: f32,
    /// Medium.
    pub md: f32,
    /// Large.
    pub lg: f32,
}

/// The font a type role uses. `craft_ui::fonts` maps each slot to a face.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontSlot {
    /// The interface face, regular weight.
    #[default]
    Ui,
    /// The interface face, semibold.
    UiSemibold,
    /// The monospace face.
    Mono,
}

/// Size, line height and font of one type role.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TypeStyle {
    /// Font size in points.
    pub size: f32,
    /// Line height in points.
    pub line_height: f32,
    /// Font slot (family and weight).
    pub font: FontSlot,
}

/// Names one type role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeRole {
    /// Small print, badges.
    Caption,
    /// Control labels.
    Label,
    /// Body text.
    Body,
    /// Emphasised body text.
    BodyStrong,
    /// Panel and dialog titles.
    Title,
    /// Section headings.
    Heading,
    /// Large regular text: a prominent text field.
    BodyLarge,
    /// Code, numbers in tables.
    Mono,
}

/// Every type role.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TypeTokens {
    /// Small print, badges.
    pub caption: TypeStyle,
    /// Control labels.
    pub label: TypeStyle,
    /// Body text.
    pub body: TypeStyle,
    /// Emphasised body text.
    pub body_strong: TypeStyle,
    /// Panel and dialog titles.
    pub title: TypeStyle,
    /// Section headings.
    pub heading: TypeStyle,
    /// Large regular text: a prominent text field.
    pub body_large: TypeStyle,
    /// Code, numbers in tables.
    pub mono: TypeStyle,
}

impl TypeTokens {
    /// The style of `role`.
    pub fn get(&self, role: TypeRole) -> TypeStyle {
        match role {
            TypeRole::Caption => self.caption,
            TypeRole::Label => self.label,
            TypeRole::Body => self.body,
            TypeRole::BodyStrong => self.body_strong,
            TypeRole::Title => self.title,
            TypeRole::Heading => self.heading,
            TypeRole::BodyLarge => self.body_large,
            TypeRole::Mono => self.mono,
        }
    }

    /// Every size and line height multiplied by `factor` and rounded to `step` points.
    pub(super) fn scaled(&self, factor: f32, step: f32) -> TypeTokens {
        let round = |v: f32| (v * factor / step).round() * step;
        let s = |t: TypeStyle| TypeStyle { size: round(t.size), line_height: round(t.line_height), font: t.font };
        TypeTokens {
            caption: s(self.caption),
            label: s(self.label),
            body: s(self.body),
            body_strong: s(self.body_strong),
            title: s(self.title),
            heading: s(self.heading),
            body_large: s(self.body_large),
            mono: s(self.mono),
        }
    }
}

/// Shadows of floating surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElevationTokens {
    /// Popups, menus and dialogs (PhotoCraft's dialogs are popup frames).
    pub popup: Elevation,
    /// Floating windows, panels and toolbars: PhotoCraft's `window_shadow`.
    pub floating: Elevation,
    /// Knobs on a track (`surface.knob`); level 0 in bevel themes, whose bevel draws the relief.
    pub knob: Elevation,
}

/// One animation: how long and along which curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Motion {
    /// Duration in milliseconds; 0 with reduced motion.
    pub millis: u32,
    /// Easing curve.
    pub easing: Easing,
}

/// Animations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MotionTokens {
    /// Hover and press feedback.
    pub hover: Motion,
    /// Popups, menus and panels opening.
    pub open: Motion,
    /// Elements moving within the screen.
    pub r#move: Motion,
    /// A text caret gliding to its new place (`motion.caret`).
    pub caret: Motion,
    /// A selection outline gliding to its new cells (`motion.outline`).
    pub outline: Motion,
    /// The appearance asks for reduced motion: every duration above is zero.
    pub reduced: bool,
}

/// Every resolved token: the semantic layer and the component tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    /// Colours.
    pub color: ColorTokens,
    /// Corner radii and edge.
    pub shape: ShapeTokens,
    /// Gaps and padding.
    pub space: SpaceTokens,
    /// Control and chrome sizes.
    pub size: SizeTokens,
    /// Line widths and dashes.
    pub stroke: StrokeTokens,
    /// Type roles.
    pub typography: TypeTokens,
    /// Shadows.
    pub elevation: ElevationTokens,
    /// Animations.
    pub motion: MotionTokens,
    /// The component tokens.
    pub component: ComponentTokens,
}

impl Tokens {
    /// The value of one colour token.
    pub fn color(&self, token: ColorToken) -> Rgba {
        self.color.get(token)
    }
}
