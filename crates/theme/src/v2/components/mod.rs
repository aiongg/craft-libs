//! Component tokens: one struct per component family, every field concrete, filled from the
//! semantic tokens. A theme may override a curated set of fields ([`ComponentField`]): colours,
//! radii, edges and shadows. Sizes, padding, gaps and type roles come from the Size preset only.
//!
//! A family is one file here plus one line in the `families!` list at the end of this file.

mod button;

use std::fmt;

use super::color::Rgba;
use super::scale::Elevation;
use super::theme::EdgeSeed;
use super::tokens::{ColorTokens, ShapeTokens, SizeTokens, SpaceTokens, TokenRef};

pub use button::{ButtonColors, ButtonField, ButtonPart, ButtonTokens, ButtonVariant};

/// The semantic tokens that component defaults derive from. It holds no component tokens, so no
/// family's defaults can depend on another's. Fields are added as families need them.
pub(in crate::v2) struct Semantic<'a> {
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

/// What kind of value an overridable field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// A colour, per mode.
    Color,
    /// A length in logical pixels.
    Length,
    /// An edge style.
    Edge,
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
    /// An edge style; a bevel takes its colours from `edge.light` and `edge.dark`.
    Edge(EdgeSeed),
    /// A shadow.
    Elevation(Elevation),
}

impl OverrideValue {
    /// The kind of this value.
    pub const fn kind(&self) -> FieldKind {
        match self {
            OverrideValue::Color(_) => FieldKind::Color,
            OverrideValue::Length(_) => FieldKind::Length,
            OverrideValue::Edge(_) => FieldKind::Edge,
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
pub(in crate::v2) struct ContrastPair {
    /// The foreground field.
    pub fg: ComponentField,
    /// Its background; a translucent one is measured over `surface.panel`.
    pub bg: TokenRef,
    /// Minimum ratio with standard and with high contrast.
    pub minimum: (f64, f64),
    /// Whether a failing pair is reported; the others are fixed with high contrast only.
    pub required: bool,
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
    fn set(&mut self, f: Self::Field, v: OverrideValue, color: &ColorTokens);
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
                // Any field fills the array until the loops overwrite every slot.
                let mut out = [[$(ComponentField::$var(<$tokens as Family>::FIELDS[0])),+][0]; COUNT];
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
            pub(in crate::v2) fn defaults(s: &Semantic<'_>) -> ComponentTokens {
                ComponentTokens { $($name: <$tokens as Family>::defaults(s),)+ }
            }

            /// The value of one field.
            pub fn get(&self, field: ComponentField) -> OverrideValue {
                match field { $(ComponentField::$var(f) => self.$name.get(f),)+ }
            }

            /// Sets one field. `color` supplies a bevel's colours (`edge.light`, `edge.dark`).
            pub fn set(&mut self, field: ComponentField, value: OverrideValue, color: &ColorTokens) -> Result<(), OverrideError> {
                let (expected, found) = (field.kind(), value.kind());
                if expected != found {
                    return Err(OverrideError::WrongKind { field, expected, found });
                }
                match field { $(ComponentField::$var(f) => self.$name.set(f, value, color),)+ }
                Ok(())
            }

            /// Every family's contrast pairs.
            pub(in crate::v2) fn pairs() -> Vec<ContrastPair> {
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
}
