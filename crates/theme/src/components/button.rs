//! Push buttons: four variants, two sizes. Loading shows a spinner in the text colour and needs
//! no tokens of its own. The defaults are PhotoCraft's filled look; the Pro themes pin its outlined
//! secondary button.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States};
use crate::color::Rgba;
use crate::contrast::ensure_contrast;
use crate::derive::over;
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
    /// Outline width in logical pixels, drawn inside the button; the width of a primary button's
    /// default frame as well.
    pub border_width: f32,
    /// How much of its rest fill a primary button shows under the pointer, 0 to 1. Its hover fill
    /// is the final rest fill at this opacity unless pinned.
    pub hover_opacity: f32,
    /// How much of its rest fill a primary or secondary button shows while pressed, 0 to 1. The
    /// pressed fill is the final rest fill at this opacity unless pinned.
    pub pressed_opacity: f32,
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

    /// Derives what follows a pinned rest colour, after the theme's overrides: the primary and
    /// secondary hover and pressed fills at the opacities of the final rest fill, and the primary
    /// text of every state as the rest text. A field `pinned` says is kept. When the rest text is
    /// not pinned (`text_pinned` false), the hover and pressed text moves to the least change from
    /// the rest text that reaches the text minimum on its fill over `panel`; pinned text is kept and
    /// reported.
    pub(crate) fn follow(&mut self, pinned: impl Fn(ButtonField) -> bool, text_pinned: bool, panel: Rgba, high: bool) {
        let field = |variant, part, state| ButtonField::Color { variant, part, state };
        let (hover, pressed) = (self.hover_opacity, self.pressed_opacity);
        let set = |variant: ButtonVariant, part: ButtonPart, state: State, value: Rgba, colors: &mut ButtonColors| {
            if !pinned(field(variant, part, state)) {
                *colors.part_mut(part).get_mut(state) = value;
            }
        };
        let rest = self.primary.fill.rest;
        set(ButtonVariant::Primary, ButtonPart::Fill, State::Hover, rest.faded(hover), &mut self.primary);
        set(ButtonVariant::Primary, ButtonPart::Fill, State::Pressed, rest.faded(pressed), &mut self.primary);
        let text = self.primary.text.rest;
        let minimum = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
        for state in [State::Hover, State::Pressed] {
            let text = if text_pinned { text } else { ensure_contrast(text, over(self.primary.fill.get(state), panel), minimum) };
            set(ButtonVariant::Primary, ButtonPart::Text, state, text, &mut self.primary);
        }
        let rest = self.secondary.fill.rest;
        set(ButtonVariant::Secondary, ButtonPart::Fill, State::Pressed, rest.faded(pressed), &mut self.secondary);
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

/// An overridable button field. Paths: `radius`, `border_width`, `<variant>.<part>.<state>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonField {
    /// `button.radius`
    Radius,
    /// `button.border_width`
    BorderWidth,
    /// `button.hover_opacity`
    HoverOpacity,
    /// `button.pressed_opacity`
    PressedOpacity,
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

const FIELD_COUNT: usize = 4 + ButtonVariant::ALL.len() * ButtonPart::ALL.len() * State::ALL.len();

const FIELDS: [ButtonField; FIELD_COUNT] = {
    let mut out = [ButtonField::Radius; FIELD_COUNT];
    out[1] = ButtonField::BorderWidth;
    out[2] = ButtonField::HoverOpacity;
    out[3] = ButtonField::PressedOpacity;
    let mut i = 4;
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
        let size = s.size;
        let clear = Rgba::TRANSPARENT;
        let none = States::new(clear, clear, clear, clear);
        let text = |t: Rgba| States::new(t, t, t, c.text.disabled);
        let bad = c.status.bad;
        let (sm, md) = (s.set.button_height_sm, s.set.button_height_md);
        let pad_x = s.set.button_pad_x;
        let primary = c.accent.solid;
        ButtonTokens {
            height: BySize { sm, md },
            pad_x: BySize { sm: pad_x, md: pad_x },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            gap: s.space.gap.inline,
            label: TypeRole::Label,
            radius: s.shape.radius.button,
            border_width: values::BUTTON_BORDER_WIDTH,
            hover_opacity: values::BUTTON_HOVER_OPACITY,
            pressed_opacity: values::BUTTON_PRESSED_OPACITY,
            primary: ButtonColors {
                fill: States::new(
                    primary,
                    primary.faded(values::BUTTON_HOVER_OPACITY),
                    primary.faded(values::BUTTON_PRESSED_OPACITY),
                    c.state.disabled_fill,
                ),
                border: none,
                text: text(c.text.on_accent),
            },
            secondary: ButtonColors {
                fill: States::new(c.surface.sunken, c.state.hover, c.surface.sunken.faded(values::BUTTON_PRESSED_OPACITY), c.state.disabled_fill),
                border: States::new(c.line.control_edge, c.line.control_edge, c.line.control_edge, c.line.divider),
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
            ButtonField::BorderWidth => "border_width".to_owned(),
            ButtonField::HoverOpacity => "hover_opacity".to_owned(),
            ButtonField::PressedOpacity => "pressed_opacity".to_owned(),
            ButtonField::Color { variant, part, state } => format!("{}.{}.{}", variant.name(), part.name(), state.name()),
        }
    }

    fn from_path(path: &str) -> Option<ButtonField> {
        match path {
            "radius" => return Some(ButtonField::Radius),
            "border_width" => return Some(ButtonField::BorderWidth),
            "hover_opacity" => return Some(ButtonField::HoverOpacity),
            "pressed_opacity" => return Some(ButtonField::PressedOpacity),
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
            ButtonField::Radius | ButtonField::BorderWidth | ButtonField::HoverOpacity | ButtonField::PressedOpacity => FieldKind::Length,
            ButtonField::Color { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: ButtonField) -> OverrideValue {
        match f {
            ButtonField::Radius => OverrideValue::Length(self.radius),
            ButtonField::BorderWidth => OverrideValue::Length(self.border_width),
            ButtonField::HoverOpacity => OverrideValue::Length(self.hover_opacity),
            ButtonField::PressedOpacity => OverrideValue::Length(self.pressed_opacity),
            ButtonField::Color { variant, part, state } => OverrideValue::Color(self.colors(variant).part(part).get(state)),
        }
    }

    fn set(&mut self, f: ButtonField, v: OverrideValue) {
        match (f, v) {
            (ButtonField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ButtonField::BorderWidth, OverrideValue::Length(w)) => self.border_width = w,
            (ButtonField::HoverOpacity, OverrideValue::Length(o)) => self.hover_opacity = o,
            (ButtonField::PressedOpacity, OverrideValue::Length(o)) => self.pressed_opacity = o,
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
                out.push(ContrastPair {
                    fg: TokenRef::Component(field(variant, ButtonPart::Text, state)),
                    bg: fill,
                    minimum: values::TEXT_MIN,
                    alternate: None,
                });
            }
        }
        // A secondary button is identified by its outline or its fill on what it sits on.
        let fill = |state| TokenRef::Component(field(ButtonVariant::Secondary, ButtonPart::Fill, state));
        for state in [State::Rest, State::Hover, State::Pressed] {
            for surface in [ColorToken::SurfacePanel, ColorToken::SurfaceRaised] {
                out.push(ContrastPair {
                    fg: TokenRef::Component(field(ButtonVariant::Secondary, ButtonPart::Border, state)),
                    bg: TokenRef::Color(surface),
                    minimum: values::CONTROL_EDGE_MIN,
                    alternate: Some(fill(state)),
                });
            }
        }
        out
    }
}
