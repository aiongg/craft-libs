//! Push buttons: four variants, two sizes. Loading shows a spinner in the text colour and needs
//! no tokens of its own.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States};
use crate::v2::color::Rgba;
use crate::v2::tokens::{ColorToken, ColorTokens, Edge, TokenRef, TypeRole};
use crate::v2::values;

/// A button's emphasis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonVariant {
    /// The main action: accent fill.
    Primary,
    /// Other actions: neutral fill and border.
    Secondary,
    /// Toolbar and inline actions: no fill until hovered.
    Ghost,
    /// Destructive actions: `status.bad` fill.
    Danger,
}

impl ButtonVariant {
    /// Every variant, in path order.
    pub const ALL: [ButtonVariant; 4] = [ButtonVariant::Primary, ButtonVariant::Secondary, ButtonVariant::Ghost, ButtonVariant::Danger];

    /// The path segment, e.g. `"primary"`.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Danger => "danger",
        }
    }
}

/// One coloured part of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonPart {
    /// The background.
    Fill,
    /// The outline.
    Border,
    /// Label, icon and spinner.
    Text,
}

impl ButtonPart {
    /// Every part, in path order.
    pub const ALL: [ButtonPart; 3] = [ButtonPart::Fill, ButtonPart::Border, ButtonPart::Text];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonPart::Fill => "fill",
            ButtonPart::Border => "border",
            ButtonPart::Text => "text",
        }
    }
}

/// The colours of one variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonColors {
    /// Background.
    pub fill: States<Rgba>,
    /// Outline, `border_width` wide.
    pub border: States<Rgba>,
    /// Label, icon and spinner.
    pub text: States<Rgba>,
}

impl ButtonColors {
    /// The colours of `part`.
    pub fn part(&self, part: ButtonPart) -> &States<Rgba> {
        match part {
            ButtonPart::Fill => &self.fill,
            ButtonPart::Border => &self.border,
            ButtonPart::Text => &self.text,
        }
    }

    fn part_mut(&mut self, part: ButtonPart) -> &mut States<Rgba> {
        match part {
            ButtonPart::Fill => &mut self.fill,
            ButtonPart::Border => &mut self.border,
            ButtonPart::Text => &mut self.text,
        }
    }
}

/// Push-button tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonTokens {
    /// Height.
    pub height: BySize<f32>,
    /// Horizontal padding at the start and end.
    pub pad_x: BySize<f32>,
    /// Icon size.
    pub icon: BySize<f32>,
    /// Between icon and label.
    pub gap: f32,
    /// Label type role.
    pub label: TypeRole,
    /// Corner radius; 999 gives pills.
    pub radius: f32,
    /// Outline width in logical pixels.
    pub border_width: f32,
    /// Edge style.
    pub edge: Edge,
    /// Primary colours.
    pub primary: ButtonColors,
    /// Secondary colours.
    pub secondary: ButtonColors,
    /// Ghost colours.
    pub ghost: ButtonColors,
    /// Danger colours.
    pub danger: ButtonColors,
}

impl ButtonTokens {
    /// The colours of `variant`.
    pub fn colors(&self, variant: ButtonVariant) -> &ButtonColors {
        match variant {
            ButtonVariant::Primary => &self.primary,
            ButtonVariant::Secondary => &self.secondary,
            ButtonVariant::Ghost => &self.ghost,
            ButtonVariant::Danger => &self.danger,
        }
    }

    fn colors_mut(&mut self, variant: ButtonVariant) -> &mut ButtonColors {
        match variant {
            ButtonVariant::Primary => &mut self.primary,
            ButtonVariant::Secondary => &mut self.secondary,
            ButtonVariant::Ghost => &mut self.ghost,
            ButtonVariant::Danger => &mut self.danger,
        }
    }
}

/// An overridable button field. Paths: `radius`, `edge`, `<variant>.<part>.<state>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonField {
    /// `button.radius`
    Radius,
    /// `button.edge`
    Edge,
    /// `button.<variant>.<part>.<state>`, e.g. `button.primary.fill.hover`.
    Color {
        /// The variant.
        variant: ButtonVariant,
        /// The part.
        part: ButtonPart,
        /// The state.
        state: State,
    },
}

const FIELD_COUNT: usize = 2 + ButtonVariant::ALL.len() * ButtonPart::ALL.len() * State::ALL.len();

const FIELDS: [ButtonField; FIELD_COUNT] = {
    let mut out = [ButtonField::Radius; FIELD_COUNT];
    out[1] = ButtonField::Edge;
    let mut i = 2;
    let (mut v, mut p, mut s) = (0, 0, 0);
    while v < ButtonVariant::ALL.len() {
        while p < ButtonPart::ALL.len() {
            while s < State::ALL.len() {
                out[i] = ButtonField::Color { variant: ButtonVariant::ALL[v], part: ButtonPart::ALL[p], state: State::ALL[s] };
                i += 1;
                s += 1;
            }
            s = 0;
            p += 1;
        }
        p = 0;
        v += 1;
    }
    out
};

impl Family for ButtonTokens {
    type Field = ButtonField;
    const FIELDS: &'static [ButtonField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> ButtonTokens {
        let c = s.color;
        let (size, space) = (s.size, s.space);
        let clear = Rgba::TRANSPARENT;
        let none = States::new(clear, clear, clear, clear);
        let text = |t: Rgba| States::new(t, t, t, c.text.disabled);
        let bad = c.status.bad;
        ButtonTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: BySize { sm: space.pad.control + space.gap.inline, md: values::BUTTON_PAD_X_MD * space.pad.control },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            gap: space.gap.inline,
            label: TypeRole::Label,
            radius: s.shape.radius.button,
            border_width: values::BUTTON_BORDER_WIDTH,
            edge: s.shape.edge,
            primary: ButtonColors {
                fill: States::new(c.accent.solid, c.accent.hover, c.accent.pressed, c.state.disabled_fill),
                border: none,
                text: text(c.text.on_accent),
            },
            secondary: ButtonColors {
                fill: States::new(c.state.hover, c.state.pressed, c.state.selected, c.state.disabled_fill),
                border: States::new(c.line.border, c.line.control_edge, c.line.control_edge, c.line.divider),
                text: text(c.text.normal),
            },
            ghost: ButtonColors { fill: States::new(clear, c.state.hover, c.state.pressed, clear), border: none, text: text(c.text.normal) },
            danger: ButtonColors {
                fill: States::new(bad.solid, bad.solid_hover, bad.solid_pressed, c.state.disabled_fill),
                border: none,
                text: text(c.text.on_status),
            },
        }
    }

    fn path(f: ButtonField) -> String {
        match f {
            ButtonField::Radius => "radius".to_owned(),
            ButtonField::Edge => "edge".to_owned(),
            ButtonField::Color { variant, part, state } => format!("{}.{}.{}", variant.name(), part.name(), state.name()),
        }
    }

    fn from_path(path: &str) -> Option<ButtonField> {
        match path {
            "radius" => return Some(ButtonField::Radius),
            "edge" => return Some(ButtonField::Edge),
            _ => {}
        }
        let mut segments = path.split('.');
        let (variant, part, state) = (segments.next()?, segments.next()?, segments.next()?);
        if segments.next().is_some() {
            return None;
        }
        Some(ButtonField::Color {
            variant: ButtonVariant::ALL.into_iter().find(|v| v.name() == variant)?,
            part: ButtonPart::ALL.into_iter().find(|p| p.name() == part)?,
            state: State::from_name(state)?,
        })
    }

    fn kind(f: ButtonField) -> FieldKind {
        match f {
            ButtonField::Radius => FieldKind::Length,
            ButtonField::Edge => FieldKind::Edge,
            ButtonField::Color { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: ButtonField) -> OverrideValue {
        match f {
            ButtonField::Radius => OverrideValue::Length(self.radius),
            ButtonField::Edge => OverrideValue::Edge(self.edge.seed()),
            ButtonField::Color { variant, part, state } => OverrideValue::Color(self.colors(variant).part(part).get(state)),
        }
    }

    fn set(&mut self, f: ButtonField, v: OverrideValue, color: &ColorTokens) {
        match (f, v) {
            (ButtonField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ButtonField::Edge, OverrideValue::Edge(e)) => self.edge = e.resolve(color),
            (ButtonField::Color { variant, part, state }, OverrideValue::Color(c)) => *self.colors_mut(variant).part_mut(part).get_mut(state) = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let field = |variant, part, state| ComponentField::Button(ButtonField::Color { variant, part, state });
        let mut out = Vec::new();
        for variant in ButtonVariant::ALL {
            for state in [State::Rest, State::Hover, State::Pressed] {
                let fill = TokenRef::Component(field(variant, ButtonPart::Fill, state));
                out.push(ContrastPair { fg: field(variant, ButtonPart::Text, state), bg: fill, minimum: values::TEXT_MIN, required: true });
            }
        }
        out.push(ContrastPair {
            fg: field(ButtonVariant::Secondary, ButtonPart::Border, State::Rest),
            bg: TokenRef::Color(ColorToken::SurfacePanel),
            minimum: values::NON_TEXT_MIN,
            required: false,
        });
        out
    }
}
