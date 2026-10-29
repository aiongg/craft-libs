//! Push buttons: four variants, two sizes. Loading shows a spinner in the text colour and needs
//! no tokens of its own. A theme can frame the primary button (`default_frame`); no built-in but
//! Classic does.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, Identify, OverrideValue, Semantic, State, States};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef, TypeRole};
use crate::values;

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
    /// Primary colours.
    pub primary: ButtonColors,
    /// Secondary colours.
    pub secondary: ButtonColors,
    /// Ghost colours.
    pub ghost: ButtonColors,
    /// Danger colours.
    pub danger: ButtonColors,
    /// The one-pixel frame round a primary button, outside its bevel, as the default button had in
    /// Windows 2000. Transparent draws none.
    pub default_frame: Rgba,
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

/// An overridable button field. Paths: `radius`, `default_frame`, `<variant>.<part>.<state>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonField {
    /// `button.radius`
    Radius,
    /// `button.default_frame`
    DefaultFrame,
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
    out[1] = ButtonField::DefaultFrame;
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
        // A toggle button that is on holds the secondary pressed look, so that look identifies the
        // state: its fill or its edge reaches the non-text minimum on the panel and on a popup.
        let (pressed, pressed_text) =
            Identify::on(&[c.surface.panel, c.surface.raised], s.high).edge(c.line.control_edge).fill(c.state.selected, c.text.on_selected);
        ButtonTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: BySize { sm: space.pad.control + space.gap.inline, md: values::BUTTON_PAD_X_MD * space.pad.control },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            gap: space.gap.inline,
            label: TypeRole::Label,
            radius: s.shape.radius.button,
            border_width: values::BUTTON_BORDER_WIDTH,
            primary: ButtonColors {
                fill: States::new(c.accent.strong, c.accent.strong_hover, c.accent.strong_pressed, c.state.disabled_fill),
                border: none,
                text: text(c.text.on_strong),
            },
            secondary: ButtonColors {
                fill: States::new(c.state.hover, c.state.pressed, pressed, c.state.disabled_fill),
                border: States::new(c.line.border, c.line.control_edge, c.line.control_edge, c.line.divider),
                // The pressed fill is `state.selected`, so its text is the text drawn on it.
                text: States::new(c.text.normal, c.text.normal, pressed_text, c.text.disabled),
            },
            ghost: ButtonColors { fill: States::new(clear, c.state.hover, c.state.pressed, clear), border: none, text: text(c.text.normal) },
            danger: ButtonColors {
                fill: States::new(bad.solid, bad.solid_hover, bad.solid_pressed, c.state.disabled_fill),
                border: none,
                text: text(c.text.on_status),
            },
            default_frame: clear,
        }
    }

    fn path(f: ButtonField) -> String {
        match f {
            ButtonField::Radius => "radius".to_owned(),
            ButtonField::DefaultFrame => "default_frame".to_owned(),
            ButtonField::Color { variant, part, state } => format!("{}.{}.{}", variant.name(), part.name(), state.name()),
        }
    }

    fn from_path(path: &str) -> Option<ButtonField> {
        match path {
            "radius" => return Some(ButtonField::Radius),
            "default_frame" => return Some(ButtonField::DefaultFrame),
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
            ButtonField::DefaultFrame => FieldKind::Color,
            ButtonField::Color { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: ButtonField) -> OverrideValue {
        match f {
            ButtonField::Radius => OverrideValue::Length(self.radius),
            ButtonField::DefaultFrame => OverrideValue::Color(self.default_frame),
            ButtonField::Color { variant, part, state } => OverrideValue::Color(self.colors(variant).part(part).get(state)),
        }
    }

    fn set(&mut self, f: ButtonField, v: OverrideValue) {
        match (f, v) {
            (ButtonField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ButtonField::DefaultFrame, OverrideValue::Color(c)) => self.default_frame = c,
            (ButtonField::Color { variant, part, state }, OverrideValue::Color(c)) => *self.colors_mut(variant).part_mut(part).get_mut(state) = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let field = |variant, part, state| ComponentField::Button(ButtonField::Color { variant, part, state });
        let mut out = Vec::new();
        // The secondary pressed look is a toggle button's on look. Its fill comes before its text:
        // high contrast moves the fill first.
        let secondary = |part| TokenRef::Component(field(ButtonVariant::Secondary, part, State::Pressed));
        out.extend([ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|bg| ContrastPair {
            fg: secondary(ButtonPart::Fill),
            bg: TokenRef::Color(bg),
            minimum: values::STATE_CUE_MIN,
            alternate: Some(secondary(ButtonPart::Border)),
        }));
        for variant in ButtonVariant::ALL {
            for state in [State::Rest, State::Hover, State::Pressed] {
                let fill = TokenRef::Component(field(variant, ButtonPart::Fill, state));
                out.push(ContrastPair {
                    fg: TokenRef::Component(field(variant, ButtonPart::Text, state)),
                    bg: fill,
                    minimum: values::TEXT_MIN,
                    alternate: None,
                });
            }
        }
        out
    }
}
