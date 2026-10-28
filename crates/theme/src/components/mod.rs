//! Component tokens: one struct per component family, every field concrete, filled from the
//! semantic tokens. A theme may override a curated set of fields ([`ComponentField`]): colours,
//! radii and shadows. Sizes, padding, gaps and type roles come from the Size preset only. Edges
//! are not among them: the `Surface` primitive draws every edge from `shape.edge`.
//!
//! A family is one file here plus one line in the `families!` list at the end of this file.

mod button;
mod handle;
mod icon_button;
mod segmented;

use std::fmt;

use super::color::Rgba;
use super::scale::Elevation;
use super::tokens::{ColorTokens, ShapeTokens, SizeTokens, SpaceTokens, TokenRef};

pub use button::{ButtonColors, ButtonField, ButtonPart, ButtonTokens, ButtonVariant};
pub use handle::{HandleField, HandleTokens};
pub use icon_button::{IconButtonField, IconButtonTokens};
pub use segmented::{SegmentedField, SegmentedTokens};

/// The semantic tokens that component defaults derive from. It holds no component tokens, so no
/// family's defaults can depend on another's. Fields are added as families need them.
pub(crate) struct Semantic<'a> {
    pub color: &'a ColorTokens,
    pub shape: &'a ShapeTokens,
    pub space: &'a SpaceTokens,
    pub size: &'a SizeTokens,
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

    /// The `state.selected` look: `text.on_selected` on `state.selected`, the disabled fill and
    /// text when disabled.
    fn selected(c: &ColorTokens) -> ItemColors {
        let (fill, text) = (c.state.selected, c.text.on_selected);
        ItemColors { fill: States::new(fill, fill, fill, c.state.disabled_fill), text: States::new(text, text, text, c.text.disabled) }
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

/// A contrast requirement between a component colour and what it is drawn on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ContrastPair {
    /// The foreground field.
    pub fg: ComponentField,
    /// Its background; a translucent one is measured over `surface.panel`.
    pub bg: TokenRef,
    /// Minimum ratio with standard and with high contrast.
    pub minimum: (f64, f64),
}

/// What a family file provides. The `families!` list turns these into [`ComponentTokens`] and
/// [`ComponentField`].
trait Family: Sized {
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
    /// Push buttons.
    button: ButtonTokens => Button(ButtonField);
    /// Canvas handles.
    handle: HandleTokens => Handle(HandleField);
    /// Square icon-only buttons.
    icon_button: IconButtonTokens => IconButton(IconButtonField);
    /// Segmented controls: one of a few options on a track.
    segmented: SegmentedTokens => Segmented(SegmentedField);
}
