//! Style of the `button` family.

use craft_theme::{ButtonColors, ButtonVariant, Rgba, States, Tokens};

/// Fraction of the rest fill a primary button shows while pressed; a secondary button's pressed fill uses it too.
const PRESSED_OPACITY: f32 = 0.8;
/// Fraction of the rest fill a primary button shows under the pointer.
const HOVER_OPACITY: f32 = 0.9;

/// Colours of the four button variants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonStyle {
    /// Primary colours.
    pub primary: ButtonColors,
    /// Secondary colours.
    pub secondary: ButtonColors,
    /// Ghost colours.
    pub ghost: ButtonColors,
    /// Danger colours.
    pub danger: ButtonColors,
}

impl ButtonStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let c = &tokens.color;
        let clear = Rgba::TRANSPARENT;
        let none = States::new(clear, clear, clear, clear);
        let text = |t: Rgba| States::new(t, t, t, c.text.disabled);
        let bad = c.status.bad;
        let primary = c.accent.solid;
        ButtonStyle {
            primary: ButtonColors {
                fill: States::new(primary, primary.faded(HOVER_OPACITY), primary.faded(PRESSED_OPACITY), c.state.disabled_fill),
                border: none,
                text: text(c.text.on_accent),
            },
            secondary: ButtonColors {
                fill: States::new(c.surface.sunken, c.state.hover, c.surface.sunken.faded(PRESSED_OPACITY), c.state.disabled_fill),
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
