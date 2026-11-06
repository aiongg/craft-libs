//! Style of the `button` family.

use craft_theme::{ButtonColors, ButtonVariant, Rgba, States, Tokens};

/// Fraction of the rest fill a secondary button shows while pressed.
const PRESSED_OPACITY: f32 = 0.8;

/// Colours of the four button variants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLook {
    /// Primary colours.
    pub primary: ButtonColors,
    /// Secondary colours.
    pub secondary: ButtonColors,
    /// Ghost colours.
    pub ghost: ButtonColors,
    /// Danger colours.
    pub danger: ButtonColors,
}

impl ButtonLook {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let c = &tokens.color;
        let clear = Rgba::TRANSPARENT;
        let none = States::new(clear, clear, clear, clear);
        let text = |t: Rgba| States::new(t, t, t, c.text.disabled);
        let bad = c.status.bad;
        let secondary = c.surface.sunken;
        ButtonLook {
            primary: ButtonColors {
                fill: States::new(c.accent.strong, c.accent.strong_hover, c.accent.strong_pressed, c.state.disabled_fill),
                border: none,
                text: text(c.text.on_strong),
            },
            secondary: ButtonColors {
                fill: States::new(secondary, c.state.hover, secondary.faded(PRESSED_OPACITY), c.state.disabled_fill),
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

    /// The colours of `variant`.
    pub fn colors(&self, variant: ButtonVariant) -> &ButtonColors {
        match variant {
            ButtonVariant::Primary => &self.primary,
            ButtonVariant::Secondary => &self.secondary,
            ButtonVariant::Ghost => &self.ghost,
            ButtonVariant::Danger => &self.danger,
        }
    }
}
