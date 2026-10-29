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
    /// Marching ants and selection bands: the light colour, under `marquee_dash`.
    pub marquee: Rgba,
    /// The dashes over `marquee`, and the line of an outline being drawn.
    pub marquee_dash: Rgba,
    /// Ruler guides, unless the host gives a guide its own colour.
    pub guide: Rgba,
    /// The selected guide.
    pub guide_selected: Rgba,
    /// Smart guides and the distances they measure.
    pub smart_guide: Rgba,
    /// Canvas grid.
    pub grid: Rgba,
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
}

/// Custom window caption (title bar) colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptionColors {
    /// Close button under the pointer.
    pub close_hover: Rgba,
    /// Close glyph on `close_hover`.
    pub close_text: Rgba,
    /// Caption glyphs of an inactive window; follows `text.faint` unless pinned.
    pub inactive: Rgba,
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
            surface: SurfaceColors { background: c, canvas: c, panel: c, chrome: c, raised: c, sunken: c, knob: c, scrim: c },
            text: TextColors { normal: c, muted: c, faint: c, disabled: c, on_accent: c, on_strong: c, on_status: c, on_selected: c },
            line: LineColors { divider: c, border: c, control_edge: c, knob_edge: c, mark: c, focus: c },
            accent: AccentColors { solid: c, hover: c, pressed: c, strong: c, strong_hover: c, strong_pressed: c, soft: c, text: c },
            status: Status { good: status, warn: status, bad: status, info: status },
            state: StateColors { hover: c, pressed: c, selected: c, selected_hover: c, disabled_fill: c },
            overlay: OverlayColors {
                stroke: c,
                line: c,
                halo: c,
                handle_fill: c,
                handle_stroke: c,
                marquee: c,
                marquee_dash: c,
                guide: c,
                guide_selected: c,
                smart_guide: c,
                grid: c,
            },
            data: DataColors { series: [c; 8], red: c, green: c, blue: c, luma: c, cyan: c, magenta: c, yellow: c, scope: c },
            caption: CaptionColors { close_hover: c, close_text: c, inactive: c },
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
    /// `overlay.marquee_dash`
    OverlayMarqueeDash = "overlay.marquee_dash" => overlay.marquee_dash;
    /// `overlay.guide`
    OverlayGuide = "overlay.guide" => overlay.guide;
    /// `overlay.guide_selected`
    OverlayGuideSelected = "overlay.guide_selected" => overlay.guide_selected;
    /// `overlay.smart_guide`
    OverlaySmartGuide = "overlay.smart_guide" => overlay.smart_guide;
    /// `overlay.grid`
    OverlayGrid = "overlay.grid" => overlay.grid;
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
    /// `caption.close_hover`
    CaptionCloseHover = "caption.close_hover" => caption.close_hover;
    /// `caption.close_text`
    CaptionCloseText = "caption.close_text" => caption.close_text;
    /// `caption.inactive`
    CaptionInactive = "caption.inactive" => caption.inactive;
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
    /// (`surface.*` except `scrim`); those must be opaque.
    pub const fn is_opaque_surface(self) -> bool {
        use ColorToken::{SurfaceBackground, SurfaceCanvas, SurfaceChrome, SurfaceKnob, SurfacePanel, SurfaceRaised, SurfaceSunken};
        matches!(self, SurfaceBackground | SurfaceCanvas | SurfaceChrome | SurfacePanel | SurfaceRaised | SurfaceSunken | SurfaceKnob)
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusSet {
    /// Buttons other than pills, fields, small controls.
    pub control: f32,
    /// Popups and menus.
    pub popup: f32,
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edge {
    /// No edge effect.
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

/// Corner radii and edge style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeTokens {
    /// Corner radii.
    pub radius: RadiusSet,
    /// Edge style.
    pub edge: Edge,
}

/// Gaps between neighbouring elements, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaps {
    /// Between items on one line.
    pub inline: f32,
    /// Between the rows of a panel body (PhotoCraft's `ROW_GAP`) and around a section header.
    pub row: f32,
    /// Between stacked rows.
    pub stack: f32,
    /// Between groups.
    pub section: f32,
}

/// Inner padding, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pads {
    /// Inside controls.
    pub control: f32,
    /// Inside panels.
    pub panel: f32,
    /// Inside dialogs.
    pub dialog: f32,
}

/// Spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceTokens {
    /// Gaps.
    pub gap: Gaps,
    /// Padding.
    pub pad: Pads,
}

/// Control heights, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlSizes {
    /// Small controls.
    pub sm: f32,
    /// Standard controls.
    pub md: f32,
}

/// Icon sizes, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSizes {
    /// Inline and small-control icons.
    pub sm: f32,
    /// Standard icons.
    pub md: f32,
    /// Rail and toolbar icons.
    pub lg: f32,
}

/// Tool-rail sizes, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailSizes {
    /// Rail width.
    pub width: f32,
    /// Square button inside the rail.
    pub button: f32,
    /// Width of a panel rail: the iconic column of a dock and a panel rail.
    pub panel_width: f32,
    /// Square button inside a panel rail.
    pub panel_button: f32,
}

/// Bar heights, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarSizes {
    /// Window title bar.
    pub title: f32,
    /// Tab strip.
    pub tab: f32,
    /// Status bar.
    pub status: f32,
    /// Tool options bar.
    pub options: f32,
}

/// Fixed sizes of controls and chrome.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Lists and trees.
    pub list: ListSizes,
    /// Tabs.
    pub tab: TabSizes,
}

/// Thumbnail sides of a list row, in logical pixels, for the three user settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbSizes {
    /// Small.
    pub sm: f32,
    /// Medium.
    pub md: f32,
    /// Large.
    pub lg: f32,
}

/// Sizes and looks of list rows that differ by Size preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListSizes {
    /// Thumbnail sides.
    pub thumb: ThumbSizes,
    /// Least height of a row with a thumbnail.
    pub thumb_row_min: f32,
    /// Indent of one tree level.
    pub indent: f32,
    /// Width of the disclosure column.
    pub disclosure: f32,
    /// Width of one toggle column (eye, lock).
    pub toggle_column: f32,
    /// Least width of a row's label before optional trailing items drop out and the indent
    /// shrinks.
    pub min_label: f32,
    /// Room kept at a row's trailing edge for an overlay scroll bar.
    pub trailing_pad: f32,
    /// State fills have `radius.control` corners (flat otherwise).
    pub rounded: bool,
    /// Rows have a divider under them and toggle columns one at their trailing edge.
    pub dividers: bool,
}

/// Sizes and looks of tabs that differ by Size preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabSizes {
    /// Height of a dock group's tab strip (PhotoCraft's panel tabs are 2 taller than its
    /// document tabs).
    pub panel_height: f32,
    /// Height of a strip with Large Tabs.
    pub height_large: f32,
    /// Least width of a panel tab before the strip overflows.
    pub min_width: f32,
    /// Side of the × hit box.
    pub close: f32,
    /// Side of the × glyph.
    pub close_icon: f32,
    /// The active and hovered tab's top corners are `radius.control` (square otherwise).
    pub rounded: bool,
    /// Document tabs have a divider at their trailing edge.
    pub dividers: bool,
}

/// The font a type role uses. `craft_ui::fonts` maps each slot to a face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontSlot {
    /// The interface face, regular weight.
    Ui,
    /// The interface face, semibold.
    UiSemibold,
    /// The monospace face.
    Mono,
}

/// Size, line height and font of one type role.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Code, numbers in tables.
    Mono,
}

/// Every type role.
#[derive(Clone, Copy, Debug, PartialEq)]
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
            mono: s(self.mono),
        }
    }
}

/// Shadows of floating surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElevationTokens {
    /// Popups and menus.
    pub popup: Elevation,
    /// Dialogs.
    pub dialog: Elevation,
    /// Floating panels and toolbars.
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
}

/// Every resolved token: the semantic layer and the component layer.
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
    /// Type roles.
    pub typography: TypeTokens,
    /// Shadows.
    pub elevation: ElevationTokens,
    /// Animations.
    pub motion: MotionTokens,
    /// Per-family component tokens, derived from the fields above.
    pub component: ComponentTokens,
}

impl Tokens {
    /// The value of one colour token.
    pub fn color(&self, token: ColorToken) -> Rgba {
        self.color.get(token)
    }
}
