//! Component tokens: one struct per component family, every field concrete, filled from the
//! semantic tokens. A theme may override a curated set of fields ([`ComponentField`]): colours,
//! radii and shadows. Sizes, padding, gaps and type roles come from the Size preset only, and an
//! app may pin any of them ([`SizeField`]). Edges
//! are not among them: the `Surface` primitive draws every edge from `shape.edge`.
//!
//! A family is one file here plus one line in the `families!` list at the end of this file.

/// One size of a family, addressed by a path after the family name: how an app pins it. The
/// range is what [`AppSizes`](crate::AppSizes) accepts.
pub(crate) struct SizeLeaf<T> {
    pub path: &'static str,
    pub lo: f32,
    pub hi: f32,
    pub get: fn(&T) -> f32,
    pub set: fn(&mut T, f32),
}

/// A family's `SIZES`: one line per length, `"path" lo..=hi => place;`, where `place` is the
/// field (`height.md`, `pad[0]`) of the family's tokens.
macro_rules! sizes {
    ($tokens:ident; $( $path:literal $lo:literal..=$hi:literal => $first:ident $(. $rest:ident)* $([$index:literal])?; )*) => {
        &[ $( SizeLeaf::<$tokens> {
            path: $path,
            lo: $lo,
            hi: $hi,
            get: |t| t.$first $(. $rest)* $([$index])?,
            set: |t, v| t.$first $(. $rest)* $([$index])? = v,
        } ),* ]
    };
}

mod about;
mod actions;
mod add_beside;
mod adjust;
mod align;
mod alt_text;
mod angle_dial;
mod app_shell;
mod artboard;
mod autocomplete;
mod badge;
mod before_after;
mod bezier;
mod blend_if;
mod blur_pins;
mod breadcrumb;
mod brush;
mod button;
mod canvas;
mod channels;
mod check;
mod check_spelling;
mod chip;
mod color_picker;
mod color_ramp;
mod color_wheel;
mod command_palette;
mod connector;
mod constrain;
mod corner;
mod crop;
mod cursor;
mod curve;
mod date;
mod dialog;
mod dock;
mod drag;
mod drop_veil;
mod empty_state;
mod event_log;
mod export;
mod eyedropper;
mod field;
mod file_list;
mod filter_gallery;
mod findings;
mod float;
mod font_picker;
mod form;
mod full_window;
mod glyphs;
mod gradient_editor;
mod gradient_line;
mod gradient_palette;
mod grid;
mod grip;
mod group_box;
mod guide;
mod handle;
mod harmony;
mod histogram;
mod history;
mod hover;
mod icon_button;
mod icon_picker;
mod identify;
mod inspector;
mod kbd;
mod layer_style;
mod layers;
mod levels;
mod line_guide;
mod link;
mod liquify;
mod list;
mod list_manager;
mod listbox;
mod marquee;
mod measure;
mod menu;
mod metadata;
mod navigator;
mod new_document;
mod notice;
mod options_bar;
mod outline_draft;
mod paint_chip;
mod panel;
mod panel_footer;
mod panel_header;
mod password;
mod pasteboard;
mod popover;
mod popup;
mod preferences;
mod preset;
mod progress;
mod property;
mod rail;
mod rating;
mod readout;
mod ref_point;
mod reset_dot;
mod ruler;
mod sampler;
mod scopes;
mod screen_picker;
mod scroll;
mod section;
mod segmented;
mod select;
mod select_mask;
mod selection_bar;
mod selection_frame;
mod shape_drag;
mod slice;
mod slider;
mod slider_row;
mod spinner;
mod split;
mod split_button;
mod start;
mod status_bar;
mod stroke_editor;
mod swatch;
mod swatch_palette;
mod symmetry;
mod tab;
mod table;
mod text_editor;
mod text_styles;
mod theme_chip;
mod thumbnail;
mod tile;
mod title_bar;
mod toast;
mod toggle_button;
mod tool_pill;
mod toolbar;
mod tooltip;
mod tour;
mod transform;
mod tree_picker;
mod vanishing_point;
mod wide_angle;

use std::fmt;

use super::color::Rgba;
use super::scale::Elevation;
use super::sizes::ComponentSizes;
use identify::Identify;

use super::tokens::{ColorTokens, ElevationTokens, ShapeTokens, SizeTokens, SpaceTokens, TokenRef, TypeTokens};

pub use about::{AboutField, AboutTokens};
pub use actions::{ActionsField, ActionsTokens};
pub use add_beside::{AddBesideField, AddBesideTokens};
pub use adjust::{AdjustField, AdjustTokens};
pub use align::{AlignField, AlignTokens};
pub use alt_text::{AltTextField, AltTextTokens};
pub use angle_dial::{AngleDialField, AngleDialTokens};
pub use app_shell::{AppShellField, AppShellTokens};
pub use artboard::{ArtboardField, ArtboardTokens};
pub use autocomplete::{AutocompleteField, AutocompleteTokens};
pub use badge::{BadgeColors, BadgeField, BadgePart, BadgeStyle, BadgeTokens, BadgeTone};
pub use before_after::{BeforeAfterField, BeforeAfterTokens};
pub use bezier::{BezierField, BezierTokens};
pub use blend_if::{BlendIfField, BlendIfTokens};
pub use blur_pins::{BlurPinsField, BlurPinsTokens};
pub use breadcrumb::{BreadcrumbField, BreadcrumbTokens};
pub use brush::{BrushField, BrushTokens};
pub use button::{ButtonColors, ButtonField, ButtonPart, ButtonTokens, ButtonVariant};
pub use canvas::{CanvasField, CanvasTokens};
pub use channels::{ChannelsField, ChannelsTokens};
pub use check::{CheckField, CheckOffColors, CheckOffPart, CheckOnColors, CheckOnPart, CheckSwitchColors, CheckSwitchPart, CheckTokens};
pub use check_spelling::{CheckSpellingField, CheckSpellingTokens};
pub use chip::{ChipField, ChipTokens};
pub use color_picker::{ColorPickerField, ColorPickerTokens};
pub use color_ramp::{ColorRampField, ColorRampTokens};
pub use color_wheel::{ColorWheelField, ColorWheelTokens};
pub use command_palette::{CommandPaletteField, CommandPaletteTokens};
pub use connector::{ConnectorField, ConnectorTokens};
pub use constrain::{ConstrainField, ConstrainTokens};
pub use corner::{CornerField, CornerTokens};
pub use crop::{CropField, CropTokens};
pub use cursor::{CursorField, CursorTokens};
pub use curve::{CurveField, CurveTokens};
pub use date::{DateField, DateTokens};
pub use dialog::{DialogField, DialogTokens};
pub use dock::{DockField, DockTokens};
pub use drag::{DragField, DragTokens};
pub use drop_veil::{DropVeilField, DropVeilTokens};
pub use empty_state::{EmptyStateField, EmptyStateTokens};
pub use event_log::{EventLogField, EventLogTokens};
pub use export::{ExportField, ExportTokens};
pub use eyedropper::{EyedropperField, EyedropperTokens};
pub use field::{BorderColors, BorderState, Dimmed, FieldField, FieldPart, FieldTokens};
pub use file_list::{FileListField, FileListTokens};
pub use filter_gallery::{FilterGalleryField, FilterGalleryTokens};
pub use findings::{FindingsField, FindingsTokens};
pub use float::{FloatField, FloatTokens};
pub use font_picker::{FontPickerField, FontPickerTokens};
pub use form::{FormField, FormSpacing, FormTokens};
pub use full_window::{FullWindowField, FullWindowTokens};
pub use glyphs::{GlyphsField, GlyphsTokens};
pub use gradient_editor::{GradientEditorField, GradientEditorTokens};
pub use gradient_line::{GradientLineField, GradientLineTokens};
pub use gradient_palette::{GradientPaletteField, GradientPaletteTokens};
pub use grid::{GridField, GridTokens};
pub use grip::{GripField, GripTokens};
pub use group_box::{GroupBoxField, GroupBoxTokens};
pub use guide::{GuideField, GuideTokens};
pub use handle::{HandleField, HandleTokens};
pub use harmony::{HarmonyField, HarmonyTokens};
pub use histogram::{HistogramField, HistogramTokens};
pub use history::{HistoryField, HistoryTokens};
pub use hover::{HoverField, HoverTokens};
pub use icon_button::{IconButtonField, IconButtonTokens};
pub use icon_picker::{IconPickerField, IconPickerTokens};
pub use inspector::{InspectorField, InspectorTokens};
pub use kbd::{KbdField, KbdTokens};
pub use layer_style::{LayerStyleField, LayerStyleTokens};
pub use layers::{LayersField, LayersTokens};
pub use levels::{LevelsField, LevelsTokens};
pub use line_guide::{LineGuideField, LineGuideTokens};
pub use link::{LinkField, LinkTokens};
pub use liquify::{LiquifyField, LiquifyTokens};
pub use list::{LabelColor, LabelPart, LabelTint, ListField, ListTokens};
pub use list_manager::{ListManagerField, ListManagerTokens};
pub use listbox::{ListboxField, ListboxTokens};
pub use marquee::{MarqueeField, MarqueeTokens};
pub use measure::{MeasureField, MeasureTokens};
pub use menu::{MenuField, MenuHue, MenuTint, MenuTintPart, MenuTokens};
pub use metadata::{MetadataField, MetadataTokens};
pub use navigator::{NavigatorField, NavigatorTokens};
pub use new_document::{NewDocumentField, NewDocumentTokens};
pub use notice::{NoticeField, NoticeTokens, Tone};
pub use options_bar::{OptionsBarField, OptionsBarTokens};
pub use outline_draft::{OutlineDraftField, OutlineDraftTokens};
pub use paint_chip::{PaintChipField, PaintChipTokens};
pub use panel::{PanelField, PanelTokens};
pub use panel_footer::{PanelFooterField, PanelFooterTokens};
pub use panel_header::{PanelHeaderField, PanelHeaderTokens};
pub use password::{PasswordField, PasswordTokens};
pub use pasteboard::{PasteboardField, PasteboardTokens};
pub use popover::{PopoverField, PopoverTokens};
pub use popup::{PopupField, PopupTokens};
pub use preferences::{PreferencesField, PreferencesTokens};
pub use preset::{PresetField, PresetTokens};
pub use progress::{ProgressField, ProgressTokens};
pub use property::{PropertyField, PropertyTokens};
pub use rail::{RailField, RailTokens};
pub use rating::{RatingField, RatingTokens};
pub use readout::{ReadoutField, ReadoutTokens};
pub use ref_point::{RefPointField, RefPointTokens};
pub use reset_dot::{ResetDotField, ResetDotTokens};
pub use ruler::{RulerField, RulerTokens};
pub use sampler::{SamplerField, SamplerTokens};
pub use scopes::{ScopesField, ScopesTokens};
pub use screen_picker::{ScreenPickerField, ScreenPickerTokens};
pub use scroll::{ScrollField, ScrollTokens};
pub use section::{SectionField, SectionTokens};
pub use segmented::{SegmentedField, SegmentedTokens};
pub use select::{SelectField, SelectTokens};
pub use select_mask::{SelectMaskField, SelectMaskTokens};
pub use selection_bar::{SelectionBarField, SelectionBarTokens};
pub use selection_frame::{SelectionFrameField, SelectionFrameTokens};
pub use shape_drag::{ShapeDragField, ShapeDragTokens};
pub use slice::{SliceField, SliceTokens};
pub use slider::{SliderField, SliderTokens};
pub use slider_row::{SliderRowField, SliderRowTokens};
pub use spinner::{SpinnerField, SpinnerTokens};
pub use split::{SplitField, SplitTokens};
pub use split_button::{SplitButtonField, SplitButtonTokens};
pub use start::{StartField, StartTokens};
pub use status_bar::{StatusBarField, StatusBarTokens};
pub use stroke_editor::{StrokeEditorField, StrokeEditorTokens};
pub use swatch::{SwatchColor, SwatchField, SwatchTokens};
pub use swatch_palette::{SwatchPaletteField, SwatchPaletteTokens};
pub use symmetry::{SymmetryField, SymmetryTokens};
pub use tab::{TabField, TabTokens};
pub use table::{TableField, TableTokens};
pub use text_editor::{TextEditorField, TextEditorTokens};
pub use text_styles::{TextStylesField, TextStylesTokens};
pub use theme_chip::{ThemeChipField, ThemeChipTokens};
pub use thumbnail::{ThumbnailField, ThumbnailTokens};
pub use tile::{TileField, TileTokens};
pub use title_bar::{TitleBarField, TitleBarTokens};
pub use toast::{ToastField, ToastTokens};
pub use toggle_button::{ToggleButtonField, ToggleButtonTokens};
pub use tool_pill::{ToolPillField, ToolPillTokens};
pub use toolbar::{ToolbarField, ToolbarTokens};
pub use tooltip::{TooltipField, TooltipTokens};
pub use tour::{TourField, TourTokens};
pub use transform::{TransformField, TransformTokens};
pub use tree_picker::{TreePickerField, TreePickerTokens};
pub use vanishing_point::{VanishingPointField, VanishingPointTokens};
pub use wide_angle::{WideAngleField, WideAngleTokens};

/// The semantic tokens that component defaults derive from. It holds no component tokens, so no
/// family's defaults can depend on another's. Fields are added as families need them.
pub(crate) struct Semantic<'a> {
    pub color: &'a ColorTokens,
    pub shape: &'a ShapeTokens,
    pub space: &'a SpaceTokens,
    pub size: &'a SizeTokens,
    /// Shadows.
    pub elevation: &'a ElevationTokens,
    /// The size set's component values.
    pub set: &'a ComponentSizes,
    /// After the Text size setting.
    pub typography: &'a TypeTokens,
    /// High contrast: the derived colours of a family aim for the high minimums.
    pub high: bool,
}

/// The two control sizes a family offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlSize {
    /// Small: toolbars, dense rows.
    Sm,
    /// Standard.
    Md,
}

/// One value per [`ControlSize`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BySize<T> {
    /// Small.
    pub sm: T,
    /// Standard.
    pub md: T,
}

impl<T: Copy> BySize<T> {
    /// The value for `size`.
    pub fn get(&self, size: ControlSize) -> T {
        match size {
            ControlSize::Sm => self.sm,
            ControlSize::Md => self.md,
        }
    }
}

/// An interaction state. Keyboard focus is not one: the `FocusRing` primitive draws it from
/// `line.focus`. Selected and checked belong to the families that have them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum State {
    /// Idle.
    Rest,
    /// Under the pointer.
    Hover,
    /// While pressed.
    Pressed,
    /// Not interactive.
    Disabled,
}

impl State {
    /// Every state, in path order.
    pub const ALL: [State; 4] = [State::Rest, State::Hover, State::Pressed, State::Disabled];

    /// The path segment, e.g. `"hover"`.
    pub const fn name(self) -> &'static str {
        match self {
            State::Rest => "rest",
            State::Hover => "hover",
            State::Pressed => "pressed",
            State::Disabled => "disabled",
        }
    }

    /// The state whose [`name`](State::name) is `name`.
    pub fn from_name(name: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// One value per [`State`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct States<T> {
    /// Idle.
    pub rest: T,
    /// Under the pointer.
    pub hover: T,
    /// While pressed.
    pub pressed: T,
    /// Not interactive.
    pub disabled: T,
}

impl<T: Copy> States<T> {
    /// The four values, in [`State::ALL`] order.
    pub const fn new(rest: T, hover: T, pressed: T, disabled: T) -> States<T> {
        States { rest, hover, pressed, disabled }
    }

    /// The value for `state`.
    pub fn get(&self, state: State) -> T {
        match state {
            State::Rest => self.rest,
            State::Hover => self.hover,
            State::Pressed => self.pressed,
            State::Disabled => self.disabled,
        }
    }

    /// The value for `state`, to change it.
    pub fn get_mut(&mut self, state: State) -> &mut T {
        match state {
            State::Rest => &mut self.rest,
            State::Hover => &mut self.hover,
            State::Pressed => &mut self.pressed,
            State::Disabled => &mut self.disabled,
        }
    }
}

/// One coloured part of an item without an outline: a segment, a toggled icon button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemPart {
    /// The background.
    Fill,
    /// Label and icon.
    Text,
}

impl ItemPart {
    /// Every part, in path order.
    pub const ALL: [ItemPart; 2] = [ItemPart::Fill, ItemPart::Text];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ItemPart::Fill => "fill",
            ItemPart::Text => "text",
        }
    }
}

/// The colours of an item without an outline, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemColors {
    /// Background.
    pub fill: States<Rgba>,
    /// Label and icon.
    pub text: States<Rgba>,
}

impl ItemColors {
    /// The colours of `part`.
    pub fn part(&self, part: ItemPart) -> &States<Rgba> {
        match part {
            ItemPart::Fill => &self.fill,
            ItemPart::Text => &self.text,
        }
    }

    fn part_mut(&mut self, part: ItemPart) -> &mut States<Rgba> {
        match part {
            ItemPart::Fill => &mut self.fill,
            ItemPart::Text => &mut self.text,
        }
    }

    /// The colour of `part` in `state`, to set it.
    fn get_mut(&mut self, part: ItemPart, state: State) -> &mut Rgba {
        self.part_mut(part).get_mut(state)
    }
}

/// The path of an item colour after its set's name: `fill.rest`.
fn item_path(part: ItemPart, state: State) -> String {
    format!("{}.{}", part.name(), state.name())
}

/// The part and state of an item colour path after its set's name.
fn item_from_path(path: &str) -> Option<(ItemPart, State)> {
    let (part, state) = path.split_once('.')?;
    Some((ItemPart::ALL.into_iter().find(|p| p.name() == part)?, State::from_name(state)?))
}

/// Every (part, state) of an item colour set, in path order, for a family's `FIELDS`.
const ITEM_FIELDS: [(ItemPart, State); 8] = {
    let mut out = [(ItemPart::Fill, State::Rest); 8];
    let mut i = 0;
    while i < 8 {
        out[i] = (ItemPart::ALL[i / 4], State::ALL[i % 4]);
        i += 1;
    }
    out
};

/// One coloured part of a knob that rides on a track: a slider or switch thumb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThumbPart {
    /// The face.
    Fill,
    /// The outline; a bevel keeps its own colours.
    Border,
}

impl ThumbPart {
    /// Every part, in path order.
    pub const ALL: [ThumbPart; 2] = [ThumbPart::Fill, ThumbPart::Border];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ThumbPart::Fill => "fill",
            ThumbPart::Border => "border",
        }
    }
}

/// The colours of a knob that rides on a track, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThumbColors {
    /// Face.
    pub fill: States<Rgba>,
    /// Outline.
    pub border: States<Rgba>,
}

impl ThumbColors {
    /// The colours of `part`.
    pub fn part(&self, part: ThumbPart) -> &States<Rgba> {
        match part {
            ThumbPart::Fill => &self.fill,
            ThumbPart::Border => &self.border,
        }
    }

    fn part_mut(&mut self, part: ThumbPart) -> &mut States<Rgba> {
        match part {
            ThumbPart::Fill => &mut self.fill,
            ThumbPart::Border => &mut self.border,
        }
    }

    /// The knob look: a `surface.knob` face with a `line.knob_edge` outline in every enabled
    /// state. Disabled, it recedes toward the panel in both modes: its face is
    /// [`KNOB_DISABLED`](crate::values::KNOB_DISABLED) of the way from `surface.knob` to
    /// `surface.panel`, with a `line.divider` outline.
    fn knob(c: &ColorTokens) -> ThumbColors {
        let (face, edge) = (c.surface.knob, c.line.knob_edge);
        let disabled = crate::derive::mix(face, c.surface.panel, crate::values::KNOB_DISABLED);
        ThumbColors { fill: States::new(face, face, face, disabled), border: States::new(edge, edge, edge, c.line.divider) }
    }
}

/// The path of a thumb colour after its set's name: `fill.rest`.
fn thumb_path(part: ThumbPart, state: State) -> String {
    format!("{}.{}", part.name(), state.name())
}

/// The part and state of a thumb colour path after its set's name.
fn thumb_from_path(path: &str) -> Option<(ThumbPart, State)> {
    let (part, state) = path.split_once('.')?;
    Some((ThumbPart::ALL.into_iter().find(|p| p.name() == part)?, State::from_name(state)?))
}

/// Every (part, state) of a thumb colour set, in path order, for a family's `FIELDS`.
const THUMB_FIELDS: [(ThumbPart, State); 8] = {
    let mut out = [(ThumbPart::Fill, State::Rest); 8];
    let mut i = 0;
    while i < 8 {
        out[i] = (ThumbPart::ALL[i / 4], State::ALL[i % 4]);
        i += 1;
    }
    out
};

/// What kind of value an overridable field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// A colour, per mode.
    Color,
    /// A length in logical pixels.
    Length,
    /// A shadow.
    Elevation,
}

/// A value for an overridable field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverrideValue {
    /// A colour.
    Color(Rgba),
    /// A length in logical pixels; a radius of 999 gives pills, as
    /// [`Radius::Full`](super::scale::Radius::Full).
    Length(f32),
    /// A shadow.
    Elevation(Elevation),
}

impl OverrideValue {
    /// The kind of this value.
    pub const fn kind(&self) -> FieldKind {
        match self {
            OverrideValue::Color(_) => FieldKind::Color,
            OverrideValue::Length(_) => FieldKind::Length,
            OverrideValue::Elevation(_) => FieldKind::Elevation,
        }
    }
}

/// An override that can't be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverrideError {
    /// The value's kind is not the field's.
    WrongKind {
        /// The field.
        field: ComponentField,
        /// The field's kind.
        expected: FieldKind,
        /// The value's kind.
        found: FieldKind,
    },
    /// A colour among the mode-independent overrides; component colours go in the mode pins.
    ColorNotPerMode(ComponentField),
    /// A pin on a field that is not a colour.
    PinNotColor(ComponentField),
}

impl fmt::Display for OverrideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverrideError::WrongKind { field, expected, found } => write!(f, "{}: expected {expected:?}, found {found:?}", field.path()),
            OverrideError::ColorNotPerMode(field) => write!(f, "{}: colours are set per mode", field.path()),
            OverrideError::PinNotColor(field) => write!(f, "{}: not a colour", field.path()),
        }
    }
}

impl std::error::Error for OverrideError {}

/// A contrast requirement between a colour a component draws and what it is drawn on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ContrastPair {
    /// The foreground: a component field, or a semantic colour the component draws on a
    /// component background (a curve's `data.*` channel colours on `curve.fill`).
    pub fg: TokenRef,
    /// Its background; a translucent one is measured over `surface.panel`.
    pub bg: TokenRef,
    /// Minimum ratio with standard and with high contrast.
    pub minimum: (f64, f64),
    /// Another foreground that identifies the same thing: the pair passes when `fg` or this
    /// reaches the minimum on `bg` (a box's outline or its fill). The measured ratio is the
    /// higher of the two, and a contrast pass that has to fix the pair moves `fg`.
    pub alternate: Option<TokenRef>,
}

/// What a family file provides. The `families!` list turns these into [`ComponentTokens`] and
/// [`ComponentField`].
trait Family: Sized + 'static {
    /// The family's overridable fields.
    type Field: Copy + 'static;
    /// Every overridable field, in path order.
    const FIELDS: &'static [Self::Field];
    /// The defaults, from semantic tokens only.
    fn defaults(s: &Semantic<'_>) -> Self;
    /// The path of `f` after the family name.
    fn path(f: Self::Field) -> String;
    /// The field whose [`path`](Family::path) is `path`.
    fn from_path(path: &str) -> Option<Self::Field>;
    /// The kind of value `f` takes.
    fn kind(f: Self::Field) -> FieldKind;
    /// The value of `f`.
    fn get(&self, f: Self::Field) -> OverrideValue;
    /// Sets `f` to `v`, whose kind the caller has checked.
    fn set(&mut self, f: Self::Field, v: OverrideValue);
    /// The family's contrast pairs.
    fn pairs() -> Vec<ContrastPair>;
    /// Every size of the family that an app may pin, in path order. The paths are separate from
    /// [`FIELDS`](Family::FIELDS): a theme file never sets them.
    const SIZES: &'static [SizeLeaf<Self>] = &[];
}

/// Defines [`ComponentTokens`] and [`ComponentField`] from one list of families. The struct field
/// name is the family's path prefix.
macro_rules! families {
    ($( #[doc = $doc:literal] $name:ident: $tokens:ident => $var:ident($field:ident); )+) => {
        /// Every component family's tokens.
        #[derive(Clone, Debug, PartialEq)]
        pub struct ComponentTokens {
            $( #[doc = $doc] pub $name: $tokens, )+
        }

        /// Names one overridable field of one family. Theme files address fields by
        /// [`ComponentField::path`], e.g. `"button.primary.fill.hover"`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum ComponentField {
            $( #[doc = $doc] $var($field), )+
        }

        impl ComponentField {
            /// Every overridable field, family by family. Fields not listed here can't be
            /// overridden.
            pub const ALL: &'static [ComponentField] = &{
                const COUNT: usize = 0 $(+ <$tokens as Family>::FIELDS.len())+;
                // The first field of the first family that has one fills the array until the
                // loops overwrite every slot. Families may have no field (`HandleField`).
                #[allow(clippy::panic, reason = "evaluated at compile time: the list fails to build")]
                const FILL: ComponentField = {
                    let mut fill = None;
                    $(if fill.is_none() && !<$tokens as Family>::FIELDS.is_empty() {
                        fill = Some(ComponentField::$var(<$tokens as Family>::FIELDS[0]));
                    })+
                    match fill {
                        Some(f) => f,
                        None => panic!("no component family has an overridable field"),
                    }
                };
                let mut out = [FILL; COUNT];
                let mut i = 0;
                $({
                    let fields = <$tokens as Family>::FIELDS;
                    let mut j = 0;
                    while j < fields.len() {
                        out[i] = ComponentField::$var(fields[j]);
                        i += 1;
                        j += 1;
                    }
                })+
                out
            };

            /// The dotted path, e.g. `"button.radius"`.
            pub fn path(self) -> String {
                match self { $(ComponentField::$var(f) => format!("{}.{}", stringify!($name), <$tokens as Family>::path(f)),)+ }
            }

            /// The field whose [`path`](ComponentField::path) is `path`; `None` for unknown and
            /// non-overridable paths.
            pub fn from_path(path: &str) -> Option<ComponentField> {
                let (family, rest) = path.split_once('.')?;
                match family {
                    $(stringify!($name) => <$tokens as Family>::from_path(rest).map(ComponentField::$var),)+
                    _ => None,
                }
            }

            /// The kind of value the field takes.
            pub fn kind(self) -> FieldKind {
                match self { $(ComponentField::$var(f) => <$tokens as Family>::kind(f),)+ }
            }
        }

        /// Names one size of one family, such as `"toolbar.button"`: the lengths an app pins.
        /// Theme files don't accept these paths.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct SizeField(SizeRef);

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        enum SizeRef {
            $( $var(usize), )+
        }

        impl SizeField {
            /// Every size, family by family.
            pub fn all() -> Vec<SizeField> {
                let mut out = Vec::new();
                $( out.extend((0..<$tokens as Family>::SIZES.len()).map(|i| SizeField(SizeRef::$var(i)))); )+
                out
            }

            /// The dotted path, e.g. `"toolbar.button"`.
            pub fn path(self) -> String {
                match self.0 { $(SizeRef::$var(i) => format!("{}.{}", stringify!($name), <$tokens as Family>::SIZES[i].path),)+ }
            }

            /// The size whose [`path`](SizeField::path) is `path`.
            pub fn from_path(path: &str) -> Option<SizeField> {
                let (family, rest) = path.split_once('.')?;
                match family {
                    $(stringify!($name) => <$tokens as Family>::SIZES.iter().position(|l| l.path == rest).map(|i| SizeField(SizeRef::$var(i))),)+
                    _ => None,
                }
            }

            /// The kind of value a size takes.
            pub const fn kind(self) -> FieldKind {
                FieldKind::Length
            }

            /// The least and greatest value an app may pin.
            pub fn range(self) -> (f32, f32) {
                match self.0 { $(SizeRef::$var(i) => { let l = &<$tokens as Family>::SIZES[i]; (l.lo, l.hi) },)+ }
            }

            /// The size in `tokens`.
            pub fn get(self, tokens: &ComponentTokens) -> f32 {
                match self.0 { $(SizeRef::$var(i) => (<$tokens as Family>::SIZES[i].get)(&tokens.$name),)+ }
            }

            /// Sets the size in `tokens`.
            pub fn set(self, tokens: &mut ComponentTokens, value: f32) {
                match self.0 { $(SizeRef::$var(i) => (<$tokens as Family>::SIZES[i].set)(&mut tokens.$name, value),)+ }
            }
        }

        impl ComponentTokens {
            /// Every family's defaults.
            pub(crate) fn defaults(s: &Semantic<'_>) -> ComponentTokens {
                ComponentTokens { $($name: <$tokens as Family>::defaults(s),)+ }
            }

            /// The value of one field.
            pub fn get(&self, field: ComponentField) -> OverrideValue {
                match field { $(ComponentField::$var(f) => self.$name.get(f),)+ }
            }

            /// Sets one field.
            pub fn set(&mut self, field: ComponentField, value: OverrideValue) -> Result<(), OverrideError> {
                let (expected, found) = (field.kind(), value.kind());
                if expected != found {
                    return Err(OverrideError::WrongKind { field, expected, found });
                }
                match field { $(ComponentField::$var(f) => self.$name.set(f, value),)+ }
                Ok(())
            }

            /// Every family's contrast pairs.
            pub(crate) fn pairs() -> Vec<ContrastPair> {
                let mut out = Vec::new();
                $(out.extend(<$tokens as Family>::pairs());)+
                out
            }
        }
    };
}

impl ComponentTokens {
    /// The colour of `field`, or `None` if it is not a colour field.
    pub fn color(&self, field: ComponentField) -> Option<Rgba> {
        match self.get(field) {
            OverrideValue::Color(c) => Some(c),
            _ => None,
        }
    }
}

families! {
    /// About windows: the window width, the Contributors list height and the grab bag.
    about: AboutTokens => About(AboutField);
    /// Password prompts: the width of the dialog.
    password: PasswordTokens => Password(PasswordField);
    /// List managers: the list box, the buttons and the gap between them.
    list_manager: ListManagerTokens => ListManager(ListManagerField);
    /// Align bars: the icon buttons.
    align: AlignTokens => Align(AlignField);
    /// Tree pickers: the popup's list.
    tree_picker: TreePickerTokens => TreePicker(TreePickerField);
    /// Alternative text editors: the box.
    alt_text: AltTextTokens => AltText(AltTextField);
    /// Tables: the header, the sort chevron, the column edges and the drop line.
    table: TableTokens => Table(TableField);
    /// Chips: pills with a remove cross.
    chip: ChipTokens => Chip(ChipField);
    /// Icon pickers: the cells of the grid.
    icon_picker: IconPickerTokens => IconPicker(IconPickerField);
    /// Split buttons: the arrow part and its divider.
    split_button: SplitButtonTokens => SplitButton(SplitButtonField);
    /// Breadcrumb bars: the height, chevrons and segment room.
    breadcrumb: BreadcrumbTokens => Breadcrumb(BreadcrumbField);
    /// Ratings: the star colour and the star and cell sizes.
    rating: RatingTokens => Rating(RatingField);
    /// Date pickers: the day grid and its popup.
    date: DateTokens => Date(DateField);
    /// Reset dots: the square, dot and hover corner.
    reset_dot: ResetDotTokens => ResetDot(ResetDotField);
    /// Drag badges: the offset from the pointer and the padding.
    drag: DragTokens => Drag(DragField);
    /// Drop veils: the frame and card of the file-drop overlay.
    drop_veil: DropVeilTokens => DropVeil(DropVeilField);
    /// Event logs: the level glyphs and the details box.
    event_log: EventLogTokens => EventLog(EventLogField);
    /// Autocomplete lists: the least width and the gap to the editor.
    autocomplete: AutocompleteTokens => Autocomplete(AutocompleteField);
    /// Metadata tables: the column and row gaps and the most height.
    metadata: MetadataTokens => Metadata(MetadataField);
    /// Export As and Save for Web: the widths of the dialogs and their settings columns.
    export: ExportTokens => Export(ExportField);
    /// File list pickers: the list box and the width of its buttons.
    file_list: FileListTokens => FileList(FileListField);
    /// Findings lists: the status dot of the summary line.
    findings: FindingsTokens => Findings(FindingsField);
    /// The New Document dialog: the card grid, the page icon and the Saved list.
    new_document: NewDocumentTokens => NewDocument(NewDocumentField);
    /// Theme chips: the small preview of a theme in the Appearance page.
    theme_chip: ThemeChipTokens => ThemeChip(ThemeChipField);
    /// Picture thumbnails: the checkerboard, the empty cell and the failure icon.
    thumbnail: ThumbnailTokens => Thumbnail(ThumbnailField);
    /// Media tiles: the grid, the compare tile and the media bin's label chip.
    tile: TileTokens => Tile(TileField);
    /// Link Media: the status badge, links table, candidate list and preview.
    link: LinkTokens => Link(LinkField);
    /// The Navigator: the preview box, the proxy rectangle and the zoom row's gap.
    navigator: NavigatorTokens => Navigator(NavigatorField);
    /// The scope panel: warning buttons, statistics, All Channels plots and the vectorscope.
    scopes: ScopesTokens => Scopes(ScopesField);
    /// The Actions panel: row heights, the recording dot's radius and the Button Mode fills.
    actions: ActionsTokens => Actions(ActionsField);
    /// Angle dials: the round face and needle that point in a direction.
    angle_dial: AngleDialTokens => AngleDial(AngleDialField);
    /// The app shell: the backdrop behind the document and the edge reveal zone.
    app_shell: AppShellTokens => AppShell(AppShellField);
    /// Badges: small labels in a tone.
    badge: BadgeTokens => Badge(BadgeField);
    /// Paths: outlines, anchors, direction handles and the path being drawn.
    bezier: BezierTokens => Bezier(BezierField);
    /// Push buttons.
    button: ButtonTokens => Button(ButtonField);
    /// The canvas: the Rotate View compass.
    canvas: CanvasTokens => Canvas(CanvasField);
    /// Check boxes, radio buttons and switches.
    check: CheckTokens => Check(CheckField);
    /// The Channels panel: the thumbnail side and the spot channel's ink swatch.
    channels: ChannelsTokens => Channels(ChannelsField);
    /// Colour pickers: the sizes of the area, strip, compare swatch and marker.
    color_picker: ColorPickerTokens => ColorPicker(ColorPickerField);
    /// Colour ramps: the bar under the slider rows, with its white and black boxes.
    color_ramp: ColorRampTokens => ColorRamp(ColorRampField);
    /// Colour wheels and discs: the hue ring and triangle, and the harmony disc.
    color_wheel: ColorWheelTokens => ColorWheel(ColorWheelField);
    /// Command palettes: the modal search box over the app's commands.
    command_palette: CommandPaletteTokens => CommandPalette(CommandPaletteField);
    /// Curve editors: the graph, points and gradient bars of a tone curve.
    curve: CurveTokens => Curve(CurveField);
    /// The crop frame and its shield.
    crop: CropTokens => Crop(CropField);
    /// Painted pointers: the crosshair, selection badge, distort and skew arrows.
    cursor: CursorTokens => Cursor(CursorField);
    /// Dialogs: the frame every dialog shares.
    dialog: DialogTokens => Dialog(DialogField);
    /// Dock areas: the panel groups, column bars and iconic columns of a docking surface.
    dock: DockTokens => Dock(DockField);
    /// Empty states: the message of an empty panel, list or canvas.
    empty_state: EmptyStateTokens => EmptyState(EmptyStateField);
    /// The Eyedropper: the sampling ring round the pointer.
    eyedropper: EyedropperTokens => Eyedropper(EyedropperField);
    /// Floating windows: the frame and title bar of a floating panel group.
    float: FloatTokens => Float(FloatField);
    /// Form grids: the gaps and label column of a two-column form.
    form: FormTokens => Form(FormField);
    /// Text fields: one line of editable text in a sunken box.
    field: FieldTokens => Field(FieldField);
    /// Check Spelling dialog: the sizes of its fields and suggestion list.
    check_spelling: CheckSpellingTokens => CheckSpelling(CheckSpellingField);
    /// The font picker: the open list and its class column.
    font_picker: FontPickerTokens => FontPicker(FontPickerField);
    /// The Character Styles and Paragraph Styles panels: their list.
    text_styles: TextStylesTokens => TextStyles(TextStylesField);
    /// The Glyphs panel: the character grid and the Recently Used strip.
    glyphs: GlyphsTokens => Glyphs(GlyphsField);
    /// Gradient editors: the stop bar, its markers and midpoint diamonds.
    gradient_editor: GradientEditorTokens => GradientEditor(GradientEditorField);
    /// Gradient lines: the line, end handles, stops and midpoints of a gradient on the canvas.
    gradient_line: GradientLineTokens => GradientLine(GradientLineField);
    /// Canvas grid lines.
    grid: GridTokens => Grid(GridField);
    /// Group boxes: the outlined, titled group of a dialog or preferences page.
    group_box: GroupBoxTokens => GroupBox(GroupBoxField);
    /// Gradient palettes: the grid and list of gradient presets.
    gradient_palette: GradientPaletteTokens => GradientPalette(GradientPaletteField);
    /// Ruler guides and smart guides.
    guide: GuideTokens => Guide(GuideField);
    /// Canvas handles.
    handle: HandleTokens => Handle(HandleField);
    /// Harmony panels: the Color Guide's base button, strip and variation grid.
    harmony: HarmonyTokens => Harmony(HarmonyField);
    /// Histograms: the plot height and the fill and backdrop alphas.
    histogram: HistogramTokens => Histogram(HistogramField);
    /// The History panel: the snapshot thumbnail and the Undo list's width.
    history: HistoryTokens => History(HistoryField);
    /// Square icon-only buttons.
    icon_button: IconButtonTokens => IconButton(IconButtonField);
    /// Inspectors: the body of a Properties panel and its target header.
    inspector: InspectorTokens => Inspector(InspectorField);
    /// Key chords and keycaps.
    kbd: KbdTokens => Kbd(KbdField);
    /// The Layers panel: indicators, mask badges, blend rows and the filter bar.
    layers: LayersTokens => Layers(LayersField);
    /// Levels editors: the triangle thumbs and output bar under a histogram.
    levels: LevelsTokens => Levels(LevelsField);
    /// Lists: the rows of a list or tree panel.
    list: ListTokens => List(ListField);
    /// List boxes: the rows of popup lists.
    listbox: ListboxTokens => Listbox(ListboxField);
    /// Marching ants, selection bands and outlines being drawn.
    marquee: MarqueeTokens => Marquee(MarqueeField);
    /// Menus: what a menu adds to its list box rows.
    menu: MenuTokens => Menu(MenuField);
    /// Notices: inline status messages.
    notice: NoticeTokens => Notice(NoticeField);
    /// Options bars: the bar of the active tool's options.
    options_bar: OptionsBarTokens => OptionsBar(OptionsBarField);
    /// Paint chips: the shape fill and stroke chip.
    paint_chip: PaintChipTokens => PaintChip(PaintChipField);
    /// Panels: the body of a docked or floating panel, with an optional footer bar.
    panel: PanelTokens => Panel(PanelField);
    /// Panel footers: the bar of buttons under a panel body.
    panel_footer: PanelFooterTokens => PanelFooter(PanelFooterField);
    /// Panel headers: the title row of a flyout or floating panel.
    panel_header: PanelHeaderTokens => PanelHeader(PanelHeaderField);
    /// The pasteboard: the page name's gap and the page outline.
    pasteboard: PasteboardTokens => Pasteboard(PasteboardField);
    /// Popovers: the padding, resize grip and screen margin of an anchored popup.
    popover: PopoverTokens => Popover(PopoverField);
    /// Popups: the card that holds a menu or a list under a field.
    popup: PopupTokens => Popup(PopupField);
    /// Preferences windows: the page list column and the gap to the page.
    preferences: PreferencesTokens => Preferences(PreferencesField);
    /// Preset browsers: the grid, headers and captions of a library of thumbnails.
    preset: PresetTokens => Preset(PresetField);
    /// Progress bars.
    progress: ProgressTokens => Progress(ProgressField);
    /// Property rows: the label, control and column widths of an inspector row.
    property: PropertyTokens => Property(PropertyField);
    /// Rails: the tool rail, the panel rail and the flyout of a rail button.
    rail: RailTokens => Rail(RailField);
    /// Readouts: the values beside the pointer during a canvas gesture.
    readout: ReadoutTokens => Readout(ReadoutField);
    /// Reference point pickers: the nine points of a box.
    ref_point: RefPointTokens => RefPoint(RefPointField);
    /// Rulers along the top and left of a canvas.
    ruler: RulerTokens => Ruler(RulerField);
    /// Color Sampler points: the reticle drawn at each numbered point.
    sampler: SamplerTokens => Sampler(SamplerField);
    /// The screen colour picker: the loupe over the captured screen.
    screen_picker: ScreenPickerTokens => ScreenPicker(ScreenPickerField);
    /// Scroll bars: the thin bars of panel scroll areas and the bars of a canvas.
    scroll: ScrollTokens => Scroll(ScrollField);
    /// Section headers: the row, chevron and title of a collapsible section.
    section: SectionTokens => Section(SectionField);
    /// Segmented controls: one of a few options as separate pills.
    segmented: SegmentedTokens => Segmented(SegmentedField);
    /// Selection tools: the lasso close reach, the Magnetic Lasso fastening point and closing ring.
    select: SelectTokens => Select(SelectField);
    /// Select and Mask's canvas marks: the polygon close reach, the Object Selection click reach and the stroke colour.
    select_mask: SelectMaskTokens => SelectMask(SelectMaskField);
    /// Hover outlines: the object under the pointer.
    hover: HoverTokens => Hover(HoverField);
    /// Outlines being drawn: the close reach and the same-point reach.
    outline_draft: OutlineDraftTokens => OutlineDraft(OutlineDraftField);
    /// Selection frames: the box, key object line, centre mark and dashed outline.
    selection_frame: SelectionFrameTokens => SelectionFrame(SelectionFrameField);
    /// Grips: the squares and midpoint bars of an object's selection.
    grip: GripTokens => Grip(GripField);
    /// Constrained moves: the copy reach and the stroke lock.
    constrain: ConstrainTokens => Constrain(ConstrainField);
    /// Line guides: the line a straight-line stroke previews.
    line_guide: LineGuideTokens => LineGuide(LineGuideField);
    /// Before and after views: the divider line, knob and gaps.
    before_after: BeforeAfterTokens => BeforeAfter(BeforeAfterField);
    /// Brushes: the cursor, resize gesture, preset cards, picker, chip, settings panel and tip ellipse.
    brush: BrushTokens => Brush(BrushField);
    /// Full-window editors: the side column, canvas region, zoom bar and progress bar.
    full_window: FullWindowTokens => FullWindow(FullWindowField);
    /// Corner widgets: the round marks at a rectangle's corners.
    corner: CornerTokens => Corner(CornerField);
    /// Stroke editors: the popover that edits a stroke, its opener and samples.
    stroke_editor: StrokeEditorTokens => StrokeEditor(StrokeEditorField);
    /// The Ruler tool: the reach of its ends and the arms of their crosses.
    measure: MeasureTokens => Measure(MeasureField);
    /// Symmetry guides: the axis lines and the Transform Symmetry handles' reach.
    symmetry: SymmetryTokens => Symmetry(SymmetryField);
    /// The adjustment editors: Hue/Saturation's spectrum bars and range thumbs, the point pickers' tone dot and Threshold's histogram.
    adjust: AdjustTokens => Adjust(AdjustField);
    /// The Layer Style dialog: the effect list and page columns, the Styles page's thumbnails and the contour field.
    layer_style: LayerStyleTokens => LayerStyle(LayerStyleField);
    /// The Blend If bar: its width range, ramp and handle triangles.
    blend_if: BlendIfTokens => BlendIf(BlendIfField);
    /// The "+" buttons beside a selected object.
    add_beside: AddBesideTokens => AddBeside(AddBesideField);
    /// The Shape tools' drag: the preview outline and the click slop.
    shape_drag: ShapeDragTokens => ShapeDrag(ShapeDragField);
    /// The Slice tool's overlay: outlines, dashes, badges and the click slop.
    slice: SliceTokens => Slice(SliceField);
    /// The Artboard tool: the drag preview and the click slop.
    artboard: ArtboardTokens => Artboard(ArtboardField);
    /// The Connector tool: the route, attach points, handles and the ghost.
    connector: ConnectorTokens => Connector(ConnectorField);
    /// Vanishing Point's marks: plane outlines, grid, nodes and the stamp ring's halo.
    vanishing_point: VanishingPointTokens => VanishingPoint(VanishingPointField);
    /// Adaptive Wide Angle's constraint marks.
    wide_angle: WideAngleTokens => WideAngle(WideAngleField);
    /// The blur pins' marks: dots, the amount ring, shape handles and grab reach.
    blur_pins: BlurPinsTokens => BlurPins(BlurPinsField);
    /// Liquify's marks on the canvas: the Freeze Lasso and the freeze mask.
    liquify: LiquifyTokens => Liquify(LiquifyField);
    /// The Filter Gallery's thumbnail and stack sizes.
    filter_gallery: FilterGalleryTokens => FilterGallery(FilterGalleryField);
    /// Selection bars: the floating bar of actions beside a selection.
    selection_bar: SelectionBarTokens => SelectionBar(SelectionBarField);
    /// Sliders: a track filled to the value, with a thumb on it.
    slider: SliderTokens => Slider(SliderField);
    /// Slider rows: a caption and number field over a full-width slider.
    slider_row: SliderRowTokens => SliderRow(SliderRowField);
    /// Spinners: the ring of an indeterminate wait.
    spinner: SpinnerTokens => Spinner(SpinnerField);
    /// Split views: the gap between panes.
    split: SplitTokens => Split(SplitField);
    /// Start screens: the recent-file card grid.
    start: StartTokens => Start(StartField);
    /// Status bars: the bar at the bottom of the window.
    status_bar: StatusBarTokens => StatusBar(StatusBarField);
    /// Colour wells.
    swatch: SwatchTokens => Swatch(SwatchField);
    /// Swatch palettes: the cell of the table mode.
    swatch_palette: SwatchPaletteTokens => SwatchPalette(SwatchPaletteField);
    /// Tabs: one tab of a document or panel strip.
    tab: TabTokens => Tab(TabField);
    /// The text editor on a canvas: the caret and the marked-text underlines.
    text_editor: TextEditorTokens => TextEditor(TextEditorField);
    /// Title bars: the caption row with the window's buttons and the document title.
    title_bar: TitleBarTokens => TitleBar(TitleBarField);
    /// Toasts: cards stacked in a corner of the window.
    toast: ToastTokens => Toast(ToastField);
    /// Toggle buttons that are on.
    toggle_button: ToggleButtonTokens => ToggleButton(ToggleButtonField);
    /// Tool pills: the floating row of tools over a canvas.
    tool_pill: ToolPillTokens => ToolPill(ToolPillField);
    /// Toolbars: a row or column of icon-button groups.
    toolbar: ToolbarTokens => Toolbar(ToolbarField);
    /// Tooltips.
    tooltip: TooltipTokens => Tooltip(TooltipField);
    /// Tours: the coach marks and the card of a guided tour.
    tour: TourTokens => Tour(TourField);
    /// Transform boxes and warp meshes.
    transform: TransformTokens => Transform(TransformField);
}
